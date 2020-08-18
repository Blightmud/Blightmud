# Timers

Timers allow you to execute a callback function a set number of times with a
provided duration between each call.

## Creating a Timer

***blight:add_timer(secs, repeat, callback)***

- `secs`       The number of seconds to wait between calls to the callback function.
- `repeat`     The number of times to repeat the timer. A repeat of 0 will run the timer indefinitely.
- `callback`   The Lua function to run when the time has elapsed.

```lua
local count = 0
blight:add_timer(0.5, 3, function ()
    count = count + 1
    blight:send("say " .. count)
end)
```

***blight:get_timer_ids()***

- Returns a list of all timer ids

***blight:clear_timers()***

Removes all timers
