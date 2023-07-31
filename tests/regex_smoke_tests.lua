require "tests.common"

script.on_reset(function ()
    blight.quit()
end)

local test_regex = function(pattern, line)
    local r = regex.new(pattern)
    return r:test(line)
end

-- Test anchors
assert(test_regex("^anchor", "anchor"))
assert(not test_regex("^anchor", "some anchor"))
assert(not test_regex("^anchor$", "anchor test"))
assert(not test_regex("^anchor$", "test anchor"))
assert(test_regex("^anchor$", "anchor"))

-- Test alphanum wildcard
assert(test_regex("^wildcard \\w+", "wildcard something"))
assert(test_regex("^wildcard \\w+", "wildcard something other"))
assert(not test_regex("^wildcard \\w+", "wildcard $something"))

-- Test digit wildcard
assert(test_regex("^\\d+$", "42"))
assert(not test_regex("^\\d+$", "and42"))
assert(test_regex("[0-9]+", "1942"))

-- Test digit and char
assert(test_regex("^[a-z\\d]+", "42something42"))

-- Test global wildcard
assert(test_regex("^.+$", "and42*$%&"))
assert(test_regex("^.*$", ""))
assert(test_regex("^.*$", "dhsjaklh 782173891 !#¤%¤#"))

-- Test char limit regex
assert(test_regex("\\d{2,4}", "1234"))
assert(test_regex("\\d{2,4}", "123"))
assert(test_regex("\\d{2,4}", "12"))
assert(not test_regex("\\d{2,4}", "1"))
assert(not test_regex("^\\d{2,4}$", "12345"))

timer.add(1, 1, function ()
    script.reset()
end)
