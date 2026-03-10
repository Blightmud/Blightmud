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

-- History navigation
blight.bind("up", history.previous_command)
blight.bind("down", history.next_command)
blight.bind("ctrl-p", history.previous_command)
blight.bind("ctrl-n", history.next_command)
