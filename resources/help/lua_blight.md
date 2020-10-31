# General Methods

These methods allow you to interface with the connected server or manipulate
Blightmud's scripting engine.

##

***blight:output(str)***
Prints output to the screen. Also aliased as `print()`.
- `str`  The string to output.
```lua
-- Standard printing
blight:output("A", "nice", "message")

-- Color printing
blight:output(C_RED .. "Red message" .. C_RESET);
blight:output(C_BWHITE .. BG_BLUE .. "White text with blue background" .. C_RESET);

-- Lua's print()
print("Another", BG_BLUE .. "nice" .. C_RESET, "message")
```
For a list of available colors se `/help colors`

##

***blight:mud_output(str)***
Sends a line of text as if it was received from the mud. This can be useful to test triggers etc.
- `str`  The string to output.
```lua
blight:mud_output("A trigger line to test")
```

##

***blight:user_input(str)***
Sends a line to the client as if it was typed at the prompt (this will trigger aliases).
- `str`  The string to output.
```lua
blight:user_input("An alias line to trigger")
```

##

***blight:send(str, options)***
Sends a command to the MUD.
- `str`     The command to send.
- `options` An optional table of options (see `Options` below)

**Options**
- `gag`         Gag echoing of what was sent in the client
- `skip_log`    Don't print the send command in the log

***blight:send_bytes(bytes)***
Sends bytes to the MUD
- `bytes`       A list of bytes to send

```lua
blight:send("kill bat")
blight:send("password", {gag=true, skip_log=true})
blight:send_bytes({ 0xff, 0xf1 })
```

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

***blight:connect(host, port[, tls])***
Connect to a server

- `host`  The host
- `port`  The port
- `tls`   Tls connection? true/false

##

***blight:on_connect(callback)***
Registers a callback that is triggered when the client successfully connects to
a server.

- `callback`   A Lua function to be called upon connection. (host, port)
-
```lua
blight:on_connect(function (host, port)
    blight:output("Connected to:", host, port)
end)
```

##

***blight:on_disconnect(callback)***
Registers a callback that is triggered upon disconnecting from a server.

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

