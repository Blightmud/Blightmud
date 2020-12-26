# The script module

Exposes methods to load and reset lua code in blightmud.

##

***script.load(file)***
Loads a script file. You can also use the regular `require` command for this.

- `file`  The filename of the script to load.
 
When loading a file the Lua `package.path` will be prepended with the current
dir of this script. Eg. If you
`script.load("/home/user/scripts/mud/script.lua")` then package path will be
prepended with `"/home/user/scripts/mud/?.lua" .. package.path`.  So within
`script.lua` file you can require other lua files as needed without worrying
about the package path.

Eg.
```lua
require("module") -- To include "module.lua" from the same dir
require("submodule.module") -- To include "submodule/module.lua" from the same dir.
```

The prepended path will be removed from the lua state once `script.load`
completes. So you don't need to worry about conflicts with future loads and
paths.

##

***script.reset()***
Resets the script engine, clearing the entire Lua environment.

## Tips and tricks

- Try to create one *main* lua script which you load using `script.load()`.
  Additional files can be included through this file using `require`.
- In order to make `require` look in the right places you can modify
  `package.path`.
  ```lua
  -- Example
  package.path = "/path/to/mud/script/folder/?.lua;" .. package.path
  ```
- Leverage `mud.on_connect()` and `mud.on_disconnect()` to load the right
  scripts for the right mud and to `script.reset()` when disconnecting
  (switching).
