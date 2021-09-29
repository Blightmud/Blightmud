# Config scripts

Any `.lua` file placed in `$CONFIGDIR` will automatically load on program start. You can use this to automatically load the
right script depending on what mud you connect to or anything else you find useful.

Example config:
```lua
local self = {
	host = store.session_read("cur_host"),
	port = tonumber(store.session_read("cur_port") or "0"),
}

local function reload_scripts()
	blight.status_height(1)
	blight.status_line(0, "")
	script.reset()
	script.load("$CONFIGDIR/config.lua")
end

local function disconnect()
	self.host = nil
	self.port = nil
	store.session_write("cur_host", tostring(nil))
	store.session_write("cur_port", tostring(nil))
	reload_scripts()
end

local function on_connect(host, port)
	store.session_write("cur_host", host)
	store.session_write("cur_port", tostring(port))

	if host == "the-best-mud.org" then
		script.load("~/scripts/the-best-mud/main.lua")
	elseif host == "spacemud.net" then
		script.load("~/scripts/spacemud/main.lua")
	elseif host == "fantasymud.ly" then
		script.load("~/scripts/fantasymud/main.lua")
	end 
end

alias.add("^reload$", reload_scripts)

mud.on_connect(function (host, port)
	on_connect(host, port)
end)

mud.on_disconnect(function ()
	disconnect()
end)

if self.host and self.port then
	on_connect(self.host, self.port)
end
```
