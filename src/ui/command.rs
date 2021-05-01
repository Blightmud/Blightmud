use crate::model::{Line, Servers};
use crate::{event::Event, tts::TTSController};
use crate::{lua::LuaScript, lua::UiEvent, session::Session, SaveData};
use log::debug;
use rs_complete::CompletionTree;
use std::collections::VecDeque;
use std::thread;
use std::{
    io::stdin,
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex},
};
use termion::{event::Key, input::TermRead};

const MAX_HISTORY: usize = 100;

pub type History = VecDeque<String>;
impl SaveData for History {
    fn relative_path() -> PathBuf {
        PathBuf::from("data/history.ron")
    }
}

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
    strbuf: String,
    buffer: Vec<char>,
    cached_buffer: Vec<char>,
    history: History,
    current_index: usize,
    cursor_pos: usize,
    completion_tree: CompletionTree,
    completion: CompletionStepData,
    tts_ctrl: Arc<Mutex<TTSController>>,
}

impl CommandBuffer {
    pub fn new(tts_ctrl: Arc<Mutex<TTSController>>) -> Self {
        let mut completion = CompletionTree::with_inclusions(&['/', '_']);
        completion.set_min_word_len(3);

        Self {
            strbuf: String::new(),
            buffer: vec![],
            cached_buffer: vec![],
            current_index: 0,
            history: History::default(),
            cursor_pos: 0,
            completion_tree: completion,
            completion: CompletionStepData::default(),
            tts_ctrl,
        }
    }

    fn get_buffer(&mut self) -> String {
        self.strbuf = self.buffer.iter().collect();
        self.strbuf.clone()
    }

    fn get_pos(&self) -> usize {
        self.cursor_pos
    }

    fn submit(&mut self) -> String {
        // Insert history
        let cmd = if !self.buffer.is_empty() {
            let command = self.get_buffer();
            self.completion_tree.insert(&command);

            if let Some(last_cmd) = self.history.iter().last() {
                if &command != last_cmd {
                    self.history.push_back(command.clone());
                }
            } else {
                self.history.push_back(command.clone());
            }

            while self.history.len() > MAX_HISTORY {
                self.history.pop_front();
            }

            command
        } else {
            String::new()
        };

        self.current_index = self.history.len();
        self.buffer.clear();
        self.cursor_pos = 0;

        cmd
    }

