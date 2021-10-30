-- {0} and {1} are replaced in rust integration test
local ok, server = pcall(servers.get, "test_world")
if ok then
    servers.remove(server.name)
end
servers.add("test_world", "{0}", {1})
server = servers.get("test_world")
mud.connect(server.host, server.port)
mud.on_disconnect(function ()
    servers.remove("test_world")
end)
