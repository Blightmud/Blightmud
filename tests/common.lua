function assert_eq(a, b, msg)
    assert(a == b, string.format("Assertion failed: '%s' != '%s'%s", tostring(a), tostring(b), msg and ". " .. msg or ""))
end

function assert_ge(a, b)
    assert(a >= b, string.format("Assertion failed: '%s' >= '%s'", tostring(a), tostring(b)))
end
