# Scripting

The default scripting language for Blightmud is Lua.

You can manually load your script files with the `/load` command or you can
auto-load them as configuration scripts. 
See `/help config_scripts` for more information on auto-loading scripts.

If you just want to look at some examples of triggers and aliases before
reading docs then check out `/help script_example`.

Alternatively, you can execute Lua directly with the `/lua` command.

In Lua you have access to various modules providing an api against blightmuds
core to interact with your game. You can read the documentation for each module
by typing `/help <module>`.

## The following categories of methods exist:

- `blight`      Module to interact with Blightmud. Print output etc.
- `script`      Module to load and reset lua scripts.
- `alias`       Custom commands that trigger callback functions.
- `trigger`     Functions triggered in response to incoming text.
- `timers`      Functions that execute on a timed delay.
- `regex`       Regular expressions.
- `config`      Functions for interacting with Blightmud settings
- `gmcp`        Functions for interacting with the Generic MUD Communication Protocol.
- `msdp`        Functions for interacting with the Mud Server Data Protocol
- `status_area` Functions for controlling and printing to the status bar
- `storage`     Functions for persisting data between script restarts or between sessions
- `bindings`    Functions for configuring keybindings and adding new ones
- `tasks`       Library for control of background tasks
- `mud`         Functions for interacting with the mud
- `log`         Functions for logging
- `core`        Functions for advanced scripting and telnet protocol control
- `socket`      Functions to handle opening and sending data over a socket
- `audio`       Functions to handle audio
- `history`     Module that handles command history
- `prompt`      Module for interacting with the prompt and it's content
- `prompt_mask` Module for masking/decorating input prompt content.
- `servers`     Server storage and handling
- `spellcheck`  Functions for low-level spellcheck operations.
- `fs`          Filesystem monitoring
- `ttype`       TTYPE negotiation configuration
- `plugin`      Plugin handling
- `json`        Json encoding and decoding
