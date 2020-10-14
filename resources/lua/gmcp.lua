local unpack = unpack or table.unpack
local OPT = 201

local function GMCP()
	local self = {
		receivers = {},
		ready_listeners = {},
		gmcp_ready = false,
	}

	local function parse_gmcp(msg)
		local split = string.find(msg, " ")
		local mod = string.sub(msg, 0, split-1)
		local body = string.sub(msg, split)
		return mod, body
	end

	local function string_to_bytes(str)
		values = {}
		for i, v in utf8.codes(str) do
			values[i] = v
		end
		return values
	end

	local _on_enable = function (proto)
		if proto == OPT then
			self.gmcp_ready = true
			program, version = blight:version()
			local hello_obj = {
				Version=version,
				Client=program,
			}
			core:subneg_send(201, string_to_bytes("Core.Hello " .. json.encode(hello_obj)))
			for _,cb in ipairs(self.ready_listeners) do
				cb()
			end
		end
	end

	local _subneg_recv = function (proto, data)
		if proto == OPT then
			local msg = utf8.char(unpack(data))
			local mod, json = parse_gmcp(msg)
			if self.receivers[mod] ~= nil then
				self.receivers[mod](json)
			end
		end
	end

	local register = function (mod)
		core:subneg_send(OPT, string_to_bytes("Core.Supports.Add [\"" .. mod .. " 1\"]"))
	end

	local receive = function (mod, callback)
		self.receivers[mod] = callback
	end

	local send = function (msg)
		core:subneg_send(OPT, string_to_bytes(msg))
	end

	local on_ready = function (cb)
		table.insert(self.ready_listeners, cb)
		if self.gmcp_ready then
			cb()
		end
	end

	return {
		on_ready = on_ready,
		send = send,
		receive = receive,
		register = register,
		_subneg_recv = _subneg_recv,
		_on_enable = _on_enable,
	}
end

local gmcp = GMCP()

-- Register the module
core:enable_protocol(OPT)
core:on_protocol_enabled(function (proto) 
	gmcp._on_enable(proto)
end)
core:subneg_recv(function (proto, data)
	gmcp._subneg_recv(proto, data)
end)

return gmcp
