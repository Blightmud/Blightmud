# Storage methods

These methods allow you to store data from lua between sessions.

##

***blight:store(key, data)***
Stores table data to disk
- `key`     The identifier for the data
- `data`    A lua table object consisting of string keys and string values. Number values in the table will store fine. But on read they will be string values.

```lua
local data = {
    ["A string key"]="A string value",
    target="dog",
    armor="mail143232"
}
blight:store("player_data", data)
```

##

***blight:read(key) -> Result***
Read data from disk.
- `key`     The identifier for the data (from bligh:store())
- `Result`  A lua table or nil if no data was found

```lua
local player_data = blight:read("player_data") or {}
```
