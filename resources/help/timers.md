# Timers

Timers allow you to execute a callback function a set number of times with a
provided duration between each call.

## Creating a Timer

***timer.add(secs, repeat, callback)***

- `secs`       The number of seconds to wait between calls to the callback function.
- `repeat`     The number of times to repeat the timer. A repeat of 0 will run the timer indefinitely.
- `callback`   The Lua function to run when the time has elapsed.

```lua
local count = 0
timer.add(0.5, 3, function ()
    count = count + 1
    mud.send("say " .. count)
end)
```

##

***timer.remove(timer_id)***

- `timer_id` The id returned when creating a timer

```lua
local count = 0
timer_id = timer.add(0.5, 3, function ()
    count = count + 1
    blight.output("And a " .. count)
    if count > 1 then
        -- This should only count to 2 and then show "Timer Removed".
        blight.output("Timer Removed")
        timer.remove(timer_id)
    end
end)
```

##

***timer.get_ids()***

- Returns a list of all timer ids

##

***timer.clear()***

Removes all timers

##

***timer.on_tick(callback)***

Will execute the provided callback every 100ms. The callback may take one
argument which will be the amount of milliseconds that have passed since
Blightmud was started.

- `callback` The callback function
