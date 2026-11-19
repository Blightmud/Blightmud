function bytes_to_string(bytes)
    local chars = {}
    for _, v in ipairs(bytes) do
        local byte = v < 0 and (0xff + v + 1) or v
        table.insert(chars, string.char(byte))
    end
    return table.concat(chars)
end

-- Make Lua's `print()` write to Blightmud's output buffer.
function _G.print(...)
    local strings = {}
    for _, v in ipairs({ ... }) do
        table.insert(strings, tostring(v))
    end
    blight.output(table.unpack(strings))
end

function cformat(msg, ...)
    msg = msg:gsub("<(.-)>", function(s)
        if s:find(":", 1, true) then
            local fg, bg = s:match("(%w+):(%w+)")
            return _G["C_" .. fg:upper()] .. _G["BG_" .. bg:upper()]
        else
            return _G["C_" .. s:upper()]
        end
    end)

    return msg:format(...)
end

-- Keys currently bound to insert_newline, so a later call can unbind them.
local newline_keys = {}

-- Reproduce the normalisation blight.bind applies, so that unbind removes what
-- bind stored. bind lowercases the key *except* in the alt- branch, where it
-- lowercases only the prefix so that alt-H and alt-h stay distinct; unbind
-- does no normalisation at all and sets the raw string to nil.
local function normalize_bind_key(key)
    if key:lower():sub(1, 4) == "alt-" then
        return "alt" .. key:sub(4)
    end
    return key:lower()
end

--- Retarget the keys that insert a row in the input area.
---
--- Replaces the whole group, so a user does not have to know and unbind each
--- default individually:
---
---   set_newline_keys({ "alt-enter" })            -- Alt/Option+Enter only
---   set_newline_keys({ "ctrl-o" })               -- macOS, no Meta needed
---   set_newline_keys({ "ctrl-o", "\x1b[13;2u" })  -- and a CSI-u Shift+Enter
---   set_newline_keys({})                         -- disable row insertion
---
--- Enter always submits and is not affected.
---@param keys string[]
function set_newline_keys(keys)
    for _, key in ipairs(newline_keys) do
        -- blight.bind lowercases the key except in the alt- branch, while
        -- blight.unbind does no normalisation at all, so unbinding has to use
        -- exactly the string bind would have stored.
        blight.unbind(normalize_bind_key(key))
    end

    newline_keys = {}
    for _, key in ipairs(keys) do
        blight.bind(key, function()
            blight.ui("insert_newline")
        end)
        table.insert(newline_keys, key)
    end
end
