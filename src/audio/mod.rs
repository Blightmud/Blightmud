#[cfg(feature = "audio")]
mod handler;
#[cfg(feature = "audio")]
mod player;

#[cfg(feature = "audio")]
pub use self::{handler::handle_audio_event, player::Player, player::SourceOptions};

#[cfg(not(feature = "audio"))]
mod stub;
#[cfg(not(feature = "audio"))]
pub use self::stub::{handle_audio_event, Player, SourceOptions};
