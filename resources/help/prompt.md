# Prompt

This module offers methods to interact with data that has
been typed on the prompt line.

See also `/help prompt_mask`.

##

***prompt.get() -> String***
Returns the line currently typed into the prompt

##

***prompt.set(input)***
Sets the line typed in the prompt. Replacing any current data.
This set the cursor position to the end of the new prompt.

##

***prompt.get_cursor_pos() -> number***
Gets the cursor position in the input prompt

- Returns the current index of the cursor starting at 0

##

***prompt.set_cursor_pos(pos)***
Sets the cursor position in the input prompt.

- `pos` A positive number

Attempting to set the position to a value larger then the length of the current
prompt input will move the cursor to the end of the prompt input.
Attempting to set the position to a negative value will triggger an error.

##

***prompt.add_prompt_listener(callback)***
Registers a callback that is triggered when data has been typed on the prompt
line, or set with `prompt.set`.

- `callback`   A Lua function to be called each prompt line update. (line)

```lua
blight.add_prompt_listener(function (line)
    blight.output("Prompt buffer is currently:", line)
end)
```

##

***prompt.cursor_row() -> number***
Gets the row the cursor is on within the input area, starting at 1.

Rows here are the ones separated by newlines in the input buffer, not visual
wrap rows. A buffer with no newlines is always one row.

##

***prompt.row_count() -> number***
Gets the number of newline-separated rows in the input buffer, minimum 1.

Together with `prompt.cursor_row()` this lets a binding decide whether there is
somewhere to move before acting — which matters because `blight.ui` actions are
applied *after* the binding returns, so a binding cannot observe whether a
motion actually moved.

```lua
blight.bind("up", function()
    if prompt.cursor_row() > 1 then
        blight.ui("step_up")
    else
        history.previous_command()
    end
end)
```

##

***blight.input_height([height]) -> int***
Gets or sets the height, in rows, of the user input area. Defaults to `1`.

- `height`  Optional. The requested minimum height, clamped to `1..=10`.
- Returns the height the screen actually has.

The returned value is what the screen *granted*, which is not always what was
requested: a terminal too short to honour it gets fewer rows, and reader mode
is always one row. NAWS reports this number to the MUD, so it has to be the
truth rather than an echo.

With `input_auto_expand` on, this is the *minimum* height and the area grows
above it as the content requires, shrinking back when the content does. With it
off — the default — this is a fixed height.

This is not persisted. Set it from your config script:

```lua
blight.input_height(3)
```
