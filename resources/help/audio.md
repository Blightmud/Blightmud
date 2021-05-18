# Audio

This module allows you to play audio through Blightmud. Useful for playing some
tunes or adding audio support to a mud.

There are two available channels to play audio through. `music` and `sfx`.
Behind the scenes they are the same thing. Audio sinks to play audio from. The
reason there are two is so that you may play ambiance or background music and
also sound effects in paralell.

The audio module supports the following formats:

- MP3
- WAV
- Vorbis
- Flac

## Playback options

The following options can be provided to a **playback function**. The options
should be provided as a table with the following optional keys:

- `loop`    Loop the sound infinitely (music only)
- `amplify` A float value to amplify the sound. (1.0 is default)

##

***audio.play_music(path[, options])***
Queues up an audio file to play. If there is already music playing then the new
file will be played as soon as the current one finishes. Note that `repeat`
will prevent a file from ever completing.

- `path`    Path to the audio file you want to play
- `options` Playback options *(optional)*

##

***audio.stop_music()***
Clears the music play queue and stops output.

##

***audio.play_sfx(path[, options])***
Queues up an audio file to play. If there is already sound playing then the
provided will play after those have completed.

- `path`    The path to the audio file to play.
- `options` Playback options *(optional)*

##

***audio.stop_sfx()***
Stops all sfx playback and clears the queue.
