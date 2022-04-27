require "tests.common"

mud.on_connect(function ()
    assert_eq(mud.is_connected(), true)
    mud.disconnect()
end)

mud.on_disconnect(function ()
    assert_eq(mud.is_connected(), false)
    blight.quit()
end)
