# Help

`/help <topic>`               : Get more information on a topic

Available topics:

- `config_scripts`
- `scripting`
- `logging`
- `settings`

Helpfiles can also be viewed [online](https://github.com/LiquidityC/Blightmud/tree/master/resources/help)

## Basic commands:

- `/connect <host> <port>`           : Connect to a given mud server
- `/connect <name>`                  : Connect to a saved server
- `/add_server <name> <host> <port>` : Add a saved server
- `/remove_server <name>`            : Remove a saved server
- `/list_servers, /ls`               : List all saved servers
- `/load <path/to/luafile>`          : Load a script file
- `/lua <code>`                      : Execute Lua code
- `/disconnect`, `/dc`               : Disconnect from server
- `/reconnect`, `/rc`                : Reconnect to last/current server
- `/quit`, `/q`                      : Exit program
- `/help`                            : Help information

## Default keybindings:

- `PgUp`/`PgDn`      : Scroll output view
- `End`              : Go to bottom of output view
- `Up`/`Ctrl-P`      : Previous command
- `Down`/`Ctrl-N`    : Next command
- `Ctrl-A`           : Jump to beginning of input
- `Ctrl-E`           : Jump to end of input
- `Alt-B`            : Step back one word
- `Alt-F`            : Step forward one word
- `Ctrl-K`           : Delete the remainder of the input line from cursor
- `Ctrl-U`           : Delete from start of input line to cursor
- `Ctrl-L`           : Redraw screen (good when muds mess stuff up)
- `Ctrl-C`           : Quit program

To change keybindings see `/help scripting` and `/help bindings`
