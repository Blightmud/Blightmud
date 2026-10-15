# Bindings

It is possible bind certain key commands in lua script to perform actions when
pressed. This also include re-binding keys to a setup you are more comfortable
with rather then the default.

- `Ctrl-<char>` ex. `Ctrl-a, Ctrl-b` but not `Ctrl-PgUp`, there is no distinction for capitalization
- `Alt-<char>` ex. `Alt-a, Alt-b` but not `Alt-PgUp`, there is distinction for capitalization.
  For example `Alt-H`, which is basically `Alt-Shift-h`, and `Alt-h` are treated
  as different bindings.
- `F1-F12`

You may also bind on escape sequences. For example `\x1b[1;5A` (ctrl-up). When
unbound, blightmud will echo these commands to the output when pressed. This
will make it easy for you to find the escape sequence you want to bind.

***blight.bind(cmd, callback)***
Is the command to use when creating a binding.

`cmd` has to be in the following format:
- `Ctrl-{}` where {} is a character, ex. a, b, c, etc.
- `Alt-{}` where {} is a character, ex. a, b, c, A, B, C, etc.
- `fn` where n is a number from 1-12
- Or an escape sequence such as `\x1b[1;5A`

```lua
blight.bind("f1", function ()
    mud.send("kick " .. target)
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
- `"scroll_up"`         : Scroll output view up
- `"scroll_down"`       : Scroll output view down
- `"scroll_top"`        : Scroll output view to the top
- `"scroll_bottom"`     : Scroll the output view to the bottom
- `"complete"`          : Perform *tab-completion* on the current word
- `"insert_newline"`    : Insert a row break in the input area
- `"step_up"`           : Move the cursor to the previous row of the input area
- `"step_down"`         : Move the cursor to the next row of the input area

What follows is the default configuration that blightmud starts with. You can
override this as you please using `blight.unbind` and `blight.bind`

```lua
local function bind(cmd, event)
    blight.bind(cmd, function()
        blight.ui(event)
    end)
end

bind("alt-b", "step_word_left")
bind("ctrl-left", "step_word_left") -- Ctrl + left
bind("alt-f", "step_word_right")
bind("ctrl-right", "step_word_right") -- Ctrl + right
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

-- Scrolling
bind("home", "scroll_top")
bind("end", "scroll_bottom")
bind("pageup", "scroll_up")
bind("pagedown", "scroll_down")

-- ctrl + up/down
blight.bind("ctrl-up", search.find_up)
blight.bind("ctrl-down", search.find_down)

-- ctrl + pgup/pgdn
blight.bind("\x1b[5;5~", search.find_last_input)
blight.bind("\x1b[6;5~", search.find_next_input)
blight.bind("ctrl-s", function()
    tts:stop()
end)

-- Insert a row in the input area
set_newline_keys({ "ctrl-o", "alt-enter", "\x1b\r", "\x1b\n" })

-- History navigation. Up/down move within the input area when there is
-- somewhere to move to, and fall through to history at the edges.
blight.bind("up", function()
    if prompt.cursor_row() > 1 then
        blight.ui("step_up")
    else
        history.previous_command()
    end
end)
blight.bind("down", function()
    if prompt.cursor_row() < prompt.row_count() then
        blight.ui("step_down")
    else
        history.next_command()
    end
end)
blight.bind("ctrl-p", history.previous_command)
blight.bind("ctrl-n", history.next_command)

-- Toggle tag rendering
blight.bind("ctrl-t", function()
    print("Toggling tags")
    blight.show_tags(not blight.show_tags())
end)
```

## Inserting a row in the input area

`Enter` always submits. To compose a multi-row message you insert row breaks
with a separate key, and several are bound by default:

- `Ctrl-O` — works everywhere with no terminal configuration.
- `Alt-Enter` — needs Meta enabled; see the macOS notes below.
- The raw escape forms `\x1b\r` and `\x1b\n`, because some terminal and tmux
  combinations deliver Option+Enter as unrecognised bytes rather than as an
  Alt key, in which case the named binding never fires but this one does.

`Ctrl-J` is *not* usable — terminals encode it identically to `Enter`.

***set_newline_keys(keys)***
Replaces the whole group in one call, so you do not have to know and unbind
each default individually.

```lua
set_newline_keys({ "alt-enter" })             -- Alt/Option+Enter only
set_newline_keys({ "ctrl-o" })                -- macOS-friendly, no Meta needed
set_newline_keys({ "ctrl-o", "\x1b[13;2u" })   -- and a CSI-u Shift+Enter
set_newline_keys({})                          -- disable row insertion entirely
```

## macOS keyboards

Three things about macOS terminals are worth knowing, and they are why
`Ctrl-O` rather than `Alt-Enter` is the primary binding:

1. **Option is not Meta by default.** Terminal.app and iTerm2 both ship with
   Option producing accented characters (`Option+a` gives `å`) rather than an
   ESC prefix. Until you turn it on, Option+Enter sends nothing bindable — and
   the same applies to the whole default `alt-*` group above.
   - Terminal.app: Settings → Profiles → Keyboard → **Use Option as Meta key**
   - iTerm2: Settings → Profiles → Keys → Left/Right Option key → **Esc+**
2. **Command is invisible to terminals.** No terminal sends a Cmd chord to the
   foreground process, so `blight.bind("cmd-enter", ...)` can never fire. To use
   it, map Cmd+Enter to a custom escape sequence in your terminal and bind that.
3. **Ctrl+Enter is indistinguishable from Enter**, and so is Shift+Enter: legacy
   terminals encode all three as a carriage return. Only terminals implementing
   the Kitty keyboard protocol / CSI-u can tell them apart.

If you would rather not change any terminal settings, use `Ctrl-O` — it works
as shipped in Terminal.app, iTerm2, Ghostty, WezTerm, Alacritty and kitty, and
inside tmux and screen.

To reach Shift+Enter or Cmd+Enter, configure your terminal to emit a CSI-u
sequence for it and bind that sequence directly:

```lua
set_newline_keys({ "ctrl-o", "\x1b[13;2u" })  -- CSI-u Shift+Enter
```

Blightmud needs no special support for this: unrecognised escape sequences are
echoed to the output when unbound, so you can press the key, read the bytes
back, and bind exactly what your terminal sent.
