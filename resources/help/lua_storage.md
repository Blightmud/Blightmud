# Storage methods

These methods allow you to store your data in session local in-memory storage
or disk storage.  Both kinds of storage are key-value database, where both key
and value are strings. You can use json.encode/json.decode to store more
complex data as your value. Data in session storage survive script resets, and
data in disk storage are permanent between Blightmud restarts.

##

***store.session_write(key, data)***

Writes data to in-memory session storage. Data will survive script resets,
but will be emptied after Blightmud restart.

- `key`     The identifier for the data (string)
- `value`   Content of your data (string)

```lua
local session_data = {target="blob", recal_after_flee=true}
store.session_write("fight_settings", json.encode(session_data))
```

##

***store.session_read(key)***

Returns value for specified key from session in-memory storage.

- `key`     The identifier for the data (same as used before with store.session_write)

```lua
local session_data = json.decode(store.session_read("fight_settings"))
```

##

***store.disk_write(key, data)***

Writes data to settings file (store/data.ron) in your local filesystem. This
data will be permanent between Blightmud restarts.

- `key`     The identifier for the data (string)
- `value`   Content of your data (string)

```lua
local permanent_data = {foes={"Newbian", "CuteKitty", "JackRipper"}, revenge=true}
store.disk_write("pk_settings", json.encode(permanent_data))
```

##

***store.disk_read(key)***

Returns value for specified key from disk storage.

- `key`     The identifier for the data (same as used before with store.disk_write)

```lua
local permanent_data = json.decode(store.disk_read("pk_settings"))
```
