require "tests.common"

local aliases = alias.add_group()
local triggers = trigger.add_group()

script.on_reset(function ()
    blight.quit()
end)

local trigger_triggered = false
triggers:add("^trigger$", {}, function ()
    print("Trigger triggered")
    trigger_triggered = true
end)

local alias_triggered = false
aliases:add("^alias$", function ()
    print("Alias triggered")
    alias_triggered = true
end)

mud.input("/triggers")
mud.input("/aliases")

mud.input("alias")
mud.output("trigger")

timer.add(1, 1, function ()
    assert(alias_triggered)
    assert(trigger_triggered)

    script.reset()
end)

