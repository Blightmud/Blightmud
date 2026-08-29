require("tests.common")

script.on_reset(function()
    blight.quit()
end)

local dummy_tr = trigger.Trigger.new("^dummy$", {}, function() end)
assert(trigger.Trigger.is_trigger(dummy_tr), "Trigger.new should return a valid Trigger")
assert(not trigger.Trigger.is_trigger({}), "Plain table is not a Trigger")
assert(not trigger.Trigger.is_trigger(123), "Number is not a Trigger")
assert(not trigger.Trigger.is_trigger("string"), "String is not a Trigger")
assert(not trigger.Trigger.is_trigger(nil), "Nil is not a Trigger")
assert(not trigger.Trigger.is_trigger(function() end), "Function is not a Trigger")
assert(not trigger.Trigger.is_trigger(true), "Boolean is not a Trigger")

local tr_default = trigger.Trigger.new("^def$", {}, function() end)
assert_eq(tr_default.gag, false, "default gag should be false")
assert_eq(tr_default.raw, false, "default raw should be false")
assert_eq(tr_default.prompt, false, "default prompt should be false")
assert_eq(tr_default.count, nil, "default count should be nil")
assert_eq(tr_default.enabled, true, "default enabled should be true")

local tr_custom = trigger.Trigger.new("^cust$", {
    gag = true,
    raw = true,
    prompt = true,
    count = 5,
    enabled = false,
}, function() end)
assert_eq(tr_custom.gag, true, "custom gag should be true")
assert_eq(tr_custom.raw, true, "custom raw should be true")
assert_eq(tr_custom.prompt, true, "custom prompt should be true")
assert_eq(tr_custom.count, 5, "custom count should be 5")
assert_eq(tr_custom.enabled, false, "custom enabled should be false")

local re_obj = regex.new("^re_obj$")
local tr_re = trigger.Trigger.new(re_obj, {}, function() end)
assert_eq(tr_re.regex, re_obj, "Trigger should accept pre-compiled regex object")

assert(tr_custom.id > tr_default.id, "Trigger IDs should increment")

local tr_toggle = trigger.Trigger.new("^toggle$", {}, function() end)
assert_eq(tr_toggle:is_enabled(), true, "New trigger should be enabled")
tr_toggle:disable()
assert_eq(tr_toggle:is_enabled(), false, "disable() should disable trigger")
tr_toggle:enable()
assert_eq(tr_toggle:is_enabled(), true, "enable() should enable trigger")
tr_toggle:set_enabled(false)
assert_eq(tr_toggle:is_enabled(), false, "set_enabled(false) should disable trigger")
tr_toggle:set_enabled(true)
assert_eq(tr_toggle:is_enabled(), true, "set_enabled(true) should enable trigger")

local function create_mock_line(text, raw_text, is_prompt)
    local mock = {
        _line = text or "test line",
        _raw = raw_text or text or "test line",
        _prompt = is_prompt or false,
        _gagged = false,
        _matched = false,
    }
    function mock:line() return self._line end
    function mock:raw() return self._raw end
    function mock:prompt() return self._prompt end
    function mock:gag(val) if val ~= nil then self._gagged = val end return self._gagged end
    function mock:matched(val) if val ~= nil then self._matched = val end return self._matched end
    return mock
end

local prompt_cb_fired = false
local tr_prompt = trigger.Trigger.new("^prompt line$", { prompt = true }, function()
    prompt_cb_fired = true
end)

local mock_non_prompt = create_mock_line("prompt line", nil, false)
tr_prompt:check_line(mock_non_prompt)
assert(not prompt_cb_fired, "Prompt trigger should not match non-prompt line")
assert(not mock_non_prompt:matched(), "Line should not be matched when prompt setting differs")

local mock_prompt = create_mock_line("prompt line", nil, true)
tr_prompt:check_line(mock_prompt)
assert(prompt_cb_fired, "Prompt trigger should match prompt line")
assert(mock_prompt:matched(), "Line should be marked as matched")

local raw_cb_fired = false
local tr_raw = trigger.Trigger.new("\x1b\\[31mcolor\x1b\\[0m", { raw = true }, function()
    raw_cb_fired = true
end)

local mock_raw_line = create_mock_line("color", "\x1b[31mcolor\x1b[0m", false)
tr_raw:check_line(mock_raw_line)
assert(raw_cb_fired, "Raw trigger should match raw line text")
assert(mock_raw_line:matched(), "Raw line should be marked matched")

local tg = trigger.TriggerGroup.new(99)
assert_eq(tg.id, 99, "TriggerGroup id should match constructor arg")
assert_eq(tg:is_enabled(), true, "New TriggerGroup should be enabled")

