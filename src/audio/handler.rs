use crate::event::{BadEventRoutingError, Event};
use anyhow::Result;

use super::Player;

pub fn handle_audio_event(event: Event, player: &mut Player) -> Result<()> {
    match event {
        Event::PlayMusic(path, repeat) => player.play_music(&path, repeat),
        Event::StopMusic => player.stop_music(),
        Event::PlaySFX(path) => player.play_sfx(&path),
        Event::StopSFX => player.stop_sfx(),
        _ => Err(BadEventRoutingError.into()),
    }
}
