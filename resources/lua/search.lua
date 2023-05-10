local mod = {}

local input_pattern = regex.new("^\x1b.*?m>.*$")
local search_pattern = nil

function mod.search(str)
    search_pattern = regex.new(str)
    blight.find_backward(search_pattern)
end

function mod.find_up()
    if search_pattern then
        blight.find_backward(search_pattern)
    end
end

function mod.find_down()
    if search_pattern then
        blight.find_forward(search_pattern)
    end
end

local function echo_input_enabled()
    if settings.get("echo_input") then
        return true
    end

    blight.output(C_RED .. "[!!] You must enable echo_input with '/set echo_input on' to find by input." .. C_RESET)
    return false
end

function mod.find_last_input()
    if echo_input_enabled() then
        blight.find_backward(input_pattern)
    end
end

function mod.find_next_input()
    if echo_input_enabled() then
        blight.find_forward(input_pattern)
    end
end

return mod
