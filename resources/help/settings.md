# Settings

There are an increasing number of settings that can be controlled in blightmud.
Settings are stored in `$CONFIGDIR/settings.ron` and can be either edited by
hand or toggled using `/set`. Changing some settings require a restart.

Settings are toggled as follows:

- `/set <setting>`           Shows a settings and its current value
- `/set <setting> on/off`    Toggles a setting on or off
- `/settings`                Show current value of all settings

Available settings are:

- `logging_enabled`     See `/help logging`
- `mouse_enabled`       Experimental mouse scrolling support. Requires restart.
                        (See additional details below)
- `save_history`        Save your last 100 commands to disk.
- `command_search`      Makes command history context aware (See info below for details)
- `smart_history`       Enable smart command history (See info below for details)
- `confirm_quit`        Ask for confirmation before quitting Blightmud when pressing `ctrl-c`.
- `scroll_split`        Split screen when scrolling
- `scroll_lock`         Set scroll position at start of text when showing long help files
- `tts_enabled`         Enable tts (only if compiled with TTS)
- `reader_mode`         Switches to a screen reader friendly TUI. (Does not support `status area`.)
- `hide_topbar`         Toggles the topbar
- `echo_input`          Toggles whether user input is echoed on-screen with a `> ` prefix.

##

***mouse_enabled***
This mode will capture mouse events to the terminal in order to allow mouse
scroll-wheel scrolling. One of the more noticable effects of this is that mouse
text selection won't work in blightmud. So far holding `shift` (or `cmd` on
some Apple devices) will allow you to select text using the mouse as normal on
most terminal emulators (every one we have encountered so far).

***command_search***
Makes command history stepping context aware.

If you type something in the prompt and then hit `up` (`history.previous_command()`)
you will only find results that are prefixed with what you typed first.

Eg. Type `sc`, hit `up`. Results can be `scent`, `score`, `scan`.

***smart_history***
Makes the command history slightly more useful.

- A command will never appear twice in the history
- First history command will always be the last typed command
- Entering a previously entered command will shift it to the front of the history.
