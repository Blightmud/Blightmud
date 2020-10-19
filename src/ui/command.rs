use crate::event::Event;
use crate::model::{Connection, Line};
use crate::{lua::LuaScript, lua::UiEvent, session::Session};
use log::debug;
use rs_complete::CompletionTree;
use std::collections::VecDeque;
use std::thread;
use std::{
    io::stdin,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};
use termion::{event::Key, input::TermRead};

#[derive(Default)]
struct CompletionStepData {
    options: Vec<String>,
    index: usize,
    base: String,
}

impl CompletionStepData {
    fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    fn set_options(&mut self, base: &str, options: Vec<String>) {
        self.options = options;
        self.base = base.to_string();
    }

    fn clear(&mut self) {
        self.options.clear();
        self.index = 0;
    }

    fn next(&mut self) -> Option<&String> {
        if !self.is_empty() {
            let last_index = self.index;
            self.index = (self.index + 1) % (self.options.len() + 1);
            self.options.get(last_index).or(Some(&self.base))
        } else {
            None
        }
    }
}

pub struct CommandBuffer {
    buffer: String,
    cached_buffer: String,
    history: VecDeque<String>,
    current_index: usize,
    cursor_pos: usize,
    completion_tree: CompletionTree,
    completion: CompletionStepData,
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self {
            buffer: String::default(),
            cached_buffer: String::default(),
            history: VecDeque::default(),
            current_index: 0,
            cursor_pos: 0,
            completion_tree: CompletionTree::with_inclusions(&['/', '_']),
            completion: CompletionStepData::default(),
        }
    }
}

impl CommandBuffer {
    fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    fn get_pos(&self) -> usize {
        self.cursor_pos
    }

    fn submit(&mut self) -> String {
        self.completion_tree.insert(&self.buffer.to_lowercase());

        // Insert history
        let cmd = if !self.buffer.is_empty() {
            if let Some(last_cmd) = self.history.iter().last() {
                if &self.buffer != last_cmd {
                    self.history.push_back(self.buffer.clone());
                }
            } else {
                self.history.push_back(self.buffer.clone());
            }

            while self.history.len() > 100 {
                self.history.pop_front();
            }

            self.buffer.clone()
        } else {
            String::new()
        };

        self.current_index = self.history.len();
        self.buffer.clear();
        self.cursor_pos = 0;
        cmd
    }

    fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor_pos < self.buffer.len() {
            self.cursor_pos += 1;
        }
    }

    fn move_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    fn move_to_end(&mut self) {
        self.cursor_pos = self.buffer.len();
    }

    fn move_word_right(&mut self) {
        let origin = (self.cursor_pos + 1).min(self.buffer.len());
        self.cursor_pos = if let Some(pos) = self.buffer[origin..].find(' ') {
            origin + pos
        } else {
            self.buffer.len()
        }
    }

    fn move_word_left(&mut self) {
        let origin = self.cursor_pos.max(1) - 1;
        self.cursor_pos = if let Some(pos) = self.buffer[0..origin].rfind(' ') {
            pos + 1
        } else {
            0
        }
    }

    fn delete_to_end(&mut self) {
        self.buffer = self.buffer[..self.cursor_pos].to_string();
    }

    fn delete_from_start(&mut self) {
        self.buffer = self.buffer[self.cursor_pos..].to_string();
        self.cursor_pos = 0;
    }

    fn delete_right(&mut self) {
        if self.cursor_pos < self.buffer.len() {
            self.buffer.remove(self.cursor_pos);
        }
    }

    fn delete_word_right(&mut self) {
        let origin = self.cursor_pos;
        self.move_word_right();
        if origin != self.cursor_pos {
            self.buffer.replace_range(origin..self.cursor_pos, "");
            self.cursor_pos = origin;
        }
    }

    fn delete_word_left(&mut self) {
        let origin = self.cursor_pos;
        self.move_word_left();
        if origin != self.cursor_pos {
            self.buffer.replace_range(self.cursor_pos..origin, "");
        }
    }

    fn remove(&mut self) {
        if self.cursor_pos > 0 {
            if self.cursor_pos < self.buffer.len() {
                self.buffer.remove(self.cursor_pos - 1);
            } else {
                self.buffer.pop();
            }
            self.move_left();
        }
    }

    fn push_key(&mut self, c: char) {
        if self.cursor_pos >= self.buffer.len() {
            self.buffer.push(c);
        } else {
            self.buffer.insert(self.cursor_pos, c);
        }
        self.completion.clear();
        self.move_right();
    }

    fn tab_complete(&mut self) {
        if self.buffer.len() > 1 {
            if self.completion.is_empty() {
                if let Some(options) = self.completion_tree.complete(&self.buffer) {
                    self.completion.set_options(&self.buffer, options);
                }
            }
            if let Some(comp) = self.completion.next() {
                self.buffer = comp.clone();
                self.cursor_pos = comp.len();
            }
        }
    }

    fn previous(&mut self) {
        if !self.history.is_empty() {
            if self.current_index == self.history.len() {
                self.cached_buffer = self.buffer.clone()
            }

            self.current_index = {
                if self.current_index > 0 {
                    self.current_index - 1
                } else {
                    self.current_index
                }
            };
            self.buffer = self.history[self.current_index].clone();
            self.cursor_pos = self.buffer.len();
        }
    }

    fn next(&mut self) {
        let new_index = {
            if self.current_index < self.history.len() {
                self.current_index + 1
            } else {
                self.current_index
            }
        };

        if new_index != self.current_index {
            self.current_index = new_index;
            if self.current_index == self.history.len() {
                self.buffer = self.cached_buffer.clone();
                self.cached_buffer.clear();
            } else {
                self.buffer = self.history[self.current_index].clone();
            }
        }
        self.cursor_pos = self.buffer.len();
    }
}

fn parse_key_event(
    key: termion::event::Key,
    buffer: &mut CommandBuffer,
    writer: &Sender<Event>,
    terminate: &Arc<AtomicBool>,
) {
    match key {
        Key::Char('\n') => writer.send(parse_command(&buffer.submit())).unwrap(),
        Key::Char('\t') => buffer.tab_complete(),
        Key::Char(c) => buffer.push_key(c),
        Key::Ctrl('l') => writer.send(Event::Redraw).unwrap(),
        Key::Ctrl('c') => {
            debug!("Caught ctrl-c, terminating");
            terminate.store(true, Ordering::Relaxed);
            writer.send(Event::Quit).unwrap();
        }
        Key::PageUp => writer.send(Event::ScrollUp).unwrap(),
        Key::PageDown => writer.send(Event::ScrollDown).unwrap(),
        Key::End => writer.send(Event::ScrollBottom).unwrap(),

        // Input navigation
        Key::Left => buffer.move_left(),
        Key::Right => buffer.move_right(),
        Key::Backspace => buffer.remove(),
        Key::Up => buffer.previous(),
        Key::Down => buffer.next(),
        _ => {}
    };
}

fn check_command_binds(
    cmd: termion::event::Key,
    buffer: &mut CommandBuffer,
    script: &Arc<Mutex<LuaScript>>,
    writer: &Sender<Event>,
) {
    if let Ok(mut script) = script.lock() {
        match cmd {
            Key::Ctrl(c) => {
                script.check_bindings(&format!("ctrl-{}", c));
            }
            Key::Alt(c) => {
                script.check_bindings(&format!("alt-{}", c));
            }
            Key::F(n) => {
                script.check_bindings(&format!("f{}", n));
            }
            _ => {}
        }
    }
    handle_script_ui_io(buffer, script, writer);
}

