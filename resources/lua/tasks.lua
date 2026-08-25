local mod = {}

local tasks = {}
-- used to prevent adding to tasks while iterating over it
local pendingTasks = {}

local currentTask = nil

mod.Task = {}
local Task = mod.Task
Task.__index = Task

function Task.new(callable, ...)
    local ret = setmetatable({}, Task)

    local args = { ... }

    ret.callable = callable
    ret.args = { ... }
    ret.coro = coroutine.create(function()
        return callable(table.unpack(args))
    end)
    ret.sentData = {}
    ret.started = false
    ret.dead = false
    ret.error = nil
    ret.value = nil
    ret.success = false

    return ret
end

function Task.spawn(callable, ...)
    local ret = Task.new(callable, ...)

    ret:start()

    return ret
end

function Task.spawn_later(time, callable, ...)
    local ret = Task.new(callable, ...)

    ret:startLater(time)

    return ret
end

function Task.get_current()
    return currentTask
end

function Task:start()
    if self.dead then
        error("Attempt to start dead task")
    end
    pendingTasks[self] = { time = 0 }
end

function Task:startLater(time)
    if self.dead then
        error("Attempt to start dead task")
    end
    pendingTasks[self] = { time = os.time() + time }
end

function Task:kill()
    self.dead = true
    tasks[self] = nil
    pendingTasks[self] = nil
end

function Task:send(value)
    self.sentData[#self.sentData + 1] = value
end

function Task:sleep(time)
    local data = tasks[self] or pendingTasks[self]
    if not data or data and data.idle then
        -- not sure why we don't want to allow adding more of a delay to a task. would allow "run task after x seconds once nothing else is happening"
        return
    end

    if data.time < os.time() then
        data = { time = os.time() + time }
    else
        data = { time = data.time + time }
    end
end

function Task:idle()
    local data = tasks[self] or pendingTasks[self]
    if data then
        data.idle = true
    end
end

mod.spawn = Task.spawn
mod.spawn_later = Task.spawn_later
mod.yield = coroutine.yield

function mod.sleep(time)
    if currentTask == nil then
        error("Cannot sleep main task", 2)
    end
    currentTask:sleep(time)
    coroutine.yield()
end

function mod.idle()
    if currentTask == nil then
        error("Cannot sleep main task", 2)
    end
    currentTask:idle()
    coroutine.yield()
end

function mod.get_current()
    return currentTask
end

function mod.get_tasks()
    local ret = {}

    local idx = 1
    for task, _ in pairs(tasks) do
        ret[idx] = task
        idx = idx + 1
    end
    for task, _ in pairs(pendingTasks) do
        ret[idx] = task
        idx = idx + 1
    end

    return ret
end

function mod.is_task(obj)
    return getmetatable(obj) == Task
end

local function run_task(task)
    currentTask = task
    local startTime = os.time()
    debug.sethook(task.coro, function()
        if os.time() > startTime + 2 then
            debug.sethook()
            error("Task has been running for +2 seconds without yielding. Aborting", 2)
        end
    end, "", 500)
    local ret
    if task.started then
        ret = { coroutine.resume(task.coro, task.sentData) }
        task.sentData = {}
    else
        ret = { coroutine.resume(task.coro) }
        task.started = true
    end
    debug.sethook()
    currentTask = nil

    if coroutine.status(task.coro) == "dead" then
        task.dead = true
        task.success = ret[1]

        table.remove(ret, 1)
        if task.success then
            task.value = ret
        else
            task.error = ret
        end
        tasks[task] = nil
    end
end

timer.on_tick(function(millis)
    local somethingRan = false

    for task, timespec in pairs(tasks) do
        if timespec.time < os.time() and not timespec.idle then
            somethingRan = true
            run_task(task)
        end
    end

    if not somethingRan then
        for task, timespec in pairs(tasks) do
            if timespec.idle then
                run_task(task)
                timespec.idle = nil
            end
        end
    end

    for task, timespec in pairs(pendingTasks) do
        tasks[task] = timespec
        pendingTasks[task] = nil
    end
end)

return mod