local added_tr1 = tg:add("^added1$", {}, function() end)
assert(trigger.Trigger.is_trigger(added_tr1), "TriggerGroup:add should create and return Trigger")
assert_eq(tg:get(added_tr1.id), added_tr1, "TriggerGroup:get should find added trigger by ID")

local standalone_tr = trigger.Trigger.new("^standalone$", {}, function() end)
local added_tr2 = tg:add(standalone_tr)
assert_eq(added_tr2, standalone_tr, "TriggerGroup:add should accept existing Trigger object")
assert_eq(tg:get(standalone_tr.id), standalone_tr, "TriggerGroup:get should find standalone trigger")

local all_tg_triggers = tg:get_triggers()
assert_eq(all_tg_triggers[added_tr1.id], added_tr1)
assert_eq(all_tg_triggers[standalone_tr.id], standalone_tr)

tg:remove(added_tr1.id)
assert_eq(tg:get(added_tr1.id), nil, "TriggerGroup:remove should remove trigger")
tg:clear()
assert_eq(tg:get(standalone_tr.id), nil, "TriggerGroup:clear should remove all triggers")

assert_eq(tg:is_enabled(), true)
tg:disable()
assert_eq(tg:is_enabled(), false, "TriggerGroup:disable should disable group")
tg:enable()
assert_eq(tg:is_enabled(), true, "TriggerGroup:enable should enable group")
tg:set_enabled(false)
assert_eq(tg:is_enabled(), false, "TriggerGroup:set_enabled should set group enable flag")
tg:set_enabled(true)

local group1 = trigger.get_group(1)
assert(group1 ~= nil, "Group 1 should exist by default")
assert_eq(trigger.get_group(), group1, "Default group ID for get_group should be 1")

local mod_tr = trigger.add("^mod_tr$", {}, function() end)
assert(trigger.Trigger.is_trigger(mod_tr), "trigger.add should return a Trigger")
assert_eq(trigger.get(mod_tr.id), mod_tr, "trigger.get should find trigger across groups")

local new_group = trigger.add_group()
assert(new_group ~= nil, "trigger.add_group should return a new TriggerGroup")
assert(new_group.id > 1, "New group ID should be > 1")
assert_eq(trigger.get_group(new_group.id), new_group, "trigger.get_group should find new group")

local ng_tr = new_group:add("^ng_tr$", {}, function() end)
assert_eq(trigger.get(ng_tr.id), ng_tr, "trigger.get should find triggers in custom groups")

trigger.remove(ng_tr.id)
assert_eq(trigger.get(ng_tr.id), nil, "trigger.remove should remove trigger by ID")

trigger.clear()
assert_eq(trigger.get(mod_tr.id), nil, "trigger.clear should clear triggers")

local test_flags = {
    regex_matched = false,
    captured_hp = nil,
    captured_max_hp = nil,
    disabled_fired = false,
    gagged_fired = false,
    count_fires = 0,
    group_disabled_fired = false,
}

trigger.add("^HP: (\\d+)/(\\d+)$", {}, function(matches, line)
    test_flags.regex_matched = true
    test_flags.captured_hp = matches[2]
    test_flags.captured_max_hp = matches[3]
end)

local disabled_tr = trigger.add("^disabled line$", {}, function()
    test_flags.disabled_fired = true
end)
disabled_tr:disable()

trigger.add("^gag me$", { gag = true }, function()
    test_flags.gagged_fired = true
end)

local count_tr = trigger.add("^count line$", { count = 2 }, function()
    test_flags.count_fires = test_flags.count_fires + 1
end)

local custom_grp = trigger.add_group()
custom_grp:add("^group disabled line$", {}, function()
    test_flags.group_disabled_fired = true
end)
custom_grp:disable()

local line_gagged = false
mud.add_output_listener(function(line)
    if line:line() == "gag me" and line:gag() then
        line_gagged = true
    end
    return line
end)

-- Send outputs to trigger line processing
mud.output("HP: 150/300")
mud.output("disabled line")
mud.output("gag me")
mud.output("count line")
mud.output("count line")
mud.output("count line") -- third attempt, count was 2 so this should not fire
mud.output("group disabled line")

timer.add(1, 1, function()
    assert(test_flags.regex_matched, "Regex trigger should match output line")
    assert_eq(test_flags.captured_hp, "150", "Captured HP should be 150")
    assert_eq(test_flags.captured_max_hp, "300", "Captured max HP should be 300")

    assert(not test_flags.disabled_fired, "Disabled trigger should not fire")
    assert(test_flags.gagged_fired, "Gag trigger callback should fire")
    assert(line_gagged, "Line should be gagged by trigger")

    assert_eq(test_flags.count_fires, 2, "Count trigger should fire exactly 2 times")
    assert_eq(trigger.get(count_tr.id), nil, "Count trigger should be removed once count reaches 0")

    assert(not test_flags.group_disabled_fired, "Triggers in disabled group should not fire")

    script.reset()
end)
