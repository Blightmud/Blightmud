require("tests.common")

local triggers = trigger.add_group()

script.on_reset(function()
    blight.quit()
end)

-- Track which tagged lines were seen
local tagged_combat = false
local tagged_loot = false
local untagged_seen = false

-- Trigger: tag "combat" lines with a red color and "combat" key
triggers:add("^combat line$", {}, function(_, line)
    line:tag_color("\x1b[31m")
    line:tag_key("combat")
    line:tag_symbol("!")
end)

-- Trigger: tag "loot" lines with a green color and "loot" key
triggers:add("^loot line$", {}, function(_, line)
    line:tag_color("\x1b[32m")
    line:tag_key("loot")
end)

mud.add_output_listener(function(line)
    local key = line:tag_key()
    local color = line:tag_color()
    local symbol = line:tag_symbol()

    if line:line() == "combat line" then
        assert_eq(key, "combat")
        assert_eq(color, "\x1b[31m")
        assert_eq(symbol, "!")
        tagged_combat = true
    elseif line:line() == "loot line" then
        assert_eq(key, "loot")
        assert_eq(color, "\x1b[32m")
        tagged_loot = true
    elseif line:line() == "plain line" then
        assert_eq(key, "")
        assert_eq(color, "")
        untagged_seen = true
    end

    return line
end)

mud.output("combat line")
mud.output("loot line")
mud.output("plain line")

timer.add(1, 1, function()
    assert(tagged_combat, "combat line was not tagged")
    assert(tagged_loot, "loot line was not tagged")
    assert(untagged_seen, "plain line was not seen without tags")

    script.reset()
end)
