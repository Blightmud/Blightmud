# Mud

This module allows you to interact with the active mud. Such as capturing lines
and sending input and output.

##

***mud.send(str, options)***
Sends a command to the MUD.

- `str`     The command to send.
- `options` An optional table of options (see `Options` below)

**Options**
- `gag`         Gag echoing of what was sent in the client
- `skip_log`    Don't print the send command in the log

##

***mud.send_bytes(bytes)***
Sends bytes to the MUD

- `bytes`       A list of bytes to send

##

***mud.output(str)***
Sends a line of text as if it was received from the mud. This can be useful to
test triggers etc.

##

***mud.input(str)***
Sends a line to the client as if it was typed at the prompt (this will trigger
aliases).

##

***mud.connect(host, port[, tls, verify])***
Connect to a server

- `host`   The host
- `port`   The port
- `tls`    Tls connection? true/false *(optional)*
- `verify` Verify tls cert (default: true) *(optional)*

##

***mud.on_connect(callback)***
Registers a callback that is triggered when the client successfully connects to
a server.

- `callback`   A Lua function to be called upon connection. (host, port)

```lua
blight.on_connect(function (host, port)
    blight.output("Connected to:", host, port)
end)
```

##

***mud.disconnect()***
Disconnect from the current mud

##

***mud.on_disconnect(callback)***
Registers a callback that is triggered upon disconnecting from a server.

- `callback`   A Lua function to be called upon disconnect.

```lua
blight.on_disconnect(function ()
    blight.output("Disconnected from server")
end)
```

##

***mud.reconnect()***
Reconnect to the current/last connected server

##

***mud.add_output_listener(callback)***

This method will add a listener for mud output. All lines received from the mud
will be provided to the registered callback for processing. This is one of the
core systems in blightmud and what's being used for the trigger system to
operate. For a general user this method should not be needed.

The provided callback will receive one argument. A line object. See `/help
line` for information about this object.

The provided line object must be returned at the end of the callback otherwise
modifications to the line will not be accounted for in later processing.

##

***mud.add_input_listener(callback)***

This method will add a listener for user input to the mud. All input lines from
the user will be sent to this callback.  This is one of the core systems in
blightmud and what's being used for the alias system to operate. For a general
user this method should not be needed.

The provided callback will receive one argument. A line object. See `/help
line` for information about this object.

The provided line object must be returned at the end of the callback otherwise
modifications to the line will not be accounted for in later processing.

##

***mud.add_tag(tag)***
Adds a tag for the current mud in the topbar of Blightmud after the hostname.

- `tag` The tag you want to add

```lua
mud.add_tag("GMCP")
mud.add_tag("MSDP")
-- Will display "somemud.org:4000 [GMCP][MSDP]" in the top bar of Blightmud
```

The primary use for this function is to indicate what telnet protocols are
active for the current mud.
