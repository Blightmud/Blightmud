core.enable_protocol(24)
local program, _ = blight.version()
local term = os.getenv("TERM")
local mtts = "MTTS 271"

local index = 1
local mod = {}

local NEGOTIATION_STACK = {
    program,
    term,
    mtts,
}

local function Info(msg)
    print("[TTYPE]: " .. msg)
end

local function string_to_bytes(str)
    local values = {}
    for i, v in utf8.codes(str) do
        values[i] = v
    end
    return values
end

local function concat(a, b)
    for _,v in ipairs(b) do
        a[#a+1] = v
    end
    return a
end

core.subneg_recv(function (proto, data)
    if proto == 24 and data[1] == 1 then
        local data = NEGOTIATION_STACK[index]:upper()
        blight.debug("[TTYPE] Negotiating: " .. data)
        local payload = concat({0}, string_to_bytes(data))
        core.subneg_send(24, payload)
        index = index + 1
        if index > #NEGOTIATION_STACK then
            index = #NEGOTIATION_STACK
        end
    end
end)

function mod.set_term(term_type)
    Info(string.format("Setting TERM '%s' => '%s'", term, term_type))
    term = term_type
end

function mod.set_mtts(mtts_type)
    Info(string.format("Setting MTTS '%s' => '%s'", mtts, mtts_type))
    mtts = mtts_type
end

return mod
