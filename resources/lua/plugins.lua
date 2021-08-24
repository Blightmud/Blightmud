alias.add("^/plugins$", function ()
    local plugins = plugin.get_all()
    for _,name in ipairs(plugins) do
        print("[plugin]:", name)
    end
end)

alias.add("^/add_plugin (.*?)$", function (matches)
    if #matches == 1 or matches[2] == "" then
        print("USAGE: /add_plugin <url|path>")
    else
        local path = matches[2]
        plugin.add(path, true)
    end
end)


alias.add("^/enable_plugin (.*)$", function (matches)
    plugin.enable(matches[2])
end)

alias.add("^/disable_plugin (.*)$", function (matches)
    plugin.disable(matches[2])
end)

alias.add("^/load_plugin (.*?)$", function (matches)
    if #matches == 1 or matches[2] == "" then
        print("USAGE: /load_plugin <name>")
    else
        local name = matches[2]
        print("[plugin] Loading: " .. name)
        local result, err = plugin.load(name)
        if not result then
            print("[plugin] Failed to load plugin:", err)
        end
    end
end)

alias.add("^/remove_plugin (.*?)$", function (matches)
    local name = matches[2]
    print("[plugin] Removing: " .. name)
    local result, err = plugin.remove(name)
    if result then
        print("[plugin] Removed: " .. name)
    else
        print("[plugin] Failed to remove plugin:", err)
    end
end)

alias.add("^/update_plugins$", function ()
    local plugins = plugin.get_all()
    for _,name in ipairs(plugins) do
        plugin.update(name)
    end
end)
