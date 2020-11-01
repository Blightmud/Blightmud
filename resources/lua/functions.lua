function bytes_to_string(bytes)
	local chars = {}
	for _, v in ipairs(bytes) do
		local byte = v < 0 and (0xff + v + 1) or v
		table.insert(chars, string.char(byte))
	end
	return table.concat(chars)
end

-- Make Lua's `print()` write to Blightmud's output buffer.
function _G.print(...)
	blight:output(...)
end

function cformat(msg, ...)
  msg = msg:gsub("<(.-)>", function (s)
    if s:find(':', 1, true) then
      local fg, bg = s:match('(%w+):(%w+)')
      return _G['C_' .. fg:upper()] .. _G['BG_' .. bg:upper()]
    else
      return _G['C_' .. s:upper()]
    end
  end)

  return msg:format(...)
end
