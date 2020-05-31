# Triggers

Triggers allow Blightmud to execute a callback function in response to output
from the connected server.

## Creating a Trigger
***blight:add_trigger(regex, options, callback) -> id***

- `regex`    The regular expression to match the server output against.
- `options`  A table of options (see `Trigger Options` below)
- `callback` The Lua function that gets called when a match is found.
- Returns a trigger id (used for removing the trigger)

***blight:remove_trigger(trigger_id)***

- `trigger_id` An id returned when creating the trigger

```lua
local trigger_id = blight:add_trigger(
        "^(\\w+) enters from the \\w+\\.$",
        { gag = true },
        function (matches)
            blight:output("!!! " .. match[2] .. " entered, lets kick")
            blight:send("kick " .. match[2])
        end
    )

blight:remove_trigger(trigger_id)
```

## Trigger Options
Options allow you to fine-tune how the trigger is matched or displayed.

- `gag`      Gag (don't print) the matched line.
- `prompt`   Match against the prompt instead of regular output lines.
