require("tests.common")

script.on_reset(function()
    blight.quit()
end)

-- Reproducer for undefined behavior when adding to a trigger group while iterating via pairs
local group = trigger.add_group()

-- Create sparse trigger IDs by creating and discarding dummy triggers in between
-- so that keys in group.triggers are not a dense 1..N array
local triggers = {}
local fired = {}

local iterations = 30

for i = 1, iterations do
    -- Create dummy triggers to advance next_id and create gaps
    for g = 1, 10 do
        trigger.Trigger.new("^dummy$", {}, function() end)
    end
    local tr = group:add("^test_line$", {}, function()
        fired[i] = true
        -- On first trigger execution, dynamically add many new triggers to force a table rehash
        if not fired.added then
            fired.added = true
            for j = 1, 10 do
                group:add("^extra_" .. j .. "$", {}, function() end)
            end
        end
    end)
    triggers[i] = tr
end

-- Send "test_line" to trigger check_line on the group
mud.output("test_line")

timer.add(1, 1, function()
    local total_fired = 0
    for i = 1, iterations do
        if fired[i] then
            total_fired = total_fired + 1
        end
    end
    assert_eq(total_fired, iterations, "All pre-existing triggers in group should have fired")
    script.reset()
end)