    fn step_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn step_right(&mut self) {
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

    fn step_word_right(&mut self) {
        let origin = (self.cursor_pos + 1).min(self.buffer.len());
        self.cursor_pos = if let Some(pos) = self.buffer[origin..].iter().position(|c| *c == ' ') {
            origin + pos
        } else {
            self.buffer.len()
        }
    }

    fn step_word_left(&mut self) {
        let origin = self.cursor_pos.max(1) - 1;
        self.cursor_pos = if let Some(pos) = self.buffer[0..origin].iter().rposition(|c| *c == ' ')
        {
            pos + 1
        } else {
            0
        }
    }

    fn delete_to_end(&mut self) {
        self.buffer.drain(self.cursor_pos..self.buffer.len());
    }

    fn delete_from_start(&mut self) {
        self.buffer.drain(0..self.cursor_pos);
        self.cursor_pos = 0;
    }

    fn delete_right(&mut self) {
        if self.cursor_pos < self.buffer.len() {
            self.buffer.remove(self.cursor_pos);
        }
    }

    fn delete_word_right(&mut self) {
        let origin = self.cursor_pos;
        self.step_word_right();
        if origin != self.cursor_pos {
            self.buffer.drain(origin..self.cursor_pos);
            self.cursor_pos = origin;
        }
    }

    fn delete_word_left(&mut self) {
        let origin = self.cursor_pos;
        self.step_word_left();
        if origin != self.cursor_pos {
            self.buffer.drain(self.cursor_pos..origin);
        }
    }

    fn remove(&mut self) {
        if self.cursor_pos > 0 {
            if self.cursor_pos < self.buffer.len() {
                self.buffer.remove(self.cursor_pos - 1);
            } else {
                self.buffer.pop();
            }
            self.step_left();
        }
    }

    fn push_key(&mut self, c: char) {
        if self.cursor_pos >= self.buffer.len() {
            self.buffer.push(c);
        } else {
            self.buffer.insert(self.cursor_pos, c);
        }
        self.completion.clear();
        self.step_right();
    }

    fn tab_complete(&mut self) {
        if self.buffer.len() > 1 {
            if self.completion.is_empty() {
                if let Some(options) = self.completion_tree.complete(&self.strbuf) {
                    self.completion.set_options(&self.strbuf, options);
                }
            }
            if let Some(comp) = self.completion.next() {
                self.tts_ctrl.lock().unwrap().speak(&comp, true);
                self.buffer = comp.chars().collect();
                self.cursor_pos = comp.len();
            }
        }
    }

    fn previous(&mut self) {
        if !self.history.is_empty() {
            if self.current_index == self.history.len() {
                self.cached_buffer = self.buffer.clone();
            }

            self.current_index = {
                if self.current_index > 0 {
                    self.current_index - 1
                } else {
                    self.current_index
                }
            };
            self.buffer = self.history[self.current_index].chars().collect();
            self.cursor_pos = self.buffer.len();
            self.tts_ctrl.lock().unwrap().speak(&self.strbuf, true);
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
                self.buffer = self.history[self.current_index].chars().collect();
            }
        }
        self.tts_ctrl.lock().unwrap().speak(&self.strbuf, true);
        self.cursor_pos = self.buffer.len();
    }
}

fn parse_mouse_event(event: termion::event::MouseEvent, writer: &Sender<Event>) {
    use termion::event::{MouseButton, MouseEvent};
    match event {
        MouseEvent::Press(MouseButton::WheelUp, ..) => writer.send(Event::ScrollUp).unwrap(),
        MouseEvent::Press(MouseButton::WheelDown, ..) => writer.send(Event::ScrollDown).unwrap(),
        _ => {}
    }
}

