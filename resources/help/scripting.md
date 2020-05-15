# Scripting

The default scripting language for blightmud is Lua.

You can load your scriptfile with the /load command.

In lua you have access to the 'blight' object. Methods are available on this
object to interact with your game.

The following methods exist:
---
=> blight:output(str)
    Prints output to the output screen
    Eg. 'blight:output("A", "nice", "message")'
    Will print "A nice message" on the screen

=> blight:send(str)
    Sends a command to the mud.
    Eg. 'blight:send("kill bat")'
    Will send the command "kill bat" to the server.


