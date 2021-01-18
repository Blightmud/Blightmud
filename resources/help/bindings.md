# Bindings

It is possible bind certain key commands in lua script to perform actions when
pressed. This also include rebinding keys to a setup you are more comfortable
with rather then the default.

- `Ctrl-<char>` eg. `Ctrl-a, Ctrl-b` but not `Ctrl-PgUp`, there is no distinction for capitalization
- `Alt-<char>` eg. `Alt-a, Alt-b` but not `Alt-PgUp`, there is no distinction for capitalization
- `F1-F12`

You may also bind on escape sequences. For example `\x1b[1;5A` (ctrl-up). When
unbound, blightmud will echo these commands to the output when pressed. This
will make it easy for you to find the escape sequence you want to bind.

***blight.bind(cmd, callback)***
Is the command to use when creating a binding.

`cmd` has to be in the following format:
- `Ctrl-{}` where {} is a character, eg. a, b, c, etc.
- `Alt-{}` where {} is a character, eg. a, b, c, etc.
- `fn` where n is a number from 1-12
- Or an escape sequence such as `\x1b[1;5A`

```lua
blight.bind("f1", function ()
    blight.send("kick " .. target)
end)
blight.bind("\x1b[1;5D", function ()
    blight.ui("step_word_left")
end)
```

***blight.unbind(cmd)***
Is the command to use when you want to remove a binding
You can't unbind `Ctrl-c` or `Ctrl-l`

***blight.ui(cmd)***
Allows for interactions with the UI.

The following options are available for `cmd`:
- `"step_left"`         : Moves the cursor left
- `"step_right"`        : Moves the cursor right
- `"step_to_start"`     : Moves the cursor to the start of the input line
- `"step_to_end"`       : Moves cursor to the end of the input line
- `"step_word_left"`    : Moves cursor left by one word
- `"step_word_right"`   : Moves cursor right by one word
- `"delete"`            : Deletes the character before the cursor
- `"delete_right"`      : Deletes the character after the cursor
- `"delete_word_left"`  : Deletes the word after the cursor
- `"delete_word_right"` : Deletes the word before the cursor
- `"delete_to_end"`     : Deletes from the cursor to the end of the line
- `"delete_from_start"` : Deletes from the start of the input line to the cursor
- `"previous_command"`  : Get the previous input command
- `"next_command"`      : Get the next input command
- `"scroll_up"`         : Scroll output view up
- `"scroll_down"`       : Scroll output view down
- `"scroll_top"`        : Scroll output view to the top
- `"scroll_bottom"`     : Scroll the output view to the bottom
- `"complete"`          : Perform *tab-completion* on the current word

What follows is the default configuration that blightmud starts with. You can
override this as you please using `blight.unbind` and `blight.bind`

```lua
local function bind(cmd, event)
	blight.bind(cmd, function () blight.ui(event) end)
end

bind("ctrl-p", "previous_command")
bind("ctrl-n", "next_command")
bind("alt-b", "step_word_left")
bind("\x1b[1;5D", "step_word_left")
bind("alt-f", "step_word_right")
bind("\x1b[1;5C", "step_word_right")
bind("alt-backspace", "delete_word_left")
bind("alt-d", "delete_word_right")
bind("ctrl-a", "step_to_start")
bind("ctrl-b", "step_left")
bind("ctrl-e", "step_to_end")
bind("ctrl-f", "step_right")
bind("ctrl-d", "delete_right")
bind("ctrl-h", "delete")
bind("ctrl-k", "delete_to_end")
bind("ctrl-u", "delete_from_start")

blight.bind("ctrl-s", function () tts:stop() end)
```
