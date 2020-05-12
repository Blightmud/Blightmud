use crate::event::Event;
use std::sync::mpsc::Sender;

pub fn output_stack_trace(writer: &Sender<Event>, error: &str) {
    writer
        .send(Event::Error("[Lua] Script error:".to_string()))
        .unwrap();
    for line in error.split('\n') {
        writer
            .send(Event::Error(format!("\t{}", line).to_string()))
            .unwrap();
    }
}
