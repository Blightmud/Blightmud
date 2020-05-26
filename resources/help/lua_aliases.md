# Aliases

Aliases allow you to trigger a callback function when a certain command is typed.

## Creating an Alias

***blight:add_alias(regex, callback)***
Creates an alias which when triggered runs the provided callback function.

- `regex`    A regular expression to match as the command name.
- `callback` A Lua function that gets called when the regex is matched.

```lua
local target = "dog"
blight:add_alias("^kick$", function ()
    blight:send("kick " .. target)
end)
```
