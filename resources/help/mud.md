# Mud

This module allows you to interact with the active mud. Such as capturing lines
and sending input and output.

##

***mud:add_output_listener(callback)***

This method will add a listener for mud output. All lines received from the mud
will be provided to the registered callback for processing. This is one of the
core systems in blightmud and what's being used for the trigger system to
operate. For a general user this method should not be needed.

The provided callback will receive one argument. A line object. See `/help
line` for information about this object.

##

***mud:add_input_listener(callback)***

This method will add a listener for user input to the mud. All input lines from
the user will be sent to this callback.  This is one of the core systems in
blightmud and what's being used for the alias system to operate. For a general
user this method should not be needed.

The provided callback will receive one argument. A line object. See `/help
line` for information about this object.
