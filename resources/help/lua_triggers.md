# Triggers

Triggers allow Blightmud to execute a callback function in response to output
from the connected server.

## Creating a Trigger
***blight:add_trigger(regex, options, callback) -> id***

Triggers will by default match against a clean line of text (ansi escapes
removed) so you don't have to account for this when you are writing
regexps.

- `regex`    The regular expression to match the server output against.
- `options`  A table of options (see `Trigger Options` below)
- `callback` The Lua function that gets called when a match is found.
- Returns a trigger id (used for removing the trigger)

## Trigger Options
Options allow you to fine-tune how the trigger is matched or displayed.

- `gag`      Gag (don't print) the matched line.
- `raw`      Match on the raw mud line (ANSI escapes intact)
- `prompt`   Match against the prompt instead of regular output lines.
- `count`    Number of times this trigger will match before being removed (default: 0 = infinite)
- `enabled`  Sets the enabled status of the trigger

##

***blight:enable_trigger(id, enabled)***

- `id`         The id of the trigger to enabled/disable
- `enabled`    Boolean toggling the enabled flag on the trigger

##

***blight:remove_trigger(trigger_id)***

- `trigger_id` An id returned when creating the trigger

```lua
local trigger_id = blight:add_trigger(
        "^(\\w+) enters from the \\w+\\.$",
        { gag = true },
        function (matches)
            blight:output("!!! " .. matches[2] .. " entered, lets kick")
            blight:send("kick " .. matches[2])
        end
    )

blight:remove_trigger(trigger_id)

blight:add_trigger("^\\x1b\\[31mHello\\x1b\\[0m$", {raw=true}, function ()
    blight:output(C_BLUE .. "((( Red Hello )))" .. C_RESET)
end)
```
Please note that you can use Lua's long bracket system if you find Lua string escaping tedious when creating regex patterns.
```lua
local trigger_id = blight:add_trigger(
        [=[^(\w+) enters from the \w+\.$]=],
        { gag = false },
        function (matches)
            blight:output("!!! Looks like " .. matches[2] .. " just arrived!")
        end
    )
```

##

***blight:gag()***

This method will gag the next trigger matched line from output. It's best used within a triggers
callback method in order to conditionally gag the output.

```lua
blight:add_trigger("^Health (\\d+)$", {}, function (matches)
    if matches[2] == "100" then
        blight:gag()
    end
end)
```

##

***blight:get_triggers()***

- Returns a list of all triggers currently added by the user in a table.

```lua
-- Return data
{
    id: Trigger
}

-- Trigger object
{
    regex: String,
    enabled: bool,
    gag: bool,
    raw: bool,
    prompt: bool,
    count: number,
}
```

##

***blight:clear_triggers()***

Remove all triggers
