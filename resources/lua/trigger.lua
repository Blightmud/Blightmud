local mod = {}

--------------------------------------------------------------------------------
-- Trigger ---------------------------------------------------------------------
--------------------------------------------------------------------------------

-- next_id should be global over all TriggerGroups so that an ID can uniquely
-- identify a trigger
local next_id = 1

mod.Trigger = {}
local Trigger = mod.Trigger
Trigger.__index = Trigger

function Trigger.new(re, options, callback)
    local ret = setmetatable({}, Trigger)

    ret.regex = regex.new(re)
    ret.callback = callback
    ret.gag = options.gag or false
    ret.raw = options.raw or false
    ret.prompt = options.prompt or false
    ret.count = options.count or nil
    ret.enabled = options.enabled or true
    ret.id = next_id
    next_id = next_id + 1

    return ret
end

function Trigger.isTrigger(obj)
    return getmetatable(obj) == Trigger
end

function Trigger:enable()
    self.enabled = true
end

function Trigger:disable()
    self.enabled = false
end

function Trigger:setEnabled(flag)
    self.enabled = flag
end

function Trigger:isEnabled()
    return self.enabled
end

function Trigger:checkLine(line)
    if line:prompt() ~= self.prompt then return end
    local str
    if self.raw then
        str = line:raw()
    else
        str = line:line()
    end

    local matches = self.regex:match(str)
    if matches then
        if self.gag then
            line:gag(true)
        end
        line:matched(true)

        self.callback(matches, line)

        if self.count and self.count > 0 then
            self.count = self.count - 1
        end
    end
end

--------------------------------------------------------------------------------
-- TriggerGroup ----------------------------------------------------------------
--------------------------------------------------------------------------------

local next_group_id = 1

mod.TriggerGroup = {
}
local TriggerGroup = mod.TriggerGroup
TriggerGroup.__index = TriggerGroup

function TriggerGroup.new(id)
    local ret = setmetatable({}, TriggerGroup)

    ret.id = id
    ret.triggers = {}

    return ret
end

function TriggerGroup:add(regex_or_trigger, options, callback)
    local trigger
    if Trigger.isTrigger(regex_or_trigger) then
        trigger = regex_or_trigger
    else
        trigger = Trigger.new(regex_or_trigger, options, callback)
    end
    self.triggers[trigger.id] = trigger
    return trigger
end

function TriggerGroup:get(id)
    return self.triggers[id]
end

function TriggerGroup:getTriggers()
    return self.triggers
end

function TriggerGroup:remove(id)
    self.triggers[id] = nil
end

function TriggerGroup:clear()
    self.triggers = {}
end

function TriggerGroup:checkLine(line)
    local toRemove = {}
    for _, trigger in pairs(self.triggers) do
        trigger:checkLine(line)
        if trigger.count == 0 then
            toRemove[#toRemove + 1] = trigger.id
        end
    end
    for _, trigger in ipairs(toRemove) do
        self:remove(trigger)
    end
end

--------------------------------------------------------------------------------
-- module ----------------------------------------------------------------------
--------------------------------------------------------------------------------

mod.trigger_groups = {
    TriggerGroup.new(1)
}
local user_trigger_groups = mod.trigger_groups

mod.system_trigger_groups = {
    TriggerGroup.new(1)
}
local system_trigger_groups = mod.system_trigger_groups

local function getTriggerGroups()
    if blight:is_core_mode() then
        return system_trigger_groups
    end
    return user_trigger_groups
end

function mod.add(regex, options, callback)
    return getTriggerGroups()[1]:add(regex, options, callback)
end

function mod.get(id)
    for _, group in pairs(getTriggerGroups()) do
        local trigger = group:get(id)
        if trigger then return trigger end
    end
    return nil
end

function mod.getGroup(id)
    if not id then id = 1 end
    return getTriggerGroups()[id]
end

function mod.remove(id)
    for _, group in pairs(getTriggerGroups()) do
        group:remove(id)
    end
end

function mod.clear()
    for _, group in pairs(getTriggerGroups()) do
        group:clear()
    end
end

function mod.addGroup()
    local ret = TriggerGroup.new(next_group_id)
    getTriggerGroups()[next_group_id] = ret
    next_group_id = next_group_id + 1

    return ret
end

mud.add_output_listener(function(line)
    for _, group in pairs(system_trigger_groups) do
        group:checkLine(line)
    end
    for _, group in pairs(user_trigger_groups) do
        group:checkLine(line)
    end
    return line
end)

return mod
