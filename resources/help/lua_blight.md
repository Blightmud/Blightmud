# General Methods

These methods allow you to interface with the connected server or manipulate
Blightmud's scripting engine.

##

***blight:output(str)***
Prints output to the screen.
- `str`  The string to output.
```lua
blight:output("A", "nice", "message")
```

##

***blight:send(str)***
Sends a command to the MUD.
- `str`  The command to send.
```lua
blight:send("kill bat")
```

##

***blight:load(file)***
Loads a script file. You can also use the regular 'require' command for this.
- `file`  The filename of the script to load.

##

***blight:reset()***
Resets the script engine, clearing the entire Lua environment.

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
