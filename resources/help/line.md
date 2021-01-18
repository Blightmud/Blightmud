# Line

The line object is the object you receive through triggers or when receiving
input to the mud or output from it.

##

***line:line() -> String***
Returns the line without escape sequences and color

##

***line:raw() -> String***
Returns the complete line as received from the mud.

##

***line:gag([val]) -> bool***
Get or set the `gag` flag on this line. The `gag` flag will prevent it from
being rendered on screen.

##

***line:tts_gag([val]) -> bool***
Get or set the `tts_gag` flag on this line. The `tts_gag` flag will prevent it
from being spoken by TTS.

##

***line:tts_interrupt([val]) -> bool***
Get or set the `tts_interrupt` flag on this line. The `tts_interrupt` flag will
make this line interupt anything currently being spoken by TTS.

##

***line:skip_log([val]) -> bool***
Get or set the `skip_log` flag on this line. The `skip_log` will exclude this
line from the log.

##

***line:prompt() -> bool***
Returns if this is a prompt line or not

##

***line:replace(string)***
Replaces the content of this line with the provided content.  Repeated calls to
this method will result in the last calls content becoming the new content for
the line. This is primarily intended for plugin use and for regular scripting
utilizing `Line:gag(true)` and `print()` should suffice.

- `string`  The new content for this line

##

***line:replacement() -> String***
Returns the replacement content for this line or nil if nothing has been set.

##

***line:matched([val]) -> bool***
Get or set the `matched` flag on this line. The `matched` flag tells if this line
has been matched by a trigger or not. If you are writing advanced plugins whith
full output capturing you are responsible for setting this yourself.
