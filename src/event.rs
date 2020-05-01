#[allow(dead_code)]
pub enum Event {
    ServerOutput(Vec<u8>),
    Prompt,
    ServerSend(Vec<u8>),
    ServerInput(String),
    Output(String),
    Error(String),
    Info(String),
    UserInputBuffer(String),
    Connect(String, u32),
    LoadScript(String),
    Disconnect,
    Quit,
}
