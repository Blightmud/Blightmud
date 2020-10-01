# Generic MUD Communication Protocol (GMCP)

These methods allow Blightmud to send and receive GMCP data. They are slightly
co-dependent, so a complete example is provided at the end to demonstrate
how they all work together.

##

***gmcp.on_ready(callback)***
Registers a callback that is triggered when the client and server have agreed
to use the GMCP protocol.
You may only register one callback. A secondary callback will
overwrite the first one.

- `callback`   The Lua function that gets triggered.

##

***gmcp.register(module)***
Instructs the server that our client (you) wants to receive updates for
the defined module.

- `module`  The name of the GMCP module to receive updates for.

```lua
gmcp.register("Room.Info")
```

##

***gmcp.receive(module, callback)***
Registers a callback that is executed and provided with the GMCP data when
the specified module data is received from the server. The data you receive
will be the raw data as a string. The 'json' module is readily available
within the Lua state for you to use: https://github.com/rxi/json.lua

- `module`   The name of the GMCP module to register.
- `callback` The Lua function that will receive <module> updates.

```lua
gmcp.receive("Room.Info", function (data) blight:output(data) end)
```

##

***gmcp.send(msg)***
Sends the provided msg string as GMCP to the MUD.

- `msg`   The string to send.

```lua
data = { char = { hp = "1234" } }
gmcp.send_gmcp("Char.Health " .. json.encode(data))
```

## Complete GMCP example: 

```lua
gmcp.on_ready(function ()
    blight:output("Registering GMCP")
    gmcp.register("Room")
    gmcp.register("Char")
    gmcp.receive("Room.Info", function (data)
        obj = json.decode(data)
        blight:output("ROOM NUM: " .. obj["num"])
        blight:output("ROOM MAP: " .. obj["map"])
    end)
    gmcp.receive("Char.Vitals", function (data)
        blight:output("GMCP: Char.Vitals -> " .. data)
        obj = json.decode(data)
        -- Do stuff with data
    end)
    gmcp.receive("Char.Status", function (data)
        blight:output("GMCP: Char.Status -> " .. data)
        obj = json.decode(data)
        -- Do stuff with data
    end)
end)
```