fn check_escape_bindings(
    escape: &str,
    buffer: &mut CommandBuffer,
    script: &Arc<Mutex<LuaScript>>,
    writer: &Sender<Event>,
) {
    if let Ok(mut script) = script.lock() {
        if !script.check_bindings(&escape.to_lowercase()) {
            writer
                .send(Event::Info(format!("Unknown command: {:?}", escape)))
                .unwrap();
        }
    }
    handle_script_ui_io(buffer, script, writer);
    writer
        .send(Event::UserInputBuffer(
            buffer.get_buffer(),
            buffer.get_pos(),
        ))
        .unwrap();
}

fn handle_script_ui_io(
    buffer: &mut CommandBuffer,
    script: &Arc<Mutex<LuaScript>>,
    writer: &Sender<Event>,
) {
    if let Ok(mut script) = script.lock() {
        script.get_ui_events().iter().for_each(|event| match event {
            UiEvent::StepLeft => buffer.move_left(),
            UiEvent::StepRight => buffer.move_right(),
            UiEvent::StepToStart => buffer.move_to_start(),
            UiEvent::StepToEnd => buffer.move_to_end(),
            UiEvent::StepWordLeft => buffer.move_word_left(),
            UiEvent::StepWordRight => buffer.move_word_right(),
            UiEvent::Remove => buffer.remove(),
            UiEvent::DeleteToEnd => buffer.delete_to_end(),
            UiEvent::DeleteFromStart => buffer.delete_from_start(),
            UiEvent::DeleteWordLeft => buffer.delete_word_left(),
            UiEvent::DeleteWordRight => buffer.delete_word_right(),
            UiEvent::DeleteRight => buffer.delete_right(),
            UiEvent::PreviousCommand => buffer.previous(),
            UiEvent::NextCommand => buffer.next(),
            UiEvent::ScrollDown => writer.send(Event::ScrollDown).unwrap(),
            UiEvent::ScrollUp => writer.send(Event::ScrollUp).unwrap(),
            UiEvent::ScrollBottom => writer.send(Event::ScrollBottom).unwrap(),
            UiEvent::Complete => buffer.tab_complete(),
            UiEvent::Unknown(_) => {}
        });
        script.get_output_lines().iter().for_each(|l| {
            writer.send(Event::Output(Line::from(l))).unwrap();
        });
    }
}

pub fn spawn_input_thread(session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        debug!("Input stream spawned");
        let writer = session.main_writer;
        let terminate = session.terminate;
        let script = session.lua_script;
        let stdin = stdin();
        let mut buffer = CommandBuffer::default();
        buffer
            .completion_tree
            .insert(include_str!("../../resources/completions.txt"));

        for e in stdin.events() {
            match e.unwrap() {
                termion::event::Event::Key(key) => {
                    parse_key_event(key, &mut buffer, &writer, &terminate);
                    check_command_binds(key, &mut buffer, &script, &writer);
                    writer
                        .send(Event::UserInputBuffer(
                            buffer.get_buffer(),
                            buffer.get_pos(),
                        ))
                        .unwrap();
                }
                termion::event::Event::Unsupported(bytes) => {
                    if let Ok(escape) = String::from_utf8(bytes.clone()) {
                        check_escape_bindings(
                            &escape.to_lowercase(),
                            &mut buffer,
                            &script,
                            &writer,
                        );
                    } else {
                        writer
                            .send(Event::Info(format!("Unknown command: {:?}", bytes)))
                            .unwrap();
                    }
                }
                _ => {}
            }
            if terminate.load(Ordering::Relaxed) {
                break;
            }
        }
        debug!("Input stream closing");
    })
}

