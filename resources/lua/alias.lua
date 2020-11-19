local mod = {}

--------------------------------------------------------------------------------
-- Alias ---------------------------------------------------------------------
--------------------------------------------------------------------------------

-- next_id should be global over all AliasGroup so that an ID can uniquely
-- identify a alias
local next_id = 1

mod.Alias = {}
local Alias = mod.Alias
Alias.__index = Alias

function Alias.new(re, callback)
    local ret = setmetatable({}, Alias)

    ret.regex = regex.new(re)
    ret.callback = callback
    ret.enabled = true
    ret.id = next_id
    next_id = next_id + 1

    return ret
end

function Alias.isAlias(obj)
    return getmetatable(obj) == Alias
end

function Alias:enable()
    self.enabled = true
end

function Alias:disable()
    self.enabled = false
end

function Alias:setEnabled(flag)
    self.enabled = flag
end

function Alias:isEnabled()
    return self.enabled
end

function Alias:checkLine(line)
    local str = line:line()
    local matches = self.regex:match(str)
    if matches then
        self.callback(matches, line)
        line:matched(true)
    end
end

--------------------------------------------------------------------------------
-- AliasGroup ----------------------------------------------------------------
--------------------------------------------------------------------------------

local next_group_id = 1

mod.AliasGroup = {
}
local AliasGroup = mod.AliasGroup
AliasGroup.__index = AliasGroup

function AliasGroup.new(id)
    local ret = setmetatable({}, AliasGroup)

    ret.id = id
    ret.aliases = {}

    return ret
end

function AliasGroup:add(regex_or_alias, callback)
    local alias
    if Alias.isAlias(regex_or_alias) then
        alias = regex_or_alias
    else
        alias = Alias.new(regex_or_alias, callback)
    end
    self.aliases[alias.id] = alias
    return alias
end

function AliasGroup:get(id)
    return self.aliases[id]
end

function AliasGroup:getAliases()
    return self.aliases
end

function AliasGroup:remove(id)
    self.aliases[id] = nil
end

function AliasGroup:clear()
    self.aliases = {}
end

function AliasGroup:checkLine(line)
    local toRemove = {}
    for _, alias in pairs(self.aliases) do
        alias:checkLine(line)
        if alias.count == 0 then
            toRemove[#toRemove + 1] = alias.id
        end
    end
    for _, alias in ipairs(toRemove) do
        self:remove(alias)
    end
end

--------------------------------------------------------------------------------
-- module ----------------------------------------------------------------------
--------------------------------------------------------------------------------

mod.alias_groups = {
    AliasGroup.new(1)
}
local user_alias_groups = mod.alias_groups

mod.system_alias_groups = {
    AliasGroup.new(1)
}
local system_alias_groups = mod.system_alias_groups

local function getAliasGroups()
    if blight:is_core_mode() then
        return system_alias_groups
    end
    return user_alias_groups
end

function mod.add(regex, callback)
    return getAliasGroups()[1]:add(regex, callback)
end

function mod.get(id)
    for _, group in pairs(getAliasGroups()) do
        local alias = group:get(id)
        if alias then return alias end
    end
    return nil
end

function mod.getGroup(id)
    if not id then id = 1 end
    return getAliasGroups()[id]
end

function mod.remove(id)
    for _, group in pairs(getAliasGroups()) do
        group:remove(id)
    end
end

function mod.clear()
    for _, group in pairs(getAliasGroups()) do
        group:clear()
    end
end

function mod.addGroup()
    local ret = AliasGroup.new(next_group_id)
    getAliasGroups()[next_group_id] = ret
    next_group_id = next_group_id + 1

    return ret
end

mud.add_input_listener(function(line)
    for _, group in pairs(system_alias_groups) do
        group:checkLine(line)
    end
    for _, group in pairs(user_alias_groups) do
        group:checkLine(line)
    end
    return line
end)

return mod
