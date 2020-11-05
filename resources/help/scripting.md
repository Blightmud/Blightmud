# Scripting

The default scripting language for Blightmud is Lua.

You can manually load your script files with the `/load` command or you can 
auto-load them as configuration scripts. 
See `/help config_scripts` for more information on auto-loading scripts.

Alternatively, you can execute Lua directly with the `/lua` command.

In Lua you have access to the 'blight' object. Methods are available on this
object to interact with your game. These methods can be broken down into several
categories, show below. You can view the full list of methods for each category
by typing `/help <category>`.

## The following categories of methods exist:
- `blight`      Methods for input, output, and general client and script manipulation.
- `aliases`     Custom commands that trigger callback functions.
- `triggers`    Functions triggered in response to incoming text.
- `timers`      Functions that execute on a timed delay.
- `regex`       Regular expressions.
- `gmcp`        Methods for interacting with the Generic MUD Communication Protocol.
- `msdp`        Methods for interacting with the Mud Server Data Protocol
- `status_area` Methods for controlling and printing to the status bar
- `storage`     Methods for persisting data between sessions
- `bindings`    Methods for configuring keybindings and adding new ones
- `tasks`       Library for control of background tasks
- `core`        Methods for advanced scripting and telnet protocol control
