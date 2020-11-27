local mod = {}

local tasks = {}

local currentTask = nil

mod.Task = {}
local Task = mod.Task
Task.__index = Task


function Task.new(callable, ...)
    local ret = setmetatable({}, Task)

    local args = {...}

	ret.callable = callable
    ret.args = {...}
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

function Task.spawnLater(time, callable, ...)
    local ret = Task.new(callable, ...)

    ret:startLater(time)

    return ret
end

function Task.getcurrent()
    return currentTask
end

function Task:start()
    if self.dead then
        error("Attempt to start dead task")
    end
    tasks[self] = {time = 0}
end

function Task:startLater(time)
    if self.dead then
        error("Attempt to start dead task")
    end
    tasks[self] = {time = os.time() + time}
end

function Task:kill()
    self.dead = true
    tasks[self] = nil
end

function Task:send(value)
    self.sentData[#self.sentData+1] = value
end

function Task:sleep(time)
    if tasks[self].idle then return end

    if tasks[self].time < os.time() then
        tasks[self] = {time = os.time() + time}
    else
        tasks[self] = {time = tasks[self].time + time}
    end
end

function Task:idle()
    tasks[self].idle = true
end


mod.spawn = Task.spawn
mod.spawnLater = Task.spawnLater
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


function mod.getCurrent()
    return currentTask
end


function mod.getTasks()
    local ret = {}

    local idx = 1
    for task, _ in pairs(tasks) do
        ret[idx] = task
        idx = idx + 1
    end

    return ret
end


function mod.isTask(obj)
    return getmetatable(obj) == Task
end


local function runTask(task)
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
        ret = {coroutine.resume(task.coro, task.sentData)}
        task.sentData = {}
    else
        ret = {coroutine.resume(task.coro)}
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


blight:add_timer(0.1, 0, function()
    local somethingRan = false
    for task, timespec in pairs(tasks) do
        if timespec.time < os.time() and not timespec.idle then
            somethingRan = true
            runTask(task)
        end
    end
    if not somethingRan then
        for task, timespec in pairs(tasks) do
            if timespec.idle then
                runTask(task)
                timespec.idle = nil
            end
        end
    end
end)


return mod
