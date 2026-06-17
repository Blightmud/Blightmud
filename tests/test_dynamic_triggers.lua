require "tests.common"

local triggers = trigger.add_group()

script.on_reset(function ()
    blight.quit()
end)

local first_triggered = 0
local second_triggered = 0
local third_triggered = 0

local third_trigger



triggers:add("^first$", {}, function()
    print("First triggered")
    first_triggered = first_triggered + 1
    -- This addition triggered the bug before
    triggers:add("^second$", {}, function()
        print("Second triggered")
        second_triggered = second_triggered + 1
    end)
    third_trigger = triggers:add("^third", {}, function()
        print("Third triggered")
        third_triggered = third_triggered + 1
        local id = third_trigger.id
        if id then
            print(id)
            trigger.remove(third_trigger.id)
        end
    end)
end)


mud.output("first")
mud.output("second")
mud.output("third")
mud.output("fourth")

timer.add(1, 1, function ()
    assert(first_triggered == 1, "Round 1: First trigger should have triggered")
    assert(second_triggered == 1, "Round 1: Second trigger should have triggered")
    assert(third_triggered == 1, "Round 1: Third trigger should have triggered")

    first_triggered = 0
    second_triggered = 0
    third_triggered = 0
    mud.output("Second round of triggers:")
    mud.output("first")
    mud.output("second")
    mud.output("third")
end)

timer.add(2, 1, function ()
    assert(first_triggered == 1, "Round 2: First trigger should have triggered")
    assert(second_triggered == 2, "Round 2: Second trigger should have triggered")
    assert(third_triggered == 1, "Round 2: Third trigger should have triggered")
    script.reset()
end)
