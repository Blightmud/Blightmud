local function get_args(cmd)
	local args={}
	for str in string.gmatch(cmd, "([^%s]+)") do
		table.insert(args, str)
	end
	return args
end

local function info(...)
    local args = {...}
    for _,msg in ipairs(args) do
        print("[**] " .. msg)
    end
end

local function error(...)
    local args = {...}
    for _,msg in ipairs(args) do
        print(cformat("<red>[!!]<reset> %s", msg))
    end
end

local function print_mud_output_usage()
	info("USAGE: /test <some string to test>")
end

alias.add("^/test$", function ()
	print_mud_output_usage()
end)

alias.add("^/test (.*)$", function (matches)
	local line = matches[2]:gsub("%s+", " ")
	if line:len() > 0 then
		mud.output(line)
	else
		print_mud_output_usage()
	end
end)

local function state_label (state, label)
    local len = label:len() + 1
	if not state then
        label = "-" .. label
        return cformat("<red>%" .. len .. "s<reset>", label)
    else
        label = "+" .. label
        return cformat("<green>%" .. len .. "s<reset>", label)
	end
end

local function number_label (number, label)
	local color = C_RED
	if number and number > 0 then
		color = C_GREEN
	end
	return label .. color .. tostring(number) .. C_RESET
end

alias.add("^/aliases$", function ()
	for id,alias in pairs(alias.get_group():get_aliases()) do
		local enabled = state_label(alias.enabled, "enabled")
		info(cformat("%4s : <yellow>%-20s<reset> %s", id, alias.regex:regex(), enabled))
	end
end)

alias.add("^/triggers$", function ()
	for id,trigger in pairs(trigger.get_group():get_triggers()) do
		local enabled = state_label(trigger.enabled, "enabled")
		local gag = state_label(trigger.gag, "gag")
		local raw = state_label(trigger.raw, "raw")
		local prompt = state_label(trigger.prompt, "prompt")
		local count = number_label(trigger.count, "count: ")
		info(cformat("%4s : <yellow>%-20s<reset> %s %s %s %s %s", id, trigger.regex:regex(), enabled, gag, raw, prompt, count))
	end
end)

-- TTS
alias.add("^/tts (on|off)$", function (matches)
	tts.enable(matches[2] == "on")
end)

alias.add("^/tts_rate ([-\\d]+)$", function (matches)
	tts.set_rate(matches[2])
end)

alias.add("^/tts_keypresses (on|off)$", function (matches)
	tts.echo_keypresses(matches[2] == "on")
end)

-- Settings
alias.add("^/settings$", function ()
	for key, value in pairs(settings.list()) do
		local key_format = cformat("<yellow>%-20s<reset>", key)
		local value_format
		if value then
			value_format = cformat("<bgreen>on<reset>")
		else
			value_format = cformat("<bred>off<reset>")
		end
		info(cformat("%s => %s", key_format, value_format))
	end
end)

alias.add("^/set ([^\\s]+)\\s*(on|off)?$", function (matches)
	local settings_table = settings.list()
	local key = matches[2]
	if settings_table[key] == nil then
		info(cformat("<red>Unknown setting: %s<reset>", key))
	else
		local value
		if matches[3] == "" then
			value = settings_table[key]
		else
			value = matches[3] == "on"
			settings.set(key, value)
		end
		local key_format = cformat("<yellow>%s<reset>", key)
		local value_format
		if value then
			value_format = cformat("<bgreen>on<reset>")
		else
			value_format = cformat("<bred>off<reset>")
		end
		info(cformat("%s => %s", key_format, value_format))
	end
end)

