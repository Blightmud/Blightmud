-- See https://www.rfc-editor.org/rfc/rfc1073.html
local NAWS_PROTOCOL = 31
local naws_enabled = false

-- Return table with the network byte order (e.g. big endian) encoding of the
-- width and height given.
local function network_dimensions(width, height)
    bytes = {}
    local append_byte = function(c)
        table.insert(bytes, string.byte(c))
    end
    string.pack(">i2", width):gsub(".", append_byte)
    string.pack(">i2", height):gsub(".", append_byte)
    return bytes
end

local function send_dimensions(width, height)
    -- We must adjust the height to just the writable area, subtracting
    -- the size by 2 for the input/prompt area, and by the size of the status
    -- area.
    height = height - 2 - blight.status_height()
    core.subneg_send(NAWS_PROTOCOL, network_dimensions(width, height))
end

-- Advertise NAWS support.
core.enable_protocol(NAWS_PROTOCOL)

-- If NAWS is negotiated update our enabled status and send the current
-- window dimensions.
core.on_protocol_enabled(function (proto)
    if proto == NAWS_PROTOCOL then
        mud.add_tag("NAWS")
        naws_enabled = true
        send_dimensions(blight.terminal_dimensions())
    end
end)

core.on_protocol_disabled(function (proto)
    if proto == NAWS_PROTOCOL then
        mud.remove_tag("NAWS")
        naws_enabled = false
    end
end)

-- When dimensions change, send an updated NAWS message when enabled.
blight.on_dimensions_change(function (width, height)
    if naws_enabled then
        send_dimensions(width, height)
    end
end)
