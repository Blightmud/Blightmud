-- MCCP2 integration test Lua script
-- Registers triggers to verify decompressed messages and sends confirmation back

mud.add_output_listener(function(line)
    local text = line:line()

    if text == "MCCP2_TEST_MESSAGE" then
        -- Send confirmation back to the test server
        mud.send("MCCP2_OK")
    elseif text == "MCCP2_INCREMENTAL_TEST" then
        -- Send confirmation for incremental test
        mud.send("MCCP2_INCREMENTAL_OK")
    end

    return line
end)

mud.on_disconnect(function()
    blight.quit()
end)
