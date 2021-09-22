# History

Module used to interact with command history

## Context aware history
If you enable the setting `command_search` then these functions will step
through commands that are prefixed with what's already written in the prompt.
If anything is written. Otherwise it will behave as normal.

##

***history.previous_command()***
Will shift the current prompt to the previous command.

##

***history.next_command()***
Will shift the current prompt to the next command.
This requires that you previously navigated up through the history.
