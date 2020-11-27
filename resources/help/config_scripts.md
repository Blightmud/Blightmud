# Config scripts

Any `.lua` file placed in `$CONFIGDIR` will automatically load on program start. You can use this to automatically load the
right script depending on what mud you connect to or anything else you find useful.

Example config:
```lua
local self = {
	host = core:read("cur_host"),
	port = tonumber(core:read("cur_port") or "0"),
}

local function reload_scripts()
	blight:status_height(1)
	blight:status_line(0, "")
	script.reset()
	script.load("/home/user/.config/blightmud/config.lua")
end

local function disconnect()
	blight:output("[CONFIG]: Clearing scripts")
	self.host = nil
	self.port = nil
	core:store("cur_host", tostring(nil))
	core:store("cur_port", tostring(nil))
	reload_scripts()
end

local function on_connect(host, port)
	core:store("cur_host", host)
	core:store("cur_port", tostring(port))

	if host == "the-best-mud.org" then
		blight:load("~/scripts/the-best-mud/main.lua")
	elseif host == "spacemud.net" then
		blight:load("~/scripts/spacemud/main.lua")
	elseif host == "fantasymud.ly" then
		blight:load("~/scripts/fantasymud/main.lua")
	end 
end

alias.add("^reload$", reload_scripts)

mud.on_connect(function (host, port)
	on_connect(host, port)
end)

mud.on_disconnect(function ()
	blight:output("[CONFIG]: Disconnecting")
	disconnect()
end)

if self.host and self.port then
	on_connect(self.host, self.port)
end
```
