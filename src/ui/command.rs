use crate::event::Event;
use crate::model::Connection;
use crate::session::Session;
use log::debug;
use rs_completion::CompletionTree;
use std::collections::VecDeque;
use std::thread;
use std::{io::stdin, sync::atomic::Ordering};
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

pub fn spawn_input_thread(session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        debug!("Input stream spawned");
        let writer = session.main_writer;
        let terminate = session.terminate;
        let stdin = stdin();
        let mut buffer = CommandBuffer::default();
        buffer
            .completion_tree
            .insert(include_str!("../../resources/completions.txt"));

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('\n') => {
                    writer.send(parse_command(&buffer.submit())).unwrap();
                }
                Key::Char('\t') => buffer.tab_complete(),
                Key::Char(c) => buffer.push_key(c),
                Key::Ctrl('c') => {
                    debug!("Caught ctrl-c, terminating");
                    terminate.store(true, Ordering::Relaxed);
                    writer.send(Event::Quit).unwrap();
                    break;
                }
                Key::PageUp => {
                    writer.send(Event::ScrollUp).unwrap();
                }
                Key::PageDown => {
                    writer.send(Event::ScrollDown).unwrap();
                }
                Key::End => {
                    writer.send(Event::ScrollBottom).unwrap();
                }
                Key::Up | Key::Ctrl('p') => buffer.previous(),
                Key::Down | Key::Ctrl('n') => buffer.next(),
                Key::Ctrl('l') => writer.send(Event::Redraw).unwrap(),
                Key::Left => buffer.move_left(),
                Key::Right => buffer.move_right(),
                Key::Backspace => {
                    buffer.remove();
                }
                _ => {}
            };
            writer
                .send(Event::UserInputBuffer(
                    buffer.get_buffer(),
                    buffer.get_pos(),
                ))
                .unwrap();
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

            if p1 == None && p2 == None {
                Event::Info("USAGE: /connect <host> <port>".to_string())
            } else if p2 == None {
                let name = p1.unwrap().to_string();

                Event::LoadServer(name)
            } else {
                let host = p1.unwrap().to_string();
                if let Ok(port) = p2.unwrap().parse::<u16>() {
                    Event::Connect(Connection { host, port })
                } else {
                    Event::Error(
                        "USAGE: /connect <host: String> <port: Positive number>".to_string(),
                    )
                }
            }
        }
        Some("/add_server") => {
            let p1 = iter.next();
            let p2 = iter.next();
            let p3 = iter.next();

            if p1 == None || p2 == None || p3 == None {
                Event::Info(
                    "USAGE: /add_server <name: String> <host: String> <port: Positive number>"
                        .to_string(),
                )
            } else {
                let name = p1.unwrap().to_string();
                let host = p2.unwrap().to_string();

                if let Ok(port) = p3.unwrap().parse::<u16>() {
                    Event::AddServer(name, Connection { host, port })
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
        Some("/disconnect") | Some("/dc") => Event::Disconnect,
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
        _ => Event::ServerInput(msg, true),
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
}
