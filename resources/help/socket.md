# Socket

This module allows you to open a socket and send data through it.  In it's
current state you can only interact one direction through the socket so there
is currently no handling for receiving data.  The main use-case when it was
built was to send and display mud information in separate terminal windows by
sending data to an echoing server.

##

***socket.connect(host, port)***
Connect to a host and port.

- `host`    The host to connect to (eg. "localhost")
- `port`    The port to connect to
- Returns a socket object or nil if connection failed.

##

***Socket:send(msg)***
Send a string over the socket

- `msg`     The string to send

##

***Socket:close()***
Closes the socket connection

##

Example:
```lua
-- Using netcat you can setup a server like this: `nc -lkp 1234`
local conn = socket.connect("localhost", 1234)
if conn then
    conn:send(cformat("<red>A message<reset>\n")) -- Print some red text
    conn:send("\x1b[2J\x1b[1;1H") -- Clear the screen and reset cursor to top right
    conn:close()
end
```
