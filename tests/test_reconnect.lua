require "tests.common"

local connection_count = 0
mud.on_connect(function ()
    connection_count = connection_count + 1
    if connection_count == 1 then
        mud.disconnect()
    end
end)
mud.on_disconnect(function ()
    if connection_count == 1 then
        mud.reconnect()
    else
        assert_eq(connection_count, 2)
        blight.quit()
    end
end)
