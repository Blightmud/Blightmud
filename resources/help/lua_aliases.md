# Aliases

Aliases allow you to trigger a callback function when a certain command is typed.

## Creating an Alias

***blight:add_alias(regex, callback) -> id***
Creates an alias which when triggered runs the provided callback function.

- `regex`    A regular expression to match as the command name.
- `callback` A Lua function that gets called when the regex is matched.
- Returns an id for the created alias (used for removing)

***blight:enable_alias(id, enabled)***

- `id`         The id of the alias to enabled/disable
- `enabled`    Boolean toggling the enabled flag on the alias

***blight:remove_alias(alias_id)***

- `alias_id` An id returned upon creation of the alias

```lua
local target = "dog"

local alias_id = blight:add_alias("^kick$", function ()
    blight:send("kick " .. target)
end)

blight:remove_alias(alias_id)
```

***blight:get_aliases()***

- Returns a table containing all aliases created by the user.

```lua
-- Return data
{
    id: Alias
}

-- Alias object
{
    regex: String,
    enabled: bool,
}
```

***blight:clear_aliases()***

Removes all aliases
