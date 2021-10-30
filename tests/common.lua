
function assert_eq(a, b)
    assert(a == b, string.format("Assertion failed: '%s' != '%s'", tostring(a), tostring(b)))
end
