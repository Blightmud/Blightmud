# The core module

This module allows you to interact with core subsystems within Blightmud as
well as the underlying operating system.

Take note that these functions are here mostly for plugin developers and
advanced users.  Use them with caution. For a general script implementation
they should not be needed.

##

***core:enable_protocol(proto)***
Makes Blightmud respond with `IAC DO PROTO` if the server asks for it.  Take
not that this method is best called before a client actually connects to a mud.
All servers don't play as nice if the client isn't quick to respond to a `IAC
WILL PROTO`.

- `proto`     The protocol u8 identifier

##

***core:on_protocol_enabled(callback)***
A callback to receive updates when protocols are enabled. This will trigger for
all protocols so make sure the one you are interested in is the one supplied.

- `callback`  A callback function that takes a u8 as an argument

```lua
core:on_protocol_enabled(function (proto)
    if proto == 201 then -- Check for GMCP
        -- Do your stuff
    end
end)
```

##

***core:subneg_recv(proto, data)***
Send a subnegotation to the mud.  This will send an `IAC SB proto data IAC SE`
to the mud.

- `proto`     The subnegotiation protocol identifier
- `data`      The bytes you want to send

```lua
core:subneg_recv(function (proto, data)
    if proto == 201 then -- Operator on GMCP
        -- Do stuff with data
    end
end)
```

##

***core:store(key, value)***
Stores a string value in a session local storage. If blightmud restarts the
storage will be emptied.

##

***core:read(key) -> String***
Reads a string value from session storage.

##

***core:exec(shellcommand) -> ExecResponse***
Execute a command on the OS

- `shellcommand` A command to run in the shell
- Returns a Response object containg stdout, stderr and status of the executed
  command. Described below.

```lua
local response = core:exec("curl ipinfo.io/ip")
blight:output("The ip is: " .. response:stdout())
```

***core:ExecResponse***
The object returned from the exec

***ExecResponse:code()***
Returns the exit status of the executed command or nil if it was interrupted by a signal

***ExecResponse:stdout()***
Returns the stdout output of the executed command

***ExecResponse:stderr()***
Returns the stderr output of the executed command
