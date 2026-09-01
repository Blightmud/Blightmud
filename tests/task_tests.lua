require("tests.common")

script.on_reset(function()
    blight.quit()
end)
local sleep_period = 0.5 -- we want this to be multiple ticks
local result_wait = sleep_period + 0.3 -- 1 tick to start sleep_task + sleep_period + 1 tick to resume + a bit more so we are in the next tick

assert(tasks.is_task(tasks.Task.new(function() end)), "Task.new object should be a task")
assert(not tasks.is_task({}), "Plain table should not be a task")
assert(not tasks.is_task(123), "Number should not be a task")
assert(not tasks.is_task("string"), "String should not be a task")
assert(not tasks.is_task(nil), "Nil should not be a task")
assert(not tasks.is_task(function() end), "Function should not be a task")
assert(not tasks.is_task(true), "Boolean should not be a task")

local unstarted_task = tasks.Task.new(function(a, b) return a + b end, 10, 20)
assert(tasks.is_task(unstarted_task), "Task.new result should be a task")
assert_eq(unstarted_task.started, false)
assert_eq(unstarted_task.dead, false)
assert_eq(unstarted_task.success, false)
assert_eq(unstarted_task.value, nil)
assert_eq(unstarted_task.error, nil)
assert_eq(#unstarted_task.args, 2)
assert_eq(unstarted_task.args[1], 10)
assert_eq(unstarted_task.args[2], 20)

assert_eq(tasks.get_current(), nil)
assert_eq(tasks.Task.get_current(), nil)

local sleep_ok, sleep_err = pcall(function() tasks.sleep(sleep_period) end)
assert(not sleep_ok, "tasks.sleep in main task should fail")
assert(string.find(sleep_err, "Cannot sleep main task"), "Error should mention 'Cannot sleep main task'")

local idle_ok, idle_err = pcall(function() tasks.idle() end)
assert(not idle_ok, "tasks.idle in main task should fail")
assert(string.find(idle_err, "Cannot sleep main task"), "Error should mention 'Cannot sleep main task'")

local dead_task = tasks.Task.new(function() end)
dead_task:kill()
assert_eq(dead_task.dead, true)

local start_dead_ok, start_dead_err = pcall(function() dead_task:start() end)
assert(not start_dead_ok, "Starting dead task should fail")
assert(string.find(start_dead_err, "Attempt to start dead task"), "Error should mention dead task")

local start_later_dead_ok, start_later_dead_err = pcall(function() dead_task:startLater(sleep_period) end)
assert(not start_later_dead_ok, "StartLater on dead task should fail")
assert(string.find(start_later_dead_err, "Attempt to start dead task"), "Error should mention dead task")

local test_results = {
    spawn_ran = false,
    spawn_task_current_valid = false,
    error_task_ran = false,
    send_task_received = nil,
    idle_task_ran = false,
    idle_ran_after_normal = false,
    normal_task_ran = false,
    sleep_task_resumed = false,
    sleep_task_slept = false,
    spawn_later_ran = false,
    spawn_later_waited = false,
    self_killed_sleep_ran = false,
    self_killed_idle_ran = false,
    killed_task_resumed = false,
}

local spawned_task = tasks.spawn(function(x, y)
    mud.output("spawned_task started")
    local curr = tasks.get_current()
    if curr ~= nil and tasks.is_task(curr) then
        test_results.spawn_task_current_valid = true
    end
    test_results.spawn_ran = true
    return x + y, "task_done"
end, 15, 27)

local active_list = tasks.get_tasks()
local found_spawned = false
for _, t in ipairs(active_list) do
    if t == spawned_task then
        found_spawned = true
    end
end
assert(found_spawned, "spawned_task should be present in tasks.get_tasks()")

local error_task = tasks.spawn(function()
    mud.output("error_task started")
    test_results.error_task_ran = true
    error("task failure simulation")
end)

local send_task = tasks.spawn(function()
    mud.output("send_task started")
    local received = tasks.yield()
    mud.output("send_task resumed")
    if received and received[1] then
        test_results.send_task_received = received[1]
    end
end)
send_task:send("sent_data_value")

-- Test idle vs normal task ordering
local idle_task = tasks.spawn(function()
    mud.output("idle_task started")
    tasks.idle()
    mud.output("idle_task resumed")
    if test_results.normal_task_ran then
        test_results.idle_ran_after_normal = true
    end
    test_results.idle_task_ran = true
end)

local normal_task = tasks.spawn(function()
    mud.output("normal_task started")
    test_results.normal_task_ran = true
end)

local sleep_task = tasks.spawn(function()
    mud.output("sleep_task started")
    tasks.sleep(sleep_period)
    mud.output("sleep_task resumed")
    test_results.sleep_task_resumed = true
end)

local spawn_later_task = tasks.spawn_later(sleep_period, function()
    mud.output("spawn_later_task started")
    test_results.spawn_later_ran = true
end)

local self_killed_task = tasks.spawn(function()
    mud.output("self_killed_task started")
    tasks.get_current():kill()
    test_results.self_killed_ran_after_killed = true
end)

local killed_task = tasks.spawn(function()
    mud.output("killed_task started")
    tasks.sleep(sleep_period)
    mud.output("killed_task resumed")
    test_results.killed_task_resumed = true
end)

killed_task:kill()
assert_eq(killed_task.error, nil)
local sleep_killed_ok, sleep_killed_err = pcall(function() killed_task:sleep(sleep_period) end)
assert(sleep_killed_ok, "Sleeping dead task should be ignored")
assert_eq(sleep_killed_err, nil)

local idle_killed_ok, idle_killed_err = pcall(function() killed_task:idle() end)
assert(idle_killed_ok, "Idle dead task should be ignored")
assert_eq(idle_killed_err, nil)

mud.output(#tasks.get_tasks())

-- Timer at 2 ticks before sleep_period to check if slept/spawn_later tasks are still waiting
timer.add(sleep_period - 0.2, 1, function()
    test_results.sleep_task_slept = not test_results.sleep_task_resumed
    test_results.spawn_later_waited = not test_results.spawn_later_ran
end)

-- Wait for tasks to run and assert results.
timer.add(result_wait, 1, function()
    -- make sure task ran and killed itself first
    local sleep_self_killed_ok, sleep_self_killed_err = pcall(function() self_killed_task:sleep(0) end)
    assert(sleep_self_killed_ok, "Sleeping dead task should be ignored")
    assert_eq(sleep_self_killed_err, nil)

    local idle_self_killed_ok, idle_self_killed_err = pcall(function() self_killed_task:idle() end)
    assert(idle_self_killed_ok, "Idle dead task should be ignored")
    assert_eq(idle_self_killed_err, nil)

    mud.output("Checking results...")
    -- Assert spawned_task results
    assert(test_results.spawn_ran, "spawned task should have executed")
    assert(test_results.spawn_task_current_valid, "tasks.get_current() should return valid task inside task execution")
    assert_eq(spawned_task.dead, true)
    assert_eq(spawned_task.success, true)
    assert(spawned_task.value ~= nil, "spawned_task.value should not be nil")
    assert_eq(spawned_task.value[1], 42)
    assert_eq(spawned_task.value[2], "task_done")
    assert_eq(spawned_task.error, nil)

    -- Assert error_task results
    assert(test_results.error_task_ran, "error task should have executed")
    assert_eq(error_task.dead, true)
    assert_eq(error_task.success, false)
    assert(error_task.error ~= nil, "error_task.error should not be nil")
    assert(string.find(tostring(error_task.error[1]), "task failure simulation"), "error message should match")

    -- Assert send_task results
    assert_eq(send_task.dead, true)
    assert_eq(send_task.success, true)
    assert_eq(test_results.send_task_received, "sent_data_value")

    -- Assert idle task results
    assert(test_results.normal_task_ran, "normal task should have executed")
    assert(test_results.idle_task_ran, "idle task should have executed")
    assert(test_results.idle_ran_after_normal, "idle task should run after normal task")

    -- Assert sleep task results
    assert(test_results.sleep_task_slept, "sleep task should still be sleeping at 0.5 second mark")
    assert(test_results.sleep_task_resumed, "sleep task should have executed after sleep period")

    -- Assert spawn_later task results
    assert_eq(spawn_later_task.dead, true)
    assert_eq(spawn_later_task.success, true)
    assert_eq(spawn_later_task.error, nil)
    assert(test_results.spawn_later_waited, "spawn_later task should still be waiting at 0.5 second mark")
    assert(test_results.spawn_later_ran, "spawn_later task should have executed")

    -- Assert self-killed tasks calling sleep or idle
    assert_eq(self_killed_task.dead, true)
    assert_eq(self_killed_task.error, nil)
    --fixme assert(test_results.self_killed_ran_after_killed == false, "task should not run after killed")

    assert_eq(killed_task.dead, true)
    assert_eq(killed_task.error, nil)
    assert_eq(test_results.killed_task_resumed, false, "code after idle on self-killed task should not run")

    -- Assert active tasks list is now empty (all finished)
    local final_tasks = tasks.get_tasks()
    assert_eq(#final_tasks, 0)

    script.reset()
end)
