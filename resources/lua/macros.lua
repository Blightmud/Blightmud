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

blight:add_alias("^/aliases$", function ()
	local aliases = blight:get_aliases()

	for id,alias in pairs(aliases) do
		local enabled = state_label(alias.enabled, "enabled")
		blight:output(string.format("%s :\t" .. C_YELLOW .. "%s" .. C_RESET .. "\t%s", id, alias.regex, enabled))
	end
end)

blight:add_alias("^/triggers$", function ()
	local triggers = blight:get_triggers()

	for id,trigger in pairs(triggers) do
		local enabled = state_label(trigger.enabled, "enabled")
		local gag = state_label(trigger.gag, "gag")
		local raw = state_label(trigger.raw, "raw")
		local prompt = state_label(trigger.prompt, "prompt")
		blight:output(string.format("%s :\t" .. C_YELLOW .. "%s" .. C_RESET .. "\t%s\t%s\t%s\t%s", id, trigger.regex, enabled, gag, raw, prompt))
	end
end)
