core.enable_protocol(24)

local program, _ = blight.version()
local term = os.getenv("TERM") or "xterm-256color"

local index = 1
local mod = {}
local auto_reader_mode = true

TTYPE_OPT_MTTS_ANSI =   0x001 -- Ansi support
TTYPE_OPT_MTTS_VT100 =  0x002 -- VT100 support
TTYPE_OPT_MTTS_UTF8 =   0x004 -- UTF-8 Support
TTYPE_OPT_MTTS_256C =   0x008 -- 256 color support
TTYPE_OPT_MTTS_MTRA =   0x010 -- Mouse tracking support
TTYPE_OPT_MTTS_OSCC =   0x020 -- OSC color palette support (true color)
TTYPE_OPT_MTTS_READ =   0x040 -- Client using screen reader
TTYPE_OPT_MTTS_PROX =   0x080 -- This is a proxy connection
TTYPE_OPT_MTTS_TRUC =   0x100 -- True color support
TTYPE_OPT_MTTS_MNES =   0x200 -- Mud New Env Standard enabled
TTYPE_OPT_MTTS_MSLP =   0x400 -- Mud Server Link Protocol enabled

-- Build the default MTTS value
local mtts = 0x0
mtts = mtts | TTYPE_OPT_MTTS_VT100
mtts = mtts | TTYPE_OPT_MTTS_ANSI
mtts = mtts | TTYPE_OPT_MTTS_UTF8
mtts = mtts | TTYPE_OPT_MTTS_256C
mtts = mtts | TTYPE_OPT_MTTS_TRUC

local NEGOTIATION_STACK = {}

local reader_mode = false

local function init()
    index = 1
    if auto_reader_mode then
        if tts.is_enabled() or blight.is_reader_mode() then
            mtts = mtts | TTYPE_OPT_MTTS_READ
        else
            mtts = mtts & ~TTYPE_OPT_MTTS_READ
        end
    end
    NEGOTIATION_STACK = {
        program,
        term,
        string.format("MTTS %d", mtts),
    }
end

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

core.subneg_recv(function (proto, recv)
    if proto == 24 and recv[1] == 1 then
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

core.on_protocol_enabled(function (proto)
    if proto == 24 then
        mud.add_tag("TTYPE")
        init()
    end
end)

function mod.set_term(new_term)
    term = new_term
    Info(string.format("Set TERM: %s", term))
end

function mod.set_mtts(new_mtts)
    mtts = new_mtts
    Info(string.format("Set MTTS: '0x%X'", mtts))
end

function mod.add_option(mtts_opt)
    local old_mtts = mtts
    mtts = mtts | mtts_opt
    Info(string.format("Updated MTTS 0x%X | 0x%X = 0x%X", old_mtts, mtts_opt, mtts))
end

function mod.rem_option(mtts_opt)
    local old_mtts = mtts
    mtts = mtts & ~mtts_opt
    Info(string.format("Updated MTTS 0x%X & ~0x%X = 0x%X", old_mtts, mtts_opt, mtts))
end

function mod.auto_detect_reader_mode(val)
    auto_reader_mode = val
    Info(string.format("Reader mode detection: %s", auto_reader_mode))
end

return mod
