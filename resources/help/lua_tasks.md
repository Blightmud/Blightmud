# Tasks

This library allows the creation and control of background tasks

Note that tasks have a safety function: If the task does not relinquish control
(e.g. with `tasks.yield` or `tasks.sleep`) for 2 seconds, the task will be
automatically killed.

##

***tasks.spawn(f, ...)***
Create a new task and schedule it for execution immediately

- `f`   The Lua function that will be executed
- `...` Arguments to the function
- Returns the `Task` that was created

Example:
```lua
tasks.spawn(function(a)
    blight.output(a)
end, "Hello World!")
```

##

***tasks.spawn_later(time, f, ...)***
Create a new task and schedule it for execution later

- `time` Number of seconds to wait before execution
- `f`    The Lua function that will be executed
- `...`  Arguments to the function
- Returns the `Task` that was created

##

***tasks.yield()***
Relinquishes control back to the system.

- Returns a table of sent data (see `Task:send`)

##

***tasks.sleep(time)***
Pauses execution for a given time

- `time` Number of seconds to sleep

Example:
```lua
local function someTask()
    blight.output("Hello")
    tasks.sleep(1)
    blight.output("World")
    tasks.sleep(2)
    blight.output("!")
end

tasks.spawn(someTask)
```
The example task runs for a total of 3 seconds.

##

***tasks.idle()***
Stops running the current task until no other tasks run

##

***tasks.get_current()***
Gets the currently running task

- Returns the `Task` that is currently running (nil if no task is running)

##

***tasks.get_tasks()***
Get all scheduled tasks

- Returns a table of all scheduled tasks

##

***tasks.is_task(table)***
Tests whether a given table is a `Task`

- `table` The table to test
- Returns `true` is `table` is a `Task`. `false` otherwise

# Task objects
`tasks.Task` is essentially a class, with multiple methods (some of them static)
Lua method syntax is used for normal methods, while normal table indexing is used for static methods

##

***tasks.Task.new(f, ...)***
Create a new `Task`. Note that the `Task` will NOT be scheduled

- `f`    The Lua function that will be executed
- `...`  Arguments to the function
- Returns the new `Task`

##

***tasks.Task.spawn(f, ...)***
Same as `tasks.spawn(f, ...)` above

Example:
```lua
tasks.Task.spawn(function(a)
    blight.output(a)
end, "Hello World!")
```

##

***tasks.Task.spawn_later(time, f, ...)***
Same as `tasks.spawn_later(time, f, ...)` above

##

***tasks.Task.get_current()***
Same as `tasks.get_current()` above

##

***tasks.Task:start()***
Schedules the task to run immediately
Note that this is not necessary if `tasks.spawn` is used.

##

***tasks.Task:startLater(time)***
Schedules the task to run after a given time

- `time` Number of seconds to wait before running the task

##

***tasks.Task:kill()***
Stops scheduling the task and marks it as dead

##

***tasks.Task:send(value)***
Sends a value to the task, which can retrieve it from the return value of `tasks.yield`.

- `value` The value to send

Example:
```lua
local function someTask()
    while true do
        local data = tasks.yield()
        for _, value in ipairs(data) do
            blight.output(value)
        end
    end
end

local task = tasks.spawn(someTask)
task:send("Hello")
task:send("World")
```
Output:
```
Hello
World
```

##

***tasks.Task:sleep(time)***
Stops running the task for a given number of seconds

- `time` Number of seconds to pause the task

##

***tasks.Task:idle()***
Stops running the task until no other tasks run
