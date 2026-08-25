require("tests.common")

script.on_reset(function()
    blight.quit()
end)

-- Test spawning tasks while iterating over active tasks in tasks.lua.
local tick_count = 0
timer.on_tick(function()
    tick_count = tick_count + 1
end)

local initial_count = 30
local executed_on_first_tick = 0
local tick_executions = {}
local initial_task_tick

for i = 1, initial_count do
    tasks.spawn(function()
        if initial_task_tick == nil then
            initial_task_tick = tick_count
        end
        if tick_count == initial_task_tick then
            executed_on_first_tick = executed_on_first_tick + 1
            for j = 1, 100 do
                tasks.spawn(function() end)
            end
        end
        tick_executions[tick_count] = (tick_executions[tick_count] or 0) + 1
    end)
end

timer.add(1, 1, function()
    for i = 0, 10 do
        mud.output(string.format("%s: %s", i, tick_executions[i] or 0))
    end
    assert_eq(executed_on_first_tick, initial_count, "All initial tasks should have executed on the first tick")
    script.reset()
end)
