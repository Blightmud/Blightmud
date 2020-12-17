local function auto_load_plugins()
    local plugins = plugin.enabled()
    for _,p in ipairs(plugins) do
        local ok, err = plugin.load(p)
        if ok then
            print("[plugin]: Loaded '", p, "'")
        elseif err then
            print("[plugin]: Failed to load plugin,", p, ":", err)
        end
    end
end

auto_load_plugins()
