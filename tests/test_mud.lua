local expected = ""

local send_stack = {
    "first line",
    "second line",
    "third line",
}

local function pop_stack()
    if #send_stack > 0 then
        local send = table.remove(send_stack, 1)
        expected = send
        mud.input(send)
    else
        mud.disconnect()
    end
end

mud.add_input_listener(function (line)
    blight.output("[INPUT]: " .. line:line())
    assert(line:line() == expected, string.format("'%s' != '%s'", line:line(), expected))
    return line
end)

mud.add_output_listener(function (line)
    blight.output("[RECV]: " .. line:line())
    if not line:prompt() then
        assert(line:line() == expected, string.format("'%s' != '%s'", line:line(), expected))
        pop_stack()
    end
    return line
end)

mud.on_connect(function (host)
    assert(host == "0.0.0.0")
    pop_stack()
end)

mud.on_disconnect(function ()
    blight.quit()
end)
