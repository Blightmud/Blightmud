
-- Trigger to greet arriving people
--
-- Will send "greet Bob" to the mud if the line:
-- > Bob arrives from the west.
-- is received from the mud.
trigger.add("^(\\w+) arrives from .*$", {}, function (matches)
    local person = matches[2]
    mud.send("greet " .. person)
end)

-- The following alias should make killing things require less typing
alias.add("^k (.*)$", function (m)
    mud.send("kill " .. m[2])
end)

-- If your character has a drinking habit perhaps you would
-- like to automate that? Arrr!
--
-- The following will create a timer that will send
-- "drink rum" to the mud every 3 minutes for eternity
timer.add(60*3, 0, function ()
    mud.send("drink rum")
end)
