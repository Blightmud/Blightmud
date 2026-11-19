local function bind(cmd, event)
    blight.bind(cmd, function()
        blight.ui(event)
    end)
end

bind("alt-b", "step_word_left")
bind("ctrl-left", "step_word_left") -- Ctrl + left
bind("alt-f", "step_word_right")
bind("ctrl-right", "step_word_right") -- Ctrl + right
bind("alt-backspace", "delete_word_left")
bind("alt-d", "delete_word_right")
bind("ctrl-a", "step_to_start")
bind("ctrl-b", "step_left")
bind("ctrl-e", "step_to_end")
bind("ctrl-f", "step_right")
bind("ctrl-d", "delete_right")
bind("ctrl-h", "delete")
bind("ctrl-k", "delete_to_end")
bind("ctrl-u", "delete_from_start")

-- Scrolling
bind("home", "scroll_top")
bind("end", "scroll_bottom")
bind("pageup", "scroll_up")
bind("pagedown", "scroll_down")

-- ctrl + up/down
blight.bind("ctrl-up", search.find_up)
blight.bind("ctrl-down", search.find_down)

-- ctrl + pgup/pgdn
blight.bind("\x1b[5;5~", search.find_last_input)
blight.bind("\x1b[6;5~", search.find_next_input)
blight.bind("ctrl-s", function()
    tts:stop()
end)

-- Insert a row in the input area.
--
-- ctrl-o is the primary binding because it needs no terminal configuration
-- anywhere, macOS included. alt-enter only works once "Option as Meta" is
-- enabled, which Terminal.app and iTerm2 both ship with *off*, and some
-- braille and keyboard-emulation stacks never deliver it at all. ctrl-j is
-- not usable: it is byte-identical to Enter.
--
-- The raw escape forms are bound too because some terminal/tmux combinations
-- deliver Option+Enter as unrecognised bytes rather than as an Alt key, in
-- which case the named binding never fires but this one does.
--
-- Retarget the whole group in one call with set_newline_keys{...}.
set_newline_keys({ "ctrl-o", "alt-enter", "\x1b\r", "\x1b\n" })

-- History navigation
--
-- Up and Down move within the input area when there is somewhere to move to,
-- and fall through to command history at the edges. With no row breaks in the
-- buffer this is exactly the previous behaviour. ctrl-p and ctrl-n stay bound
-- to history unconditionally as the always-available escape hatch.
blight.bind("up", function()
    if prompt.cursor_row() > 1 then
        blight.ui("step_up")
    else
        history.previous_command()
    end
end)
blight.bind("down", function()
    if prompt.cursor_row() < prompt.row_count() then
        blight.ui("step_down")
    else
        history.next_command()
    end
end)
blight.bind("ctrl-p", history.previous_command)
blight.bind("ctrl-n", history.next_command)

-- Toggle tag rendering
blight.bind("ctrl-t", function()
    blight.show_tags(not blight.show_tags())
end)
