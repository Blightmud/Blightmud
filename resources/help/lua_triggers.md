# Triggers

Triggers allow Blightmud to execute a callback function in response to output
from the connected server.

## Creating a Trigger
***blight:add_trigger(regex, options, callback)***

- `regex`    The regular expression to match the server output against.
- `options`  A table of options (see `Trigger Options` below)
- `callback` The Lua function that gets called when a match is found.

```lua
blight:add_trigger(
        "^(\\w+) enters from the \\w+\\.$",
        { gag = true },
        function (matches)
            blight:output("!!! " .. match[2] .. " entered, lets kick")
            blight:send("kick " .. match[2])
        end
    )
```

## Trigger Options
Options allow you to fine-tune how the trigger is matched or displayed.

- `gag`      Gag (don't print) the matched line.
- `prompt`   Match against the prompt instead of regular output lines.
