local function get_args(cmd)
	local args={}
	for str in string.gmatch(cmd, "([^%s]+)") do
		table.insert(args, str)
	end
	return args
end

alias.add("^/plugins$", function ()
    local plugins = plugin.get_all()
    for _,name in ipairs(plugins) do
        print("[plugin]:", name)
    end
end)

alias.add("^/add_plugin.*$", function (m)
    local args = get_args(m[1])
    if #args == 1 then
        print("USAGE: /add_plugin <url|path>")
    else
        local path = args[2]
        plugin.add(path, true)
    end
end)


alias.add("^/enable_plugin.*$", function (m)
    local args = get_args(m[1])
    if #args == 1 then
        print("USAGE: /enable_plugin <plugin_name>")
    else
        plugin.enable(m[2])
    end
end)

alias.add("^/disable_plugin (.*)$", function (m)
    local args = get_args(m[1])
    if #args == 1 then
        print("USAGE: /disable_plugin <plugin_name>")
    else
        plugin.disable(m[2])
    end
end)

alias.add("^/load_plugin.*$", function (m)
    local args = get_args(m[1])
    if #args == 1 then
        print("USAGE: /load_plugin <plugin_name>")
    else
        local name = args[2]
        print("[plugin] Loading: " .. name)
        local result, err = plugin.load(name)
        if not result then
            print("[plugin] Failed to load plugin:", err)
        end
    end
end)

alias.add("^/remove_plugin.*$", function (m)
    local args = get_args(m[1])
    if #args == 1 then
        print("USAGE: /remove_plugin <plugin_name>")
    else
        local name = args[2]
        print("[plugin] Removing: " .. name)
        local result, err = plugin.remove(name)
        if result then
            print("[plugin] Removed: " .. name)
        else
            print("[plugin] Failed to remove plugin:", err)
        end
    end
end)

alias.add("^/update_plugins$", function ()
    local plugins = plugin.get_all()
    for _,name in ipairs(plugins) do
        plugin.update(name)
    end
end)
