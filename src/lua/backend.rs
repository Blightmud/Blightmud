use std::sync::mpsc::Sender;

use mlua::UserData;

use crate::event::Event;

#[derive(Clone)]
pub struct Backend {
    pub writer: Sender<Event>,
}

impl Backend {
    pub fn new(writer: Sender<Event>) -> Self {
        Self { writer }
    }
}

impl UserData for Backend {}
