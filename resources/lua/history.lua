local mod = {}

local commands = json.decode(store.disk_read("__command_history") or "[]")
local orig_cmd = ""
local index = nil

function mod.previous_command()
    if not index then
        index = #commands
    else
        index = math.max(index-1, 1)
    end
    prompt.set(commands[index])
    orig_cmd = prompt.get()
end

function mod.next_command()
    if index then
        index = index + 1
        if index > #commands then
            index = nil
        end
    end
    if index then
        prompt.set(commands[index])
    else
        prompt.set(orig_cmd)
    end
end

blight.on_quit(function ()
    store.disk_write("__command_history", json.encode(commands))
end)

mud.add_input_listener(function (line)
    index = nil
    local str = line:line()
    if str ~= commands[#commands] then
        table.insert(commands, str)
    end
    if #commands > 100 then
        table.remove(commands, 1)
    end
    return line
end)

return mod
