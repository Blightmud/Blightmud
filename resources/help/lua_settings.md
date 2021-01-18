# Settings (Lua)

Allow you to query and change Blightmud settings directly from your Lua scripts.

##

***settings.list()***

Get all current settings as Lua table.

##

***settings.get(key)***

Returns specified setting toggle (boolean)
- `key`    Setting name to get (string)

##

***settings.set(key, value)***

Sets specified setting.
- `key`    Setting name to change (string)
- `value`  New toggle value (boolean)
