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

***blight.on_complete(callback: function(input: string) -> [string] | nil)***
Allows users to insert custom tab completion logic into Blightmud

- `callback`    The function that gets called on a `complete` event. 

The callback function should return a list of completions or `nil` if no
completions were found. The completions should be complete words (prefix
included) and not just the suffix or completion part of the word.

You may register multiple completion callbacks, the execution order of these is
undefined however the default completion inside Blightmud will always be
executed after custom completion functions.

All completions from custom functions and default completions will be
concatenated into a list with duplicates removed (order preserved). Subsequent
completion calls (default `tab` presses) will step through this list.

### Example:
1. User types: `bat<tab>`
2. Completion functions are called returning `[batman, batgirl]`
3. `batman` is inserted into users prompt
4. User types: `<tab>`
5. `batgirl` is inserted into users prompt
6. User types: `<tab>`
7. `bat` is inserted into users prompt (back to start)
8. User types `g<tab>` (prompt is `batg`, completions are cleared)
9. Completion functions are called returning `[batgirl]`

##

***blight.quit()***
Exit Blightmud

##

***blight.on_quit(callback)***
Registers a function to be called when blightmud exits

- `callback`    The function to be called

##
