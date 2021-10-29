local WORLD = "testing_world_name"

local ok, server = pcall(servers.get, WORLD)
if ok then
    servers.remove(server.name)
end


servers.add(WORLD, "0.0.0.0", 9876, false, false)

server = servers.get(WORLD)

assert(server.name == WORLD)
assert(server.host == "0.0.0.0")
assert(server.port == 9876)
assert(not server.tls)
assert(not server.verify_cert)
servers.remove(WORLD)
assert(not pcall(servers.get, WORLD))

blight.quit()
