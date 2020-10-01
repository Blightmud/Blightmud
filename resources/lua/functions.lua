function bytes_to_string(bytes)
	local chars = {}
	for _, v in ipairs(bytes) do
		local byte = v < 0 and (0xff + v + 1) or v
		table.insert(chars, string.char(byte))
	end
	return table.concat(chars)
end

