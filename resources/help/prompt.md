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

