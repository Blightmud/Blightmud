#[allow(dead_code)]
pub enum Event {
    ServerOutput(Vec<u8>),
    ServerInput(String),
    LocalOutput(String),
    UserInputBuffer(String),
    Connect(String, u32),
    LoadScript(String),
    Disconnect,
    Quit,
}
