require "tests.common"

-- Exercises the tab / top-area Lua API end to end in headless integration
-- mode. Each mutating call queues an event that the main loop applies to the
-- session's TabSet / screen; introspection (`blight.tabs`, `blight.active_tab`)
-- is deferred to a timer so the queued events have drained first.

script.on_reset(function ()
    blight.quit()
end)

-- ---- Tab creation + configuration -------------------------------------
blight.create_tab("chat", { label = "Chat", shortcut = "F2" })
blight.create_tab("combat", { gag_main = true })
blight.add_tab_filter("chat", "tells you")
blight.add_tab_exclude_filter("chat", "^He ")
blight.set_tab_label("combat", "Combat")
blight.set_tab_shortcut("combat", "F3")
blight.set_tab_shortcut("combat", nil)   -- clear an existing hint
blight.output_to("chat", "Bob tells you: hello")
blight.switch_tab("chat")

-- ---- Top-area rows ----------------------------------------------------
blight.top_line("custom status")
blight.set_top_row("host_status", { body = "HP 80/100", bar_char = "-" })
blight.set_top_row(1, { prefix = "Lead" })            -- index selector + string prefix
blight.add_top_row({
    name = "vitals",
    bar_char = "-",
    prefix = { text = " Vitals ", style = "brand" },
    body = "",
})
blight.reset_top_row("host_status")                   -- name selector
blight.reset_top_row(0)                               -- index selector
blight.remove_top_row("vitals")

-- top_rows() reports the stable built-in row names synchronously.
assert(#blight.top_rows() == 2, "top_rows returns the two built-ins")

-- ---- Deferred introspection (queued events have drained by now) -------
timer.add(1, 1, function ()
    local tabs = blight.tabs()
    assert(#tabs >= 3, "expected at least main + chat + combat")

    local chat
    for _, t in ipairs(tabs) do
        if t.name == "chat" then chat = t end
    end
    assert(chat ~= nil, "chat tab should exist")
    assert(chat.label == "Chat", "chat label comes from create opts")
    assert(chat.shortcut == "F2", "chat shortcut comes from create opts")

    assert(#blight.active_tab() > 0, "active_tab returns a name")

    script.reset()
end)
