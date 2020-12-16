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
        print("[plugin] Fetching:", path)
        local result, err = plugin.add(path)
        if result then
            print("[plugin] Installed:", path)
        else
            print("[plugin] Failed to install plugin:", err)
        end
    end
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

alias.add("^/update_plugin (.*?)$", function (matches)
    if #matches == 1 or matches[2] == "" then
        print("USAGE: /update_plugin <name>")
    else
        local name = matches[2]
        print("[plugin] Updating: " .. name)
        local result, err = plugin.update(name)
        if result then
            print("[plugin] Updated: " .. name)
        else
            print("[plugin] Failed to update plugin:", err)
        end
    end
end)

alias.add("^/update_plugins$", function ()
    local plugins = plugin.get_all()
    for _,name in ipairs(plugins) do
        print("[plugin] Updating:", name)
        local result, err = plugin.update(name)
        if result then
            print("[plugin] Updated:", name)
        else
            print("[plugin] Failed to update plugin:", err)
        end
    end
end)
