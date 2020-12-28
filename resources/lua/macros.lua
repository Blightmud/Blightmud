local function print_mud_output_usage()
	blight.output("USAGE: /test <some string to test>")
end

alias.add("^/test$", function (matches)
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

alias.add("^/aliases$", function ()
	for id,alias in pairs(alias.getGroup():getAliases()) do
		local enabled = state_label(alias.enabled, "enabled")
		blight.output(cformat("%s :\t<yellow>%s<reset>\t%s", id, alias.regex:regex(), enabled))
	end
end)

alias.add("^/triggers$", function ()
	for id,trigger in pairs(trigger.getGroup():getTriggers()) do
		local enabled = state_label(trigger.enabled, "enabled")
		local gag = state_label(trigger.gag, "gag")
		local raw = state_label(trigger.raw, "raw")
		local prompt = state_label(trigger.prompt, "prompt")
		local count = number_label(trigger.count, "count: ")
		blight.output(cformat("%s :\t<yellow>%s<reset>\t%s\t%s\t%s\t%s\t%s", id, trigger.regex:regex(), enabled, gag, raw, prompt, count))
	end
end)

alias.add("^/tts (on|off)$", function (matches)
	tts:enable(matches[2] == "on")
end)

alias.add("^/tts_rate ([-\\d]+)$", function (matches)
	tts:set_rate(matches[2])
end)

alias.add("^/tts_keypresses (on|off)$", function (matches)
	tts:echo_keypresses(matches[2] == "on")
end)
