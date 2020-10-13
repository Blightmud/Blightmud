# Text to Speech

Blightmud has built in support for Text-to-speech (TTS).

You can enable TTS using `/tts on|off` or by providing the flag `--tts` on the commandline.
This will make everything printed spoken through TTS.

Make sure you disable your screen reader before you do this as blightmud and your screen
reader software sharing the same speech dispatcher isn't always a match made
in heaven.

If you want to use TTS just for notifications and other special information
you can interact with it through lua.

## Macros

`/tts on|off`               Enable or disable TTS
`/tts_rate <rate>`          Set the TTS rate
`/tts_keypresses on|off`    Toggle key press speaking when typing

## Settings

Any of the various settings included in tts will be persisted between
blightmud restarts so you only need to configure this once.

## Functions

***tts:speak(msg, interupt)***
Will speak the provided `msg`. If interupt is true, this message will interupt
possible messages that are waiting to be spoken.

##

***tts:speak_direct(msg)***
Will speak the provided `msg` directly and interrupt anything that's being said
at the moment but it won't clear subsequent messages in queue. This message
will not be stored in the TTS history.

##

***tts:enable(enabled)***
Toggle general TTS on or off. Where `enabled` is wither true or false.

##

***tts:echo_keypresses(enabled)***
Toggle if TTS should speak keypresses when typing or not

##

***tts:set_rate(rate)***
Set the speech rate. Default is usually 0, max is 100 and min is -100. This can
vary on different operating systems.

##

***tts:change_rate(change)***
Increase or decrease the rate of speech

##

***tts:gag()***
Used from within a triggers callback function this will prevent the matched
line from being spoken through TTS.

See `/help triggers` for details about triggers.

##

***tts:step_back(step)***
Move the current reading index back by `step` rows. TTS will continue reading
lines from the point you step forward to.

##

***tts:step_forward(step)***
Move the current reading index forward by `step` rows. TTS will continue reading
lines from the point you step forward to.

##

***tts:scan_back(step)***
Read out the line `step` lines back from the scan index.

##

***tts:scan_forward(step)***
Read out the line `step` lines forward from the scan index.

##

***tts:stop()***
Stop all speach and move the reading index and the scan index to the bottom of
the output.

## Bindings

By default `ctrl-s` is bound to stop current TTS and clear the queue.
You can rebind this as you please. See `/help bindings`
