require "tests.common"

script.on_reset(function ()
    blight.quit()
end)

-- Test core.exec
-- Assumes running on a system where "echo" and "false"
-- are executables in $PATH, e.g. from GNU coreutils
assert(core.exec("echo test"):stdout() == "test\n")
assert(core.exec({"false"}):code() == 1)
assert(core.exec({"echo", "one", "two"}):stdout() == "one two\n")

timer.add(1, 1, function ()
    script.reset()
end)