fn parse_command(msg: &str) -> Event {
    let msg = String::from(msg);
    let lc_msg = msg.to_ascii_lowercase();
    let mut iter = lc_msg.split_whitespace();
    match iter.next() {
        Some("/connect") => {
            let p1 = iter.next();
            let p2 = iter.next();
            let p3 = iter.next();

            if p1 == None && p2 == None {
                Event::Info("USAGE: /connect <host> <port>".to_string())
            } else if p2 == None {
                let name = p1.unwrap().to_string();

                Event::LoadServer(name)
            } else {
                let host = p1.unwrap().to_string();
                let tls = if let Some(tls) = p3 {
                    tls == "tls"
                } else {
                    false
                };
                if let Ok(port) = p2.unwrap().parse::<u16>() {
                    Event::Connect(Connection::new(&host, port, tls))
                } else {
                    Event::Error(
                        "USAGE: /connect <host: String> <port: Positive number>".to_string(),
                    )
                }
            }
        }
        Some("/disconnect") | Some("/dc") => Event::Disconnect(0),
        Some("/reconnect") | Some("/rc") => Event::Reconnect,
        Some("/add_server") => {
            let p1 = iter.next();
            let p2 = iter.next();
            let p3 = iter.next();
            let p4 = iter.next();

            if p1 == None || p2 == None || p3 == None {
                Event::Info(
                    "USAGE: /add_server <name: String> <host: String> <port: Positive number>"
                        .to_string(),
                )
            } else {
                let name = p1.unwrap().to_string();
                let host = p2.unwrap().to_string();

                let tls = if let Some(tls) = p4 {
                    tls == "tls"
                } else {
                    false
                };

                if let Ok(port) = p3.unwrap().parse::<u16>() {
                    Event::AddServer(name, Connection::new(&host, port, tls))
                } else {
                    Event::Error(
                        "USAGE: /add_server <name: String> <host: String> <port: Positive number>"
                            .to_string(),
                    )
                }
            }
        }
        Some("/remove_server") => {
            let p1 = iter.next();

            if let Some(name) = p1 {
                Event::RemoveServer(name.to_string())
            } else {
                Event::Info("USAGE: /remove_server <name: String>".to_string())
            }
        }
        Some("/list_servers") | Some("/ls") => Event::ListServers,
        Some("/load") => {
            let p1 = iter.next();
            if p1 == None {
                Event::Info("USAGE: /load <path>".to_string())
            } else {
                let p1 = p1.unwrap().to_string();
                Event::LoadScript(p1)
            }
        }
        Some("/help") => {
            let p1 = iter.next();
            if let Some(hfile) = p1 {
                Event::ShowHelp(hfile.to_string())
            } else {
                Event::ShowHelp("help".to_string())
            }
        }
        Some("/start_log") => {
            let p1 = iter.next();
            if let Some(world) = p1 {
                Event::StartLogging(world.to_string(), true)
            } else {
                Event::Info("USAGE: /start_log <name>".to_string())
            }
        }
        Some("/stop_log") => Event::StopLogging,
        Some("/set") => {
            let p1 = iter.next();
            let p2 = iter.next();

            if p1 == None && p2 == None {
                Event::Info("USAGE: /set <setting> or /set <setting> <new_value>".to_string())
            } else if p2.is_some() && p1.is_some() {
                Event::ToggleSetting(p1.unwrap().to_string(), p2.unwrap().to_string())
            } else {
                Event::ShowSetting(p1.unwrap().to_string())
            }
        }
        Some("/quit") | Some("/q") => Event::Quit,
        _ => Event::ServerInput(Line::from(msg)),
    }
}

#[cfg(test)]
mod command_test {

    use super::CommandBuffer;

    fn push_string(buffer: &mut CommandBuffer, msg: &str) {
        msg.chars().for_each(|c| buffer.push_key(c));
    }

    #[test]
    fn test_editing() {
        let mut buffer = CommandBuffer::default();

        push_string(&mut buffer, "test is test");
        assert_eq!(buffer.get_buffer(), "test is test");
        assert_eq!(buffer.get_pos(), 12);
        buffer.move_left();
        buffer.move_left();
        buffer.move_left();
        buffer.move_left();
        buffer.remove();
        buffer.remove();
        buffer.remove();
        buffer.remove();
        assert_eq!(buffer.get_buffer(), "testtest");
        assert_eq!(buffer.get_pos(), 4);
        push_string(&mut buffer, " confirm ");
        assert_eq!(buffer.get_buffer(), "test confirm test");
        assert_eq!(buffer.get_pos(), 13);
    }

