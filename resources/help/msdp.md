# Mud Server Data Protocol (MSDP)

These methods will allow you to communicate with a server that suuports MSDP.
The `msdp` module will store all data it receives and make it accessible to
scripts. You will be able to `register` to these values or just `get` them when
you need them.

MSDP values can be strings, lists or tables.

You can read more about the protocol [here](https://mudhalla.net/tintin/protocols/msdp/):

##

***msdp.list(list)***
Request information on a list that the server supports. When the MSDP handshake
is completed the `msdp` module will automatically request the list
`REPORTABLE_VARIABLES`.

- `list`    The name of the requested list

##

***msdp.send(var)***
Request that the server return the value of the provided variable.

- `var`   The variable (string) or a list of variables (table)

##

***msdp.report(var)***
Request that the server re-send variables when their values change

- `var`   The variable (string) or a list of variables (table)

##

***msdp.unreport(var)***
Request that the server stop reporting on a variable(s)

- `var`   The variable (string) or a list of variables (table)

##

***msdp.set(var val)***
Set a variable on the server, commonly one of the `CONFIGURABLE_VARIABLES`

- `var`   The variable name
- `val`   The variable value

##

***msdp.get(var) -> value***
Get a variable from the `msdp` module. This is not a value fetched directly
from the server. It's a variable that has been previously requested through
`msdp.report()`, `msdp.send()` or `msdp.list()`

- `var`     The variable to fetch
- Returns either a string, or table

##

***msdp.register(var, callback)***
Register for updates on a variable that is being reported (see
`msdp.report()`).

- `var`         The variable name
- `callback`    The callback method, should take a value as an argument

##

***msdp.on_ready(callback)***
Register a callback to be triggered when MSDP is ready between the client and
server

- `callback`    The callback to trigger when ready

