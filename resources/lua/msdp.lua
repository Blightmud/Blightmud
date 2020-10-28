local MSDP = 69
local MSDP_VAR = 1
local MSDP_VAL = 2
local MSDP_TABLE_OPEN = 3
local MSDP_TABLE_CLOSE = 4
local MSDP_ARRAY_OPEN = 5
local MSDP_ARRAY_CLOSE = 6

local function decode(data)
	local parse_var, parse_val, parse_array, parse_table

	parse_table = function(index)
		local obj = {}
		local i = index
		while i <= #data do
			local val = data[i]
			if val == MSDP_TABLE_CLOSE then
				return obj, i+1
			elseif val == MSDP_VAR then
				local name = ""
				local value = nil
				name, value, i = parse_var(i+1)
				obj[name] = value
			else
				blight:output("[MSDP]: Malformed table")
				i = i + 1
			end
		end
	end

	parse_array = function(index)
		local array = {}
		local i = index
		while i <= #data do
			local val = data[i]
			if val == MSDP_ARRAY_CLOSE then
				return array, i+1
			elseif val == MSDP_VAL then
				local value = nil
				value, i = parse_val(i+1)
				table.insert(array, value)
			else
				blight:output("[MSDP]: Malformed array")
				i = i + 1
			end
		end
	end

	parse_val = function(index)
		local val = data[index]
		if val == MSDP_TABLE_OPEN then
			return parse_table(index+1)
		elseif val == MSDP_ARRAY_OPEN then
			return parse_array(index+1)
		else
			local value = ""
			local i = index
			while i <= #data do
				local val = data[i]
				if val <= MSDP_ARRAY_CLOSE then
					return value, i
				else
					value = value .. string.char(val)
				end
				i = i + 1
			end
			return value, i
		end
	end

	parse_var = function(index)
		local name = ""
		local value = nil
		local i = index
		while i <= #data do
			local val = data[i]
			if val == MSDP_VAL then
				value, i = parse_val(i+1)
				break
			else
				name = name .. string.char(val)
			end
			i = i + 1
		end
		return name, value, i
	end

	local i = 1
	local content = {}
	while i <= #data do
		local val = data[i]
		if val == MSDP_VAR then
			local name = ""
			local value = nil
			name, value, i = parse_var(i+1)
			content[name] = value
		end
		i = i + 1
	end
	return content
end

function msdp()
	local self = {
		enabled = core:read("__msdp_enabled") == "true" or false,
		content = json.decode(core:read("__msdp_content") or "{}"),
		ready_listeners = {},
		update_listeners = {},
	}

	local function string_to_bytes(str)
		values = {}
		for i, v in utf8.codes(str) do
			values[i] = v
		end
		return values
	end

	local function concat(t1, t2)
		for _,v in ipairs(t2) do
			table.insert(t1, v)
		end
	end

	local function assemble(data)
		local bytes = {}
		for _,v in ipairs(data) do
			if type(v) == "string" then
				concat(bytes, string_to_bytes(v))
			else
				table.insert(bytes, v)
			end
		end
		return bytes
	end

	local function msdp_send(data)
		core:subneg_send(MSDP, assemble(data))
	end

	local function store_content(content)
		if content ~= nil then
			for k,v in pairs(content) do
				self.content[k] = v
			end
			core:store("__msdp_content", json.encode(self.content))
		else
			self.content = {}
			core:store("__msdp_content", json.encode(self.content))
		end
	end

	local get = function (key)
		return self.content[key]
	end

	local set = function (var, val)
		msdp_send({ MSDP_VAR, var, MSDP_VAL, val })
	end

	local register = function (value, cb)
		self.update_listeners[value] = cb
		if self.content[value] ~= nil then
			cb(self.content[value])
		end
	end

	local report = function (value)
		local payload = { MSDP_VAR, "REPORT" }
		if type(value) == "string" then
			table.insert(payload, MSDP_VAL)
			table.insert(payload, value)
			msdp_send(payload)
		elseif type(value) == "table" then
			for _,val in ipairs(value) do
				table.insert(payload, MSDP_VAL)
				table.insert(payload, val)
			end
			msdp_send(payload)
		end

	end

	local unreport = function (value)
		local payload = { MSDP_VAR, "UNREPORT" }
		if type(value) == "string" then
			table.insert(payload, MSDP_VAL)
			table.insert(payload, value)
			msdp_send(payload)
		elseif type(value) == "table" then
			for _,val in ipairs(value) do
				table.insert(payload, MSDP_VAL)
				table.insert(payload, val)
			end
			msdp_send(payload)
		end

	end

	local list = function (list)
		msdp_send({
				MSDP_VAR,
				"LIST",
				MSDP_VAL,
				list
			})
	end

	local send = function (var)
		local payload = { MSDP_VAR, "SEND" }
		if type(var) == "string" then
			table.insert(payload, MSDP_VAL)
			table.insert(payload, var)
			msdp_send(payload)
		elseif type(var) == "table" then
			for _,v in pairs(var) do
				table.insert(payload, MSDP_VAL)
				table.insert(payload, v)
			end
			msdp_send(payload)
		end
	end

	local on_ready = function (cb)
		table.insert(self.ready_listeners, cb)
		if self.enabled then
			cb()
		end
	end

	local _on_enable = function ()
		self.enabled = true
		core:store("__msdp_enabled", tostring(true))
		store_content(nil)
		for _,list in ipairs({
				"REPORTABLE_VARIABLES",
			}) do
			msdp_send({
					MSDP_VAR,
					"LIST",
					MSDP_VAL,
					list
				})
		end
		for _,cb in ipairs(self.ready_listeners) do
			cb()
		end
	end

	local _subneg_recv = function (data)
		local recv = decode(data)
		store_content(recv)
		for var,val in pairs(recv) do
			if self.update_listeners[var] ~= nil then
				self.update_listeners[var](val)
			end
		end
	end

	local _reset = function ()
		self.enabled = false
		core:store("__msdp_enabled", tostring(false))
		store_content(nil)
	end

	return {
		_on_enable = _on_enable,
		_subneg_recv = _subneg_recv,
		_reset = _reset,
		get = get,
		set = set,
		report = report,
		unreport = unreport,
		list = list,
		send = send,
		register = register,
		on_ready = on_ready,
	}
end

local msdp = msdp()
core:enable_protocol(MSDP)
core:on_protocol_enabled(function (proto) 
	if proto == MSDP then
		msdp._on_enable()
	end
end)
core:subneg_recv(function (proto, data)
	if proto == MSDP then
		msdp._subneg_recv(data)
	end
end)
blight:on_disconnect(function ()
	msdp._reset()
end)

return msdp