    #[test]
    fn test_no_zero_index_remove_crash() {
        let mut buffer = CommandBuffer::default();
        buffer.push_key('t');
        buffer.move_left();
        assert_eq!(buffer.get_pos(), 0);
        buffer.remove();
        assert_eq!(buffer.get_pos(), 0);
    }

    #[test]
    fn test_no_history_empty_input() {
        let mut buffer = CommandBuffer::default();
        buffer.submit();
        assert!(buffer.history.is_empty());
    }

    #[test]
    fn no_duplicate_commands_in_history() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "test");
        buffer.submit();
        push_string(&mut buffer, "test");
        buffer.submit();
        push_string(&mut buffer, "test");
        buffer.submit();
        push_string(&mut buffer, "test");
        buffer.submit();
        push_string(&mut buffer, "random");
        buffer.submit();
        push_string(&mut buffer, "random");
        buffer.submit();
        push_string(&mut buffer, "random");
        buffer.submit();
        push_string(&mut buffer, "test");
        buffer.submit();
        push_string(&mut buffer, "random");
        buffer.submit();

        assert_eq!(buffer.history.len(), 4);
        let mut it = buffer.history.iter();
        assert_eq!(it.next(), Some(&"test".to_string()));
        assert_eq!(it.next(), Some(&"random".to_string()));
        assert_eq!(it.next(), Some(&"test".to_string()));
        assert_eq!(it.next(), Some(&"random".to_string()));
    }

    #[test]
    fn test_input_navigation() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_word_left();
        assert_eq!(buffer.cursor_pos, 12);
        buffer.move_word_left();
        assert_eq!(buffer.cursor_pos, 5);
        buffer.move_word_left();
        assert_eq!(buffer.cursor_pos, 0);
        buffer.move_word_left();
        assert_eq!(buffer.cursor_pos, 0);
        buffer.move_word_right();
        assert_eq!(buffer.cursor_pos, 4);
        buffer.move_word_right();
        assert_eq!(buffer.cursor_pos, 11);
        buffer.move_word_right();
        assert_eq!(buffer.cursor_pos, 17);
        buffer.move_word_right();
        assert_eq!(buffer.cursor_pos, 17);
    }

    #[test]
    fn test_end_start_navigation() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        assert_eq!(buffer.cursor_pos, 0);
        buffer.move_to_start();
        assert_eq!(buffer.cursor_pos, 0);
        buffer.move_to_end();
        assert_eq!(buffer.cursor_pos, 17);
        buffer.move_to_end();
        assert_eq!(buffer.cursor_pos, 17);
    }

    #[test]
    fn test_delete_rest_of_line() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.move_word_right();
        buffer.delete_from_start();
        assert_eq!(buffer.get_buffer(), " random words");
    }

    #[test]
    fn test_delete_from_start_of_line() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.move_word_right();
        buffer.move_word_right();
        buffer.delete_to_end();
        assert_eq!(buffer.get_buffer(), "some random");
    }

    #[test]
    fn test_delete_right() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.move_word_right();
        buffer.delete_right();
        assert_eq!(buffer.get_buffer(), "somerandom words");
        buffer.delete_right();
        assert_eq!(buffer.get_buffer(), "someandom words");
        buffer.move_to_end();
        buffer.delete_right();
        assert_eq!(buffer.get_buffer(), "someandom words");
    }

    #[test]
    fn test_delete_word_left() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_to_end();
        buffer.delete_word_left();
        assert_eq!(buffer.get_buffer(), "some random ");
        buffer.move_to_start();
        buffer.move_word_right();
        buffer.delete_word_left();
        assert_eq!(buffer.get_buffer(), " random ");
    }

    #[test]
    fn test_delete_word_right() {
        let mut buffer = CommandBuffer::default();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.delete_word_right();
        assert_eq!(buffer.get_buffer(), " random words");
        buffer.delete_word_right();
        assert_eq!(buffer.get_buffer(), " words");
    }
}
