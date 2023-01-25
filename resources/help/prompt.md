# Prompt

This module offers methods to interact with data that has
been typed on the prompt line

##

***prompt.get() -> String***
Returns the line currently typed into the prompt

##

***prompt.set(input)***
Sets the line typed in the prompt. Replacing any current data.

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

