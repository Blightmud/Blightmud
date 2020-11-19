use std::sync::mpsc::Sender;

use rlua::UserData;

use crate::{event::Event, model::Line};

#[derive(Clone)]
pub struct Backend {
    pub writer: Sender<Event>,
    pub lines: Vec<Line>,
}

impl Backend {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            writer,
            lines: vec![],
        }
    }
}

impl UserData for Backend {}
