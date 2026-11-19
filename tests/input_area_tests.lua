require "tests.common"

-- Exercises the multi-line input area's Lua surface end to end in headless
-- mode. Headless screens report an input height of 1 regardless of what is
-- requested, so the assertions here cover the API contract — clamping,
-- get-returns-set, row arithmetic — rather than rendering, which is not
-- reachable without a terminal.

script.on_reset(function()
    blight.quit()
end)

-- ---- blight.input_height ------------------------------------------------
assert_eq(blight.input_height(), 1)   -- default leaves today's layout alone

assert_eq(blight.input_height(3), 3)  -- set returns what was set
assert_eq(blight.input_height(), 3)   -- and reads back

assert_eq(blight.input_height(0), 1)     -- zero would leave nowhere to type
assert_eq(blight.input_height(500), 10)  -- bounded so output stays renderable

-- Ask for five rows. A headless screen cannot give them, and the deferred
-- check below asserts that what reads back is what the screen granted.
blight.input_height(5)

-- ---- prompt row arithmetic ----------------------------------------------
prompt.set("")
assert_eq(prompt.row_count(), 1)
assert_eq(prompt.cursor_row(), 1)

-- Row breaks round-trip through prompt.set/get rather than being emitted as
-- raw control characters, which is what used to happen.
prompt.set("one\ntwo\nthree")
assert_eq(prompt.get(), "one\ntwo\nthree")
assert_eq(prompt.row_count(), 3)

-- A buffer with no row breaks is one row, which is what makes the up/down
-- bindings fall through to command history exactly as before.
prompt.set("no rows here")
assert_eq(prompt.row_count(), 1)
assert_eq(prompt.cursor_row(), 1)

-- ---- the input_auto_expand setting exists and defaults off ---------------
assert_eq(settings.get("input_auto_expand"), false)
settings.set("input_auto_expand", true)
assert_eq(settings.get("input_auto_expand"), true)
settings.set("input_auto_expand", false)

-- ---- set_newline_keys ---------------------------------------------------
-- Retargeting the whole group must not error, including the empty case that
-- disables row insertion entirely. This is what catches the bind/unbind
-- normalisation asymmetry: unbind does no case folding, so the helper has to
-- unbind exactly the strings bind stored.
set_newline_keys({ "alt-enter" })
set_newline_keys({ "ctrl-o", "\x1b[13;2u" })
set_newline_keys({})
set_newline_keys({ "ctrl-o", "alt-enter", "\x1b\r", "\x1b\n" })

-- ---- Deferred: let the queued events drain, then finish ------------------
timer.add(1, 1, function()
    -- Five rows were requested above, but a headless screen has one. This is
    -- why input_height() is a report of what the screen granted rather than an
    -- echo of the request: NAWS sends this number to the MUD, so an echo would
    -- advertise a window height the screen never had.
    assert_eq(blight.input_height(), 1)
    script.reset()
end)
