// Stub types for when audio feature is disabled
// These are used when compiling without the 'audio' feature flag

#[derive(Debug, Clone, PartialEq)]
pub struct SourceOptions {
    pub repeat: bool,
    pub amplify: f32,
}

impl Default for SourceOptions {
    fn default() -> Self {
        Self {
            repeat: false,
            amplify: 1.0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Player;

impl Player {
    pub fn new() -> Self {
        Self
    }
}

pub fn handle_audio_event(_event: crate::event::Event, _player: &mut Player) -> anyhow::Result<()> {
    Ok(())
}
