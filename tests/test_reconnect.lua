require "tests.common"

local connection_count = 0
mud.on_connect(function ()
    connection_count = connection_count + 1
    print(string.format("Connected: %d", connection_count))
    mud.disconnect()
end)
mud.on_disconnect(function ()
    print(string.format("Disconnected: %d", connection_count))
    if connection_count == 1 then
        mud.reconnect()
    else
        assert_eq(connection_count, 2)
        blight.quit()
    end
end)
