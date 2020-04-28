use std::sync::{atomic::AtomicBool, mpsc::Sender, Arc};

#[derive(Clone)]
pub struct Session {
    pub terminate: Arc<AtomicBool>,
    pub transmit_writer: Sender<Vec<u8>>,
    pub output_writer: Sender<Vec<u8>>,
    pub input_writer: Sender<String>,
    pub ui_update_notifier: Sender<bool>,
    pub input_buffer_write: Sender<String>,
}

#[derive(Clone)]
pub struct SessionBuilder {
    terminate: Arc<AtomicBool>,
    transmit_writer: Option<Sender<Vec<u8>>>,
    output_writer: Option<Sender<Vec<u8>>>,
    input_writer: Option<Sender<String>>,
    ui_update_notifier: Option<Sender<bool>>,
    input_buffer_write: Option<Sender<String>>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            terminate: Arc::new(AtomicBool::new(false)),
            transmit_writer: None,
            output_writer: None,
            input_writer: None,
            ui_update_notifier: None,
            input_buffer_write: None,
        }
    }

    pub fn transmit_writer(mut self, transmit_writer: Sender<Vec<u8>>) -> Self {
        self.transmit_writer = Some(transmit_writer);
        self
    }

    pub fn output_writer(mut self, output_writer: Sender<Vec<u8>>) -> Self {
        self.output_writer = Some(output_writer);
        self
    }

    pub fn input_writer(mut self, input_writer: Sender<String>) -> Self {
        self.input_writer = Some(input_writer);
        self
    }

    pub fn ui_update_notifier(mut self, ui_update_notifier: Sender<bool>) -> Self {
        self.ui_update_notifier = Some(ui_update_notifier);
        self
    }

    pub fn input_buffer_write(mut self, input_buffer_write: Sender<String>) -> Self {
        self.input_buffer_write = Some(input_buffer_write);
        self
    }

    pub fn build(self) -> Session {
        Session {
            terminate: self.terminate,
            transmit_writer: self.transmit_writer.unwrap(),
            output_writer: self.output_writer.unwrap(),
            input_writer: self.input_writer.unwrap(),
            ui_update_notifier: self.ui_update_notifier.unwrap(),
            input_buffer_write: self.input_buffer_write.unwrap(),
        }
    }
}
