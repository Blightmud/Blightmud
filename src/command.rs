use crate::event::Event;
use crate::session::Session;
use log::debug;
use std::collections::VecDeque;
use std::thread;
use std::{io::stdin, sync::atomic::Ordering};
use termion::{event::Key, input::TermRead};

#[derive(Default)]
struct CommandBuffer {
    buffer: String,
    cached_buffer: String,
    history: VecDeque<String>,
    current_index: usize,
    cursor_pos: usize,
}

impl CommandBuffer {
    fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    fn get_pos(&self) -> usize {
        self.cursor_pos
    }

    fn submit(&mut self) -> &str {
        self.history.push_back(self.buffer.clone());
        while self.history.len() > 100 {
            self.history.pop_front();
        }
        self.current_index = self.history.len();
        self.buffer.clear();
        self.cursor_pos = 0;
        &self.history[self.current_index - 1]
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
        if self.cursor_pos < self.buffer.len() {
            self.buffer.remove(self.cursor_pos - 1);
        } else {
            self.buffer.pop();
        }
        self.move_left();
    }

    fn push_key(&mut self, c: char) {
        if self.cursor_pos >= self.buffer.len() {
            self.buffer.push(c);
        } else {
            self.buffer.insert(self.cursor_pos, c);
        }
        self.move_right();
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

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('\n') => {
                    writer.send(parse_command(buffer.submit())).unwrap();
                }
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

            if p1 == None || p2 == None {
                Event::Info("USAGE: /connect <host> <port>".to_string())
            } else {
                let p1 = p1.unwrap().to_string();
                if let Ok(p2) = p2.unwrap().parse::<u32>() {
                    Event::Connect(p1, p2)
                } else {
                    Event::Error(
                        "USAGE: /connect <host: String> <port: Positive number>".to_string(),
                    )
                }
            }
        }
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
}
