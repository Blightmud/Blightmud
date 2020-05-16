# Scripting

The default scripting language for blightmud is Lua.

You can load your scriptfile with the /load command.

In lua you have access to the 'blight' object. Methods are available on this
object to interact with your game.

## The following methods exist:
***blight:output(str)***

Prints output to the output screen
Eg. 'blight:output("A", "nice", "message")'
Will print "A nice message" on the screen

***blight:send(str)***

Sends a command to the mud.
Eg. 'blight:send("kill bat")'
Will send the command "kill bat" to the server.

***blight:load(file)***

Loads a script file. You can also use the regular 'require' command
for this.

***blight:reset()***

Resets the script engine clearing the entire lua env.

***blight:add_alias(regex, callback)***

Creates an alias which when triggered runs the provided callback function.
Eg.
```lua
local target = "dog"
blight:add_alias("^kick$", function ()
    blight:send("kick " .. target)
end)
```

***blight:add_trigger(regex, options, callback)***

Creates a trigger that when matched on server output fires the callback 
provided.
This trigger takes a set of options for certain behaviour:
Options are as follows:
```lua
local options = {
    gag = true, -- Gag (don't print) the matched line.
        prompt = true, -- Match prompt instead of regular output lines
}
```
Example:
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

***blight:add_timer(secs, repeat, callback)***

Add a timer that calls the provided callback a set number of times with
the provided duration between each call.

Example:
```lua
local count = 0
blight:add_timer(0.5, 3, function ()
    count = count + 1
    blight:send("say " .. count)
end)
```

### GMCP Handling
Below is the handling for GMCP. These functions are slightly co-dependent so
check out the final large example for a complete instruction how it should be
used.

***blight:on_gmcp_ready(callback)***

Registers a callback that is triggered when the client and server have agreed
to use the GMCP protocol.
You may only register one callback. A secondary callback will
overwrite the first one.

***blight:register_gmcp(module)***

Instructs the server that our client (you) wants to receive updates for
the defined module.
Example: blight:register_gmcp("Room.Info")

***blight:add_gmcp_receiver(module, callback)***

Registers a callback that is executed and provided with the gmcp data when
the specified module data is received from the server.

Example: blight:add_gmcp_receiver("Room.Info", function (data) blight:output(data) end)

The data you receive will be the raw data as a string. The 'json' module is readily available
within the lua state for you to use. You can read about it here: https://github.com/rxi/json.lua

### Complete GMCP example: 

```lua
blight:on_gmcp_ready(function ()
blight:output("Registering GMCP")
blight:register_gmcp("Room")
blight:register_gmcp("Char")
blight:add_gmcp_receiver("Room.Info", function (data)
    obj = json.decode(data)
    blight:output("ROOM NUM: " .. obj["num"])
    blight:output("ROOM MAP: " .. obj["map"])
end)
blight:add_gmcp_receiver("Char.Vitals", function (data)
    blight:output("GMCP: Char.Vitals -> " .. data)
    obj = json.decode(data)
    -- Do stuff with data
end)
blight:add_gmcp_receiver("Char.Status", function (data)
    blight:output("GMCP: Char.Status -> " .. data)
    obj = json.decode(data)
    -- Do stuff with data
end)
end)
```

***blight:on_connect(callback)***

Registers a callback that is triggered when the client successfully connects
to a server.
You may only register one callback. A secondary callback will
overwrite the first one.
