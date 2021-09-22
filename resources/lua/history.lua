local mod = {}

local commands = {}
local orig_cmd = nil
local index = nil

local search_index = nil
local search_commands = nil

if settings.get("save_history") then
    commands = json.decode(store.disk_read("__command_history") or "[]")
end

local function reset()
    index = nil
    orig_cmd = nil
    search_commands = nil
    search_index = nil
end

local function startsWith(src, pat)
    return src:find(pat, 1) == 1
end

local function find_match_up()
    if not search_commands then
        local command_set = {[orig_cmd]=true}
        search_commands = {}
        for i,c in ipairs(commands) do
            if startsWith(c, orig_cmd) then
                if not command_set[c] then
                    table.insert(search_commands, i)
                end
                command_set[c] = true
            end
        end
        search_index = #search_commands
    else
        search_index = math.max(search_index - 1, 1)
    end

    if search_index > 0 then
        return search_commands[search_index]
    else
        return 0
    end
end

local function find_match_down()
    if search_index then
        search_index = search_index + 1
        if search_index > #search_commands then
            return nil
        else
            return search_commands[search_index]
        end
    end
    return nil
end

function mod.previous_command()
    if not orig_cmd then
        orig_cmd = prompt.get()
    end
    if orig_cmd == "" or not settings.get("command_search") then
        if not index then
            index = #commands
        else
            index = math.max(index-1, 1)
        end
    else
        index = find_match_up()
    end
    if index > 0 then
        prompt.set(commands[index])
    else
        reset()
    end
end

function mod.next_command()
    if orig_cmd then
        if orig_cmd == "" or not settings.get("command_search") then
            if index then
                index = index + 1
                if index > #commands then
                    index = nil
                end
            end
        else
            index = find_match_down()
        end
        if index then
            prompt.set(commands[index])
        else
            prompt.set(orig_cmd)
            reset()
        end
    end
end

blight.on_quit(function ()
    if settings.get("save_history") then
        store.disk_write("__command_history", json.encode(commands))
    end
end)

mud.add_input_listener(function (line)
    reset()
    local str = line:line()
    if str ~= commands[#commands] and #str > 0 then
        table.insert(commands, str)
    end
    if #commands > 100 then
        table.remove(commands, 1)
    end
    return line
end)

return mod
