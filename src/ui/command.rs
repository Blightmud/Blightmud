use crate::event::QuitMethod;
use crate::model::{Completions, Line, Servers};
use crate::{event::Event, tts::TTSController};
use crate::{lua::LuaScript, lua::UiEvent, session::Session, SaveData};
use log::debug;
use rs_complete::CompletionTree;
use std::collections::HashSet;
use std::thread;
use std::{
    io::stdin,
    sync::{mpsc::Sender, Arc, Mutex},
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
    buffer: Vec<char>,
    cursor_pos: usize,
    completion_tree: CompletionTree,
    completion: CompletionStepData,
    script: Arc<Mutex<LuaScript>>,
    tts_ctrl: Arc<Mutex<TTSController>>,
}

impl CommandBuffer {
    pub fn new(tts_ctrl: Arc<Mutex<TTSController>>, script: Arc<Mutex<LuaScript>>) -> Self {
        let mut completion = CompletionTree::with_inclusions(&['/', '_']);
        completion.set_min_word_len(3);

        Self {
            buffer: vec![],
            cursor_pos: 0,
            completion_tree: completion,
            completion: CompletionStepData::default(),
            script,
            tts_ctrl,
        }
    }

    pub fn get_buffer(&mut self) -> String {
        self.buffer.iter().collect::<String>()
    }

    pub fn get_pos(&self) -> usize {
        self.cursor_pos
    }

