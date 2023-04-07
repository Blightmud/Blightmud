local MSSP_PROTO        = 70
local MSSP_VAR          = 1
local MSSP_VAL          = 2
local MSSP_CACHE_KEY    = "__mssp_recv_key"

local mod = {}
local mssp_values = json.decode(store.session_read(MSSP_CACHE_KEY) or "{}")

local function Info(msg)
    print("[MSSP]: " .. msg)
end

local function print_info()
    for k,v in pairs(mssp_values) do
        Info(k .. " = " .. v)
    end
end

local function decode(data)
    local parse_key, parse_val

    parse_key = function (index)
        local value = ""
        local i = index + 1
        while i <= #data do
            local val = data[i]
            if val == MSSP_VAL then
                return value, i
            else
                value = value .. utf8.char(val)
            end
            i = i + 1
        end
        -- Data should not end with a key
        print("[MSSP]: Malformed payload")
        return value, i
    end

    parse_val = function (index)
        local value = ""
        local i = index + 1
        while i <= #data do
            local val = data[i]
            if val == MSSP_VAR then
                return value, i
            else
                value = value .. utf8.char(val)
            end
            i = i + 1
        end
        return value, i
    end

    local i = 1
    local content = {}
    while i <= #data do
        local val = data[i]
        if val == MSSP_VAR then
            local key, value
            key, i = parse_key(i)
            value, i = parse_val(i)
            content[key] = value
        else
            Info("Unexpected byte: " .. val)
            Info("MSSP Parse failed")
            return content
        end
    end
    return content
end

core.enable_protocol(MSSP_PROTO)

core.on_protocol_enabled(function (proto)
    if proto == MSSP_PROTO then
        mud.add_tag("MSSP")
    end
end)

core.subneg_recv(function (proto, recv)
    if proto == MSSP_PROTO then
        mssp_values = decode(recv)
        store.session_write(MSSP_CACHE_KEY, json.encode(mssp_values))
    end
end)

mod.get = function ()
    return mssp_values
end
mod.print = print_info

return mod