fn parse_key_event(
    key: termion::event::Key,
    buffer: &mut CommandBuffer,
    writer: &Sender<Event>,
    tts_ctrl: &mut Arc<Mutex<TTSController>>,
    save_history: bool,
) {
    match key {
        Key::Char('\n') => {
            let input = Line::from(buffer.get_buffer());
            match parse_command(&buffer.submit()) {
                Event::ServerInput(msg) => writer.send(Event::ServerInput(msg)).unwrap(),
                Event::Quit => {
                    if save_history {
                        buffer.history.save();
                    }
                    writer.send(Event::Quit).unwrap();
                }
                e => {
                    writer.send(Event::InputSent(input)).unwrap();
                    writer.send(e).unwrap();
                }
            }
        }
        Key::Char('\t') => buffer.tab_complete(),
        Key::Char(c) => {
            tts_ctrl.lock().unwrap().key_press(c);
            buffer.push_key(c);
        }
        Key::Ctrl('l') => writer.send(Event::Redraw).unwrap(),
        Key::Ctrl('c') => {
            if save_history {
                buffer.history.save();
            }
            writer.send(Event::Quit).unwrap();
        }
        Key::PageUp => writer.send(Event::ScrollUp).unwrap(),
        Key::PageDown => writer.send(Event::ScrollDown).unwrap(),
        Key::Home => writer.send(Event::ScrollTop).unwrap(),
        Key::End => writer.send(Event::ScrollBottom).unwrap(),

        // Input navigation
        Key::Left => buffer.step_left(),
        Key::Right => buffer.step_right(),
        Key::Backspace => buffer.remove(),
        Key::Delete => buffer.delete_right(),
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
) -> bool {
    let mut ran = false;
    if let Ok(mut script) = script.lock() {
        ran = match cmd {
            Key::Ctrl(c) => script.check_bindings(&human_key("ctrl-", c)),
            Key::Alt(c) => script.check_bindings(&human_key("alt-", c)),
            Key::F(n) => script.check_bindings(&format!("f{}", n)),
            Key::PageUp => script.check_bindings("pageup") || script.check_bindings("page up"),
            Key::PageDown => {
                script.check_bindings("pagedown") || script.check_bindings("page down")
            }
            Key::Home => script.check_bindings("home"),
            Key::End => script.check_bindings("end"),
            _ => false,
        }
    }
    handle_script_ui_io(buffer, script, writer);
    ran
}

/// Convert a key combination to a human-readable form.
fn human_key(prefix: &str, c: char) -> String {
    let mut out = prefix.to_owned();
    match c {
        '\u{7f}' => out.push_str("backspace"),
        '\u{1b}' => out.push_str("escape"),
        _ => out.push(c),
    }
    out
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
            UiEvent::StepLeft => buffer.step_left(),
            UiEvent::StepRight => buffer.step_right(),
            UiEvent::StepToStart => buffer.move_to_start(),
            UiEvent::StepToEnd => buffer.move_to_end(),
            UiEvent::StepWordLeft => buffer.step_word_left(),
            UiEvent::StepWordRight => buffer.step_word_right(),
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
            UiEvent::ScrollTop => writer.send(Event::ScrollTop).unwrap(),
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
    thread::Builder::new()
        .name("input-thread".to_string())
        .spawn(move || {
            debug!("Input stream spawned");
            let writer = session.main_writer.clone();
            let script = session.lua_script.clone();
            let stdin = stdin();
            let mut tts_ctrl = session.tts_ctrl.clone();
            let mut buffer = CommandBuffer::new(tts_ctrl.clone());
            for server in Servers::load().keys() {
                buffer.completion_tree.insert(&server);
            }
            buffer
                .completion_tree
                .insert(include_str!("../../resources/completions.txt"));

            let save_history = session.save_history();
            if save_history {
                buffer.history = History::load();
                buffer.current_index = buffer.history.len();
                for line in buffer.history.iter() {
                    buffer.completion_tree.insert(&line);
                }
            }

            for e in stdin.events() {
                match e.unwrap() {
                    termion::event::Event::Key(key) => {
                        if !check_command_binds(key, &mut buffer, &script, &writer) {
                            parse_key_event(key, &mut buffer, &writer, &mut tts_ctrl, save_history);
                        }
                        writer
                            .send(Event::UserInputBuffer(
                                buffer.get_buffer(),
                                buffer.get_pos(),
                            ))
                            .unwrap();
                    }
                    termion::event::Event::Mouse(event) => parse_mouse_event(event, &writer),
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
                }
            }
            debug!("Input stream closing");
        })
        .unwrap()
}

fn parse_command(msg: &str) -> Event {
    let msg = String::from(msg);
    let lc_msg = msg.to_ascii_lowercase();
    let mut iter = lc_msg.split_whitespace();
    match iter.next() {
        Some("/help") => {
            let p1 = iter.next();
            if let Some(hfile) = p1 {
                Event::ShowHelp(hfile.to_string(), true)
            } else {
                Event::ShowHelp("help".to_string(), true)
            }
        }
        Some("/quit") | Some("/q") => Event::Quit,
        _ => Event::ServerInput(Line::from(msg)),
    }
}

#[cfg(test)]
mod command_test {

    use std::sync::{Arc, Mutex};

    use super::CommandBuffer;
    use crate::tts::TTSController;

    fn push_string(buffer: &mut CommandBuffer, msg: &str) {
        msg.chars().for_each(|c| buffer.push_key(c));
    }

    fn get_command() -> CommandBuffer {
        CommandBuffer::new(Arc::new(Mutex::new(TTSController::new(false))))
    }

    #[test]
    fn test_editing() {
        let mut buffer = get_command();

        push_string(&mut buffer, "test is test");
        assert_eq!(buffer.get_buffer(), "test is test");
        assert_eq!(buffer.get_pos(), 12);
        buffer.step_left();
        buffer.step_left();
        buffer.step_left();
        buffer.step_left();
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
        let mut buffer = get_command();
        buffer.push_key('t');
        buffer.step_left();
        assert_eq!(buffer.get_pos(), 0);
        buffer.remove();
        assert_eq!(buffer.get_pos(), 0);
    }

    #[test]
    fn test_no_history_empty_input() {
        let mut buffer = get_command();
        buffer.submit();
        assert!(buffer.history.is_empty());
    }

    #[test]
    fn no_duplicate_commands_in_history() {
        let mut buffer = get_command();
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
        let mut buffer = get_command();
        push_string(&mut buffer, "some random words");
        buffer.step_word_left();
        assert_eq!(buffer.cursor_pos, 12);
        buffer.step_word_left();
        assert_eq!(buffer.cursor_pos, 5);
        buffer.step_word_left();
        assert_eq!(buffer.cursor_pos, 0);
        buffer.step_word_left();
        assert_eq!(buffer.cursor_pos, 0);
        buffer.step_word_right();
        assert_eq!(buffer.cursor_pos, 4);
        buffer.step_word_right();
        assert_eq!(buffer.cursor_pos, 11);
        buffer.step_word_right();
        assert_eq!(buffer.cursor_pos, 17);
        buffer.step_word_right();
        assert_eq!(buffer.cursor_pos, 17);
    }

    #[test]
    fn test_end_start_navigation() {
        let mut buffer = get_command();
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
        let mut buffer = get_command();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.step_word_right();
        buffer.delete_from_start();
        assert_eq!(buffer.get_buffer(), " random words");
    }

    #[test]
    fn test_delete_from_start_of_line() {
        let mut buffer = get_command();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.step_word_right();
        buffer.step_word_right();
        buffer.delete_to_end();
        assert_eq!(buffer.get_buffer(), "some random");
    }

    #[test]
    fn test_delete_right() {
        let mut buffer = get_command();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.step_word_right();
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
        let mut buffer = get_command();
        push_string(&mut buffer, "some random words");
        buffer.move_to_end();
        buffer.delete_word_left();
        assert_eq!(buffer.get_buffer(), "some random ");
        buffer.move_to_start();
        buffer.step_word_right();
        buffer.delete_word_left();
        assert_eq!(buffer.get_buffer(), " random ");
    }

    #[test]
    fn test_delete_word_right() {
        let mut buffer = get_command();
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.delete_word_right();
        assert_eq!(buffer.get_buffer(), " random words");
        buffer.delete_word_right();
        assert_eq!(buffer.get_buffer(), " words");
    }

    #[test]
    fn test_fancy_chars() {
        let mut buffer = get_command();
        let input = "some weird chars: ÅÖÄø æĸœ→ €ßðßª“";
        push_string(&mut buffer, input);
        assert_eq!(input.chars().count(), buffer.buffer.len());
        assert_ne!(input.len(), buffer.buffer.len());
        assert_eq!(buffer.get_buffer().len(), input.len());
    }

    #[test]
    fn test_human_key() {
        use super::human_key;

        assert_eq!(human_key("alt-", '\u{7f}'), "alt-backspace");
        assert_eq!(human_key("ctrl-", '\u{7f}'), "ctrl-backspace");
        assert_eq!(human_key("alt-", '\u{1b}'), "alt-escape");
        assert_eq!(human_key("ctrl-", '\u{1b}'), "ctrl-escape");
        assert_eq!(human_key("ctrl-", 'd'), "ctrl-d");
        assert_eq!(human_key("f", 'x'), "fx");
    }
}
