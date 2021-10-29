local ok, server = pcall(servers.get, "test_world")
if ok then
    servers.remove(server.name)
end

servers.add("test_world", "0.0.0.0", 9876)

server = servers.get("test_world")

assert(server.name == "test_world")
assert(server.host == "0.0.0.0")
assert(server.port == 9876)
assert(not server.tls)
assert(not server.verify_cert)

mud.on_disconnect(function ()
    servers.remove("test_world")
end)
