# General Methods

These methods allow you to interface with the connected server or manipulate
Blightmud's scripting engine.

##

***blight:output(str)***
Prints output to the screen.
- `str`  The string to output.
```lua
-- Standard printing
blight:output("A", "nice", "message")

-- Color printing
blight:output(C_RED .. "Red message" .. C_RESET);
blight:output(C_BWHITE .. BG_BLUE .. "White text with blue background" .. C_RESET);
```
For a list of available colors se `/help colors`

##

***blight:send(str, options)***
Sends a command to the MUD.
- `str`     The command to send.
- `options` An optional table of options (see `Options` below)
```lua
blight:send("kill bat")
blight:send("password", {gag=true, skip_log=true})
```

**Options**
- `gag`         Gag echoing of what was sent in the client
- `skip_log`    Don't print the send command in the log

##

***blight:load(file)***
Loads a script file. You can also use the regular 'require' command for this.
- `file`  The filename of the script to load.

##

***blight:reset()***
Resets the script engine, clearing the entire Lua environment.

##

***blight:terminal_dimensions() -> width, height***
Gets the current terminal dimensions (these can change on window resize).
```lua
width, height = blight:terminal_dimensions()
```

##

***blight:on_connect(callback)***
Registers a callback that is triggered when the client successfully connects to
a server. You may only register one callback. A secondary callback will overwrite
the first one.
The callback function may take two arguments: `host` and `port`.
- `callback`   A Lua function to be called upon connection.
```lua
blight:on_connect(function (host, port)
    blight:output("Connected to:", host, port)
end)
```

##

***blight:on_disconnect(callback)***
Registers a callback that is triggered upon disconnecting from a server. You
may only register one callback. Subsequent calls to this method will overwrite
previously registered callbacks.
```lua
blight:on_disconnect(function ()
    blight:output("Disconnected from server")
end)
```

##

***blight:start_logging(worldname)***
Start logging to a specified "world" name.

If a log is already started then this command has no effect. So if you choose to use this manual logging then make
sure automatic logging is disabled. See `/help logging` for more information.

##

***blight:stop_logging()***
Stop logging

