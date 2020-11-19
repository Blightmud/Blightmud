# General Methods

These methods allow you to interface with the connected server or manipulate
Blightmud's scripting engine.

##

***blight:output(str...)***
Prints output to the screen. Also aliased as `print()`.

- `str...`  The string(s) to output.
 
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

***blight:start_logging(worldname)***
Start logging to a specified "world" name.

If a log is already started then this command has no effect. So if you choose to use this manual logging then make
sure automatic logging is disabled. See `/help logging` for more information.

##

***blight:stop_logging()***
Stop logging

