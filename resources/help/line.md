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

##

***line:source() -> String***

Return the source of the line.

Primarly used for input handling to differ between user input commands and
script input commands.

Possible values are:

- `"user"`    When the line is coming from the users prompt.
- `"script"`  When the line is sent to the mud from a lua script.
- `nil`       When the line comes from neither of the above.

##

***line:tag_color(string) -> String***

Get or set the ANSI color code used to render this line's tag symbol. When
there is no color set the tag symbol won't be rendered.

- `color`  An ANSI escape sequence string, e.g. `"\x1b[31m"` or `C_RED` for red.

##

***line:tag_key(string) -> String***

Get or set an arbitrary key associated with this line's tag. This can then be
referenced when filtering the output view on certain tags.

##

***line:tag_symbol(char) -> String***

Get or set the character used as the tag symbol. Defaults to `┃` (U+2503,
BOX DRAWINGS HEAVY VERTICAL). Only rendered when a tag color is set.
