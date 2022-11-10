local mod = {}

local commands = {}
local orig_cmd = nil
local index = nil
local command_set = {}

local search_index = nil
local search_commands = nil

if settings.get("save_history") then
    commands = json.decode(store.disk_read("__command_history") or "[]")
    for _,c in ipairs(commands) do
        command_set[c] = true
    end
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
        local command_subset = {[orig_cmd]=true}
        search_commands = {}
        for i,c in ipairs(commands) do
            if startsWith(c, orig_cmd) then
                if not command_subset[c] then
                    table.insert(search_commands, i)
                end
                command_subset[c] = true
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
        if tts.is_available() then
            tts.speak(commands[index], true)
        end
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
            if tts.is_available() then
                tts.speak(commands[index], true)
            end
            prompt.set(commands[index])
        else
            if tts.is_available() then
                tts.speak(orig_cmd, true)
            end
            prompt.set(orig_cmd)
            reset()
        end
    end
end

local function write_to_disk()
    if settings.get("save_history") then
        store.disk_write("__command_history", json.encode(commands))
    end
end

blight.on_quit(write_to_disk)
mud.on_disconnect(write_to_disk)
script.on_reset(write_to_disk)

local function shift_commands(new_cmd)
    if command_set[new_cmd] then
        for i,cmd in ipairs(commands) do
            if cmd == new_cmd then
                table.remove(commands, i)
            end
        end
    end
    table.insert(commands, new_cmd)
end

mud.add_input_listener(function (line)
    reset()
    if line:source() == "user" then
        local str = line:line()
        if str ~= commands[#commands] and #str > 0 then
            if settings.get("smart_history") then
                shift_commands(str)
            else
                table.insert(commands, str)
            end
            command_set[str] = true
        end
        if #commands > 100 then
            table.remove(commands, 1)
        end
    end
    return line
end)

return mod
