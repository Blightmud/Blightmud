local function print_mud_output_usage()
	blight:output("USAGE: /test <some string to test>")
end

blight:add_alias("^/test$", function (matches)
	print_mud_output_usage()
end)

blight:add_alias("^/test (.*)$", function (matches)
	local line = matches[2]:gsub("%s+", "")
	if line:len() > 0 then
		blight:mud_output(line)
	else
		print_mud_output_usage()
	end
end)

local function state_label (state, label)
	local color = C_RED
	if state then
		color = C_GREEN
	end
	return color .. label .. C_RESET
end

local function number_label (number, label)
	local color = C_RED
	if number and number > 0 then
		color = C_GREEN
	end
	return label .. color .. tostring(number) .. C_RESET
end

blight:add_alias("^/aliases$", function ()
	local aliases = blight:get_aliases()

	for id,alias in pairs(aliases) do
		local enabled = state_label(alias.enabled, "enabled")
		blight:output(string.format("%s :\t" .. C_YELLOW .. "%s" .. C_RESET .. "\t%s", id, alias.regex, enabled))
	end
end)

blight:add_alias("^/triggers$", function ()
	for id,trigger in pairs(trigger.getGroup():getTriggers()) do
		local enabled = state_label(trigger.enabled, "enabled")
		local gag = state_label(trigger.gag, "gag")
		local raw = state_label(trigger.raw, "raw")
		local prompt = state_label(trigger.prompt, "prompt")
		local count = number_label(trigger.count, "count: ")
		blight:output(string.format("%s :\t" .. C_YELLOW .. "%s" .. C_RESET .. "\t%s\t%s\t%s\t%s\t%s", id, trigger.regex:regex(), enabled, gag, raw, prompt, count))
	end
end)

blight:add_alias("^/tts (on|off)$", function (matches)
	tts:enable(matches[2] == "on")
end)

blight:add_alias("^/tts_rate ([-\\d]+)$", function (matches)
	tts:set_rate(matches[2])
end)

blight:add_alias("^/tts_keypresses (on|off)$", function (matches)
	tts:echo_keypresses(matches[2] == "on")
end)