    fn submit(&mut self) -> String {
        // Insert history
        let cmd = if !self.buffer.is_empty() {
            let command = self.get_buffer();
            self.completion_tree.insert(&command);
            command
        } else {
            String::new()
        };

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

    fn remove(&mut self) -> Option<char> {
        if self.cursor_pos > 0 {
            let removed = if self.cursor_pos < self.buffer.len() {
                Some(self.buffer.remove(self.cursor_pos - 1))
            } else {
                self.buffer.pop()
            };
            self.step_left();
            removed
        } else {
            None
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
                let mut completions = Completions::default();
                let strbuf = self.get_buffer();
                completions.merge(self.script.lock().unwrap().tab_complete(&strbuf));
                if let Some(mut options) = self.completion_tree.complete(&strbuf) {
                    completions.add_all(&mut options);
                }

                // Remove duplicates but preserve order of occurence
                let mut occurences: HashSet<&String> = HashSet::new();
                let completions = completions.iter().fold(vec![], |mut acc, word| {
                    if !occurences.contains(word) {
                        acc.push(word.clone());
                    }
                    occurences.insert(word);
                    acc
                });

                self.completion.set_options(&strbuf, completions);
            }
            if let Some(comp) = self.completion.next() {
                self.tts_ctrl.lock().unwrap().speak(comp, true);
                self.buffer = comp.chars().collect();
                self.cursor_pos = self.buffer.len();
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_pos = self.buffer.len();
    }

    pub fn set(&mut self, line: String) {
        self.buffer = line.chars().collect();
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
    script: &mut Arc<Mutex<LuaScript>>,
) {
    match key {
        Key::Char('\n') => {
            let mut line = Line::from(buffer.submit());
            line.flags.source = Some("user".to_string());
            writer.send(Event::ServerInput(line)).unwrap();
            if let Ok(mut script) = script.lock() {
                script.set_prompt_content(String::new());
            }
        }
        Key::Char('\t') => buffer.tab_complete(),
        Key::Char(c) => {
            tts_ctrl.lock().unwrap().key_press(c);
            buffer.push_key(c);
            if let Ok(mut script) = script.lock() {
                script.set_prompt_content(buffer.get_buffer());
            }
        }
        Key::Ctrl('l') => writer.send(Event::Redraw).unwrap(),
        Key::Ctrl('c') => {
            writer.send(Event::Quit(QuitMethod::CtrlC)).unwrap();
        }
        Key::PageUp => writer.send(Event::ScrollUp).unwrap(),
        Key::PageDown => writer.send(Event::ScrollDown).unwrap(),
        Key::Home => writer.send(Event::ScrollTop).unwrap(),
        Key::End => writer.send(Event::ScrollBottom).unwrap(),

        // Input navigation
        Key::Left => buffer.step_left(),
        Key::Right => buffer.step_right(),
        Key::Backspace => {
            if let Some(c) = buffer.remove() {
                if let Ok(mut tts_ctrl) = tts_ctrl.lock() {
                    tts_ctrl.key_press(c);
                }
            }
            if let Ok(mut script) = script.lock() {
                script.set_prompt_content(buffer.get_buffer());
            }
        }
        Key::Delete => buffer.delete_right(),
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
            Key::Up => script.check_bindings("up"),
            Key::Down => script.check_bindings("down"),
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
            UiEvent::Remove => {
                buffer.remove();
            }
            UiEvent::DeleteToEnd => buffer.delete_to_end(),
            UiEvent::DeleteFromStart => buffer.delete_from_start(),
            UiEvent::DeleteWordLeft => buffer.delete_word_left(),
            UiEvent::DeleteWordRight => buffer.delete_word_right(),
            UiEvent::DeleteRight => buffer.delete_right(),
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
            let mut script = session.lua_script.clone();
            let stdin = stdin();
            let buffer = session.command_buffer.clone();
            let mut tts_ctrl = session.tts_ctrl;

            if let Ok(mut buffer) = buffer.lock() {
                for server in Servers::load().keys() {
                    buffer.completion_tree.insert(server);
                }
                buffer
                    .completion_tree
                    .insert(include_str!("../../resources/completions.txt"));
            }

            for e in stdin.events() {
                match e.unwrap() {
                    termion::event::Event::Key(key) => {
                        if let Ok(mut buffer) = buffer.lock() {
                            if !check_command_binds(key, &mut buffer, &script, &writer) {
                                parse_key_event(
                                    key,
                                    &mut buffer,
                                    &writer,
                                    &mut tts_ctrl,
                                    &mut script,
                                );
                            }
                            writer
                                .send(Event::UserInputBuffer(
                                    buffer.get_buffer(),
                                    buffer.get_pos(),
                                ))
                                .unwrap();
                        }
                    }
                    termion::event::Event::Mouse(event) => parse_mouse_event(event, &writer),
                    termion::event::Event::Unsupported(bytes) => {
                        if let Ok(escape) = String::from_utf8(bytes.clone()) {
                            if let Ok(mut buffer) = buffer.lock() {
                                check_escape_bindings(
                                    &escape.to_lowercase(),
                                    &mut buffer,
                                    &script,
                                    &writer,
                                );
                            }
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

#[cfg(test)]
mod command_test {

    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::{Arc, Mutex};

    use super::CommandBuffer;
    use crate::lua::LuaScriptBuilder;
    use crate::tts::TTSController;
    use crate::Event;

    fn push_string(buffer: &mut CommandBuffer, msg: &str) {
        msg.chars().for_each(|c| buffer.push_key(c));
    }

    fn get_command() -> (CommandBuffer, Receiver<Event>) {
        let (tx, rx): (Sender<Event>, Receiver<Event>) = channel();
        let buffer = CommandBuffer::new(
            Arc::new(Mutex::new(TTSController::new(false, true))),
            Arc::new(Mutex::new(
                LuaScriptBuilder::new(tx).dimensions((100, 100)).build(),
            )),
        );
        (buffer, rx)
    }

    #[test]
    fn test_editing() {
        let mut buffer = get_command().0;

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
        let mut buffer = get_command().0;
        buffer.push_key('t');
        buffer.step_left();
        assert_eq!(buffer.get_pos(), 0);
        buffer.remove();
        assert_eq!(buffer.get_pos(), 0);
    }

    #[test]
    fn test_input_navigation() {
        let mut buffer = get_command().0;
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
        let mut buffer = get_command().0;
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
        let mut buffer = get_command().0;
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.step_word_right();
        buffer.delete_from_start();
        assert_eq!(buffer.get_buffer(), " random words");
    }

    #[test]
    fn test_delete_from_start_of_line() {
        let mut buffer = get_command().0;
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.step_word_right();
        buffer.step_word_right();
        buffer.delete_to_end();
        assert_eq!(buffer.get_buffer(), "some random");
    }

    #[test]
    fn test_delete_right() {
        let mut buffer = get_command().0;
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
        let mut buffer = get_command().0;
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
        let mut buffer = get_command().0;
        push_string(&mut buffer, "some random words");
        buffer.move_to_start();
        buffer.delete_word_right();
        assert_eq!(buffer.get_buffer(), " random words");
        buffer.delete_word_right();
        assert_eq!(buffer.get_buffer(), " words");
    }

    #[test]
    fn test_fancy_chars() {
        let mut buffer = get_command().0;
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

    #[test]
    fn test_completions() {
        let mut buffer = get_command().0;
        push_string(&mut buffer, "batman");
        buffer.submit();
        push_string(&mut buffer, "bat");
        buffer.tab_complete();
        assert_eq!(buffer.completion.options, vec!["batman".to_string()]);
    }

    #[test]
    fn test_completion_with_big_chars() {
        // Issue #522
        let mut buffer = get_command().0;
        push_string(&mut buffer, "fend");
        buffer.completion.options = vec!["fender🎸".to_string()];
        buffer.tab_complete();
        assert_eq!(buffer.completion.options, vec!["fender🎸".to_string()]);
        assert_eq!(buffer.buffer, vec!['f', 'e', 'n', 'd', 'e', 'r', '🎸']);
        assert_eq!(buffer.cursor_pos, 7);
    }
}
