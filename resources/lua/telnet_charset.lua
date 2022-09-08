local PROTOCOL = 42
local REQUEST = 1
local ACCEPTED = 2
local REJECTED = 3

local ACCEPTED_ENCODINGS = {
    "UTF-8",
    "ASCII",
    "US-ASCII",
}

local unpack = table.unpack
local lower = string.lower

core.enable_protocol(PROTOCOL)

local function split(istr, sep)
    if sep == nil then
        sep = "%s"
    end
    local t={}
    for str in string.gmatch(istr, "([^" .. sep .. "]+)") do
        table.insert(t, str)
    end
    return t
end

local function string_to_bytes(str)
    local values = {}
    for i, v in utf8.codes(str) do
        values[i] = v
    end
    return values
end

local function send_accept(option)
    blight.debug("TELCHR[sending]: ACCEPTED " .. option)
    local payload = string_to_bytes(option)
    table.insert(payload, 1, ACCEPTED)
    core.subneg_send(PROTOCOL, payload)
end

local function send_reject()
    blight.debug("TELCHR[sending]: REJECTED")
    core.subneg_send(PROTOCOL, { REJECTED })
end

core.subneg_recv(function (proto, recv)
    if proto ~= PROTOCOL or recv[1] ~= REQUEST then
        return
    end

    table.remove(recv, 1) -- Remove the negotiation type
    local sep = utf8.char(recv[1]) -- Extract separator
    table.remove(recv, 1) -- Remove first separator
    local options = utf8.char(unpack(recv))
    blight.debug("TELCHR[received]: " .. options)

    for _,opt in ipairs(split(options, sep)) do
        for _,accepted in ipairs(ACCEPTED_ENCODINGS) do
            if lower(opt) == lower(accepted) then
                send_accept(opt)
                return
            end
        end
    end
    send_reject()
end)

core.on_protocol_enabled(function (proto)
    if proto == PROTOCOL then
        mud.add_tag("TELCHR")
    end
end)
