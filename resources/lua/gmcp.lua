local unpack = table.unpack
local OPT = 201

local function GMCP()
    local self = {
        receivers = {},
        ready_listeners = {},
        echo_gmcp = store.session_read("__echo_gmcp") == "true",
        gmcp_ready = store.session_read("__gmcp_ready") == "true",
        recv_cache = json.decode(store.session_read("__gmcp_recv_cache") or "{}"),
    }

    local function parse_gmcp(msg)
        local mod = msg
        local body = {}
        local split = string.find(msg, " ")
        if split ~= nil then
            mod = string.sub(msg, 0, split-1)
            body = string.sub(msg, split)
        end
        return mod, body
    end

    local function string_to_bytes(str)
        local values = {}
        for i, v in utf8.codes(str) do
            values[i] = v
        end
        return values
    end

    local _on_enable = function (proto)
        if proto == OPT then
            print("[GMCP]: GMCP is ready and available for your current mud")
            self.gmcp_ready = true
            store.session_write("__gmcp_ready", "true")
            local program, version = blight.version()
            local hello_obj = {
                Version=version,
                Client=program,
            }
            core.subneg_send(201, string_to_bytes("Core.Hello " .. json.encode(hello_obj)))
            for _,cb in ipairs(self.ready_listeners) do
                cb()
            end
        end
    end

    local _subneg_recv = function (proto, data)
        if proto == OPT then
            local msg = utf8.char(unpack(data))
            local mod, json_data = parse_gmcp(msg)
            self.recv_cache[mod] = json_data
            store.session_write("__gmcp_recv_cache", json.encode(self.recv_cache))
            if self.echo_gmcp then
                blight.output("[GMCP]: " .. msg)
            end
            if self.receivers[mod] ~= nil then
                for _,cb in ipairs(self.receivers[mod]) do
                    cb(json_data)
                end
            end
        end
    end

    local echo = function (enabled)
        store.session_write("__echo_gmcp", tostring(enabled))
        self.echo_gmcp = enabled
    end

    local register = function (mod)
        core.subneg_send(OPT, string_to_bytes("Core.Supports.Add [\"" .. mod .. " 1\"]"))
    end

    local receive = function (mod, callback)
        if self.receivers[mod] == nil then
            self.receivers[mod] = {}
        end
        table.insert(self.receivers[mod], callback)
        if self.recv_cache[mod] ~= nil then
            callback(self.recv_cache[mod])
        end
    end

    local send = function (msg)
        core.subneg_send(OPT, string_to_bytes(msg))
    end

    local on_ready = function (cb)
        table.insert(self.ready_listeners, cb)
        if self.gmcp_ready then
            cb()
        end
    end

    local _reset = function ()
        self.gmcp_ready = false
        self.recv_cache = {}
        store.session_write("__gmcp_recv_cache", "{}")
        store.session_write("__gmcp_ready", tostring(false))
    end

    return {
        on_ready = on_ready,
        send = send,
        receive = receive,
        register = register,
        echo = echo,
        _subneg_recv = _subneg_recv,
        _on_enable = _on_enable,
        _reset = _reset,
    }
end

local gmcp = GMCP()

-- Register the module
core.enable_protocol(OPT)
core.on_protocol_enabled(function (proto)
    gmcp._on_enable(proto)
end)
core.subneg_recv(function (proto, data)
    gmcp._subneg_recv(proto, data)
end)
mud.on_disconnect(function ()
    gmcp._reset()
end)

return gmcp
