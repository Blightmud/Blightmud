blight:core_mode(true)

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

blight:core_mode(false)
