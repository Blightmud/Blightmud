# The blight module

Exposes methods to print to blightmuds output area and bind keys.

##

***blight.output(str...)***
Prints output to the screen. Also aliased as `print()`.

- `str...`  The string(s) to output.
 
```lua
-- Standard printing
blight.output("A", "nice", "message")

-- Color printing
blight.output(C_RED .. "Red message" .. C_RESET);
blight.output(C_BWHITE .. BG_BLUE .. "White text with blue background" .. C_RESET);

-- Lua's print()
print("Another", BG_BLUE .. "nice" .. C_RESET, "message")
```
For a list of available colors se `/help colors`

##

***blight.terminal_dimensions() -> width, height***
Gets the current terminal dimensions (these can change on window resize).
```lua
width, height = blight.terminal_dimensions()
```

##

***blight.version() -> name, version***
Returns Blightmud name and version in string format

##

***blight.config_dir() -> Path***
Returns blightmuds config directory path on the current system

##

***blight.data_dir() -> Path***
Returns blightmuds data directory path on the current system

##

***blight.show_help(subject, lock_scroll)***
Render a helpfile

- `subject`     The name of the helpfile to show
- `lock_scroll` Lock scroll to top of the helpfile

##

***blight.find_backward(regex)***
Searches for a string backward from current position

- `regex`    The `regex` to search for

##

***blight.find_forward(regex)***
Searches for a string forward from current position

- `regex`    The `regex` to search for

##

***blight.quit()***
Exit Blightmud

##

***blight.on_quit(callback)***
Registers a function to be called when blightmud exits

- `callback`    The function to be called

##
