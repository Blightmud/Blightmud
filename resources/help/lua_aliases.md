# Aliases

Aliases allow you to trigger a callback function when a certain command is typed.

## Creating an Alias

***blight:add_alias(regex, callback) -> id***
Creates an alias which when triggered runs the provided callback function.

- `regex`    A regular expression to match as the command name.
- `callback` A Lua function that gets called when the regex is matched.
- Returns an id for the created alias (used for removing)

***blight:remove_alias(alias_id)***

- `alias_id` An id returned upon creation of the alias

```lua
local target = "dog"

local alias_id = blight:add_alias("^kick$", function ()
    blight:send("kick " .. target)
end)

blight:remove_alias(alias_id)
```

***blight:get_alias_ids()***
Returns a list of all created alias ids

***blight:clear_aliases()***
Remove all aliases