-- Connection
alias.add("^/connect.*$", function (m)
    local args = get_args(m[1])
    if #args == 2 then
        local result, server = pcall(servers.get, args[2])
        if result then
            info(cformat("Connecting to saved server: <yellow>%s<reset>", args[2]))
            mud.connect(server.host, server.port, server.tls, server.verify_cert)
        else
            error(server)
        end
    elseif #args == 3 then
        mud.connect(args[2], args[3])
    elseif #args == 4 then
        mud.connect(args[2], args[3], args[4] == "true", true)
    elseif #args >= 5 then
        mud.connect(args[2], args[3], args[4] == "true", args[5] == "true")
    else
        info(
            "USAGE: /connect <host> <port> [<tls> <verify>]",
            "USAGE: /connect <server>",
            "EXAMPLE: /connect examplemud.org 4000",
            "EXAMPLE: /connect example-tls-mud.org 4000 true",
            "EXAMPLE: /connect bad-cert-tls-mud.org 4000 true false",
            "EXAMPLE: /connect stored-server-name"
            )
    end
end)
alias.add("^(:?/disconnect|/dc)$", function ()
    mud.disconnect()
end)
alias.add("^(:?/reconnect|/rc)$", function ()
    mud.reconnect()
end)

-- Logging
alias.add("^/start_log.*$", function (m)
    local args = get_args(m[1])
    if #args == 2 then
        log.start(args[2])
    else
        info("USAGE: /start_log <name>")
    end
end)
alias.add("^/stop_log$", function ()
    log.stop()
end)
alias.add("^/load.*$", function (m)
    local args = get_args(m[1])
    if #args > 1 then
        script.load(table.concat(args, " ", 2))
    else
        info("USAGE: /load <path>")
    end
end)

-- Server handling
alias.add("^(?:/list_servers|/ls)$", function ()
    local stored = servers.get_all()
    table.sort(stored, function (a, b)
        return a.name < b.name
    end)

    for _,s in ipairs(stored) do
        local tls_str = cformat("TLS: <red>off<reset>")
        if s.tls then
            tls_str = cformat("TLS:  <green>on<reset>")
        end
        local verify_str = cformat("Verify: <red>off<reset>")
        if s.verify_cert then
            verify_str = cformat("Verify:  <green>on<reset>")
        end
        info(cformat("<yellow>%-12s<reset> Host: %-25s Port: <blue>%4s<reset> %s %s", s.name, s.host, s.port, tls_str, verify_str))
    end
end)
alias.add("^/add_server.*$", function (m)
    local args = get_args(m[1])
    if #args > 3 then
        local name = args[2]
        local host = args[3]
        local port = tonumber(args[4])
        local tls = false
        local validate = false
        if args[5] then
            tls = args[5]:lower() == "on" or args[5]:lower() == "true"
            validate = true
        end
        if args[6] then
            validate = args[6]:lower() == "on" or args[6]:lower() == "true"
        end
        local result, err = pcall(servers.add, name, host, port, tls, validate)
        if result then
            info(cformat("Server added: <yellow>%s<reset>", name))
        else
            error(err)
        end
    else
        info("USAGE: /add_server <name: String> <host: String> <port: Number> [<tls: String> <verify: String>]")
        info("EXAMPLE: /add_server example no-ssl-mud.com 4000")
        info("EXAMPLE: /add_server example ssl-mud.com 4400 true")
        info("EXAMPLE: /add_server example bad-cert-ssl-mud.com 4000 true false")
    end
end)
alias.add("^/remove_server.*$", function (m)
    local args = get_args(m[1])
    if #args > 1 then
        local name = args[2]
        local result, err = pcall(servers.remove, name)
        if result then
            info(cformat("Server removed: <yellow>%s<reset>", name))
        else
            error(err)
        end
    else
        info("USAGE: /remove_server <name: String>")
    end
end)
alias.add("^(?:/quit|/q)$", blight.quit)
alias.add("^/help.*$", function (m)
    local args = get_args(m[1])
    if #args > 1 then
        local name = args[2]
        blight.show_help(name, true)
    else
        blight.show_help("help", true)
    end
end)

-- Search
alias.add("^(?:/search|/s ).*$", function (m)
    local args = get_args(m[1])
    if #args == 1 then
        info(
            "USAGE: /search <regex>",
            "USAGE: /s <regex>"
            )
    else
        search.search(args[2])
    end
end)
