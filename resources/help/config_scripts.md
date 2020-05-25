# Config scripts

Any `.lua` file placed in `$CONFIGDIR` will automatically load on program start. You can use this to automatically load the
right script depending on what mud you connect to or anything else you find useful.

Example config:
```lua
blight:on_connect(function (host, port)
    if host == "awesome-mud.net" then
        blight:reset()
        blight:load("/home/user/ws/scripts/awesome-mud-ai-battle-pwn-system.lua")
    elseif host == "noob-land.net" then
        blight:add_timer(3, 0, function ()
            blight:send("kick bunny")
            blight:send("get gold")
        end)
    end)
end)
```
