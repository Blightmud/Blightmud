# Servers

This module allows you to persist frequently used servers in Blightmud.

##

***servers.add(name, host, port[, tls, verify])***
Saves a server to disk. If a server with the provided name already exists
the call will error. You can catch the error using `pcall()`

- `name`    A name for the server
- `host`    The server host
- `port`    The server port
- `tls`     Is the connection TLS, boolean *(optional)*
- `verify`  Verify the tls cert, boolean (default: true) *(optional)*

##

***servers.remove(name)***
Removes named server from storage. Will error if the server doesn't exist.

- `name`    The name of the server to remove

##

***servers.get(name) -> Server***
Returns a `Server` for the named server.

- `name`    The name of the server to get
- Returns a `Server`

##

***servers.get_all() -> [Server]***
Returns a list of all stored servers

##

# Server
An object describing a server. It contains the following data:

```lua
Server = {
    name="The name",
    host="The host",
    port=4000,
    tls=false,
    verify_cert=true
}
```
