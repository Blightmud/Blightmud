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

function Alias.is_alias(obj)
    return getmetatable(obj) == Alias
end

function Alias:enable()
    self.enabled = true
end

function Alias:disable()
    self.enabled = false
end

function Alias:set_enabled(flag)
    self.enabled = flag
end

function Alias:is_enabled()
    return self.enabled
end

function Alias:check_line(line)
    local str = line:line()
    local matches = self.regex:match(str)
    if matches then
        line:matched(true)
        local startTime = os.time()
        debug.sethook(function ()
            if os.time() > startTime + 2 then
                debug.sethook()
                error("Alias callback has been running for +2 seconds. Aborting", 2)
            end
        end, "", 500)
    self.callback(matches, line)
    debug.sethook()
end
end

--------------------------------------------------------------------------------
-- AliasGroup ----------------------------------------------------------------
--------------------------------------------------------------------------------

local next_group_id = 2

mod.AliasGroup = {
}
local AliasGroup = mod.AliasGroup
AliasGroup.__index = AliasGroup

function AliasGroup.new(id)
    local ret = setmetatable({}, AliasGroup)

    ret.id = id
    ret.enabled = true
    ret.aliases = {}

    return ret
end

function AliasGroup:add(regex_or_alias, callback)
    local alias
    if Alias.is_alias(regex_or_alias) then
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

function AliasGroup:get_aliases()
    return self.aliases
end

function AliasGroup:remove(id)
    self.aliases[id] = nil
end

function AliasGroup:clear()
    self.aliases = {}
end

function AliasGroup:is_enabled()
    return self.enabled
end

function AliasGroup:set_enabled(flag)
    self.enabled = flag
end

function AliasGroup:enable()
    self.enabled = true
end

function AliasGroup:disable()
    self.enabled = false
end

function AliasGroup:check_line(line)
    local toRemove = {}
    if not self.enabled then
        return
    end
    for _, alias in pairs(self.aliases) do
        alias:check_line(line)
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

local function get_alias_groups()
    if blight.is_core_mode() then
        return system_alias_groups
    end
    return user_alias_groups
end

function mod.add(regex, callback)
    return get_alias_groups()[1]:add(regex, callback)
end

function mod.get(id)
    for _, group in pairs(get_alias_groups()) do
        local alias = group:get(id)
        if alias then return alias end
    end
    return nil
end

function mod.get_group(id)
    if not id then id = 1 end
    return get_alias_groups()[id]
end

function mod.remove(id)
    for _, group in pairs(get_alias_groups()) do
        group:remove(id)
    end
end

function mod.clear()
    for _, group in pairs(get_alias_groups()) do
        group:clear()
    end
end

function mod.add_group()
    local ret = AliasGroup.new(next_group_id)
    get_alias_groups()[next_group_id] = ret
    next_group_id = next_group_id + 1

    return ret
end

mud.add_input_listener(function(line)
    for _, group in pairs(system_alias_groups) do
        group:check_line(line)
    end
    for _, group in pairs(user_alias_groups) do
        group:check_line(line)
    end
    return line
end)

return mod
