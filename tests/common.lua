function assert_eq(a, b)
    assert(a == b, string.format("Assertion failed: '%s' != '%s'", tostring(a), tostring(b)))
end

function assert_ge(a, b)
    assert(a >= b, string.format("Assertion failed: '%s' >= '%s'", tostring(a), tostring(b)))
end
