# Triggers

Triggers are used to automatically execute Lua code when certain data is
received from the MUD

## Trigger Options

Triggers can be created with various options. To that end, a table of options
is passed in at creation.
The table contains the following:

- `gag`     Gag (don't print) the matched line
- `raw`     Match on the raw MUD line (ANSI escape sequences intact)
- `prompt`  Match against the prompt instead of regular lines
- `count`   Number of times this trigger will match before it is automatically
            removed (default: `nil` = infinite)
- `enabled` Whether the trigger is enabled or not (default `true`)

## Module functions

***trigger.add(regex, options, callback)***
Create a new trigger

- `regex`    A string describing the regex to match against incoming data
- `options`  A table of options (See `Trigger Options` at the top
- `callback` Lua function to call when match is found. Parameters are a table
             of matches and the line that got matched (See `/help line`)
- Returns a `Trigger` object (see below)

##

***trigger.get(id)***
Gets a trigger by its ID. If trigger groups are used, all groups will be searched

- `id` ID of the trigger to find
- Returns the `Trigger` with the given ID or `nil` if not found

##

***trigger.getGroup(id)***
Gets a trigger group by itd ID.

- `id` ID of the trigger group
- Returns the `TriggerGroup` with the given ID or `nil` if not found

##

***trigger.remove(id)***
Removes the trigger with the given ID. If the trigger is in multiple groups, it
will be removed from all of them

- `id` ID of the trigger to remove

##

***trigger.clear()***
Deletes all triggers. If trigger groups are used, they will all be cleared

##

***trigger.addGroup()***
Creates a new trigger group

- Returns the newly created `TriggerGroup`

## Trigger

The trigger object represents an individual trigger. It has the following attributes:

- `regex`    A regex object that is matched against (See `/help regex`)
- `callback` The callback function
- `gag`      See `Trigger Options`
- `raw`      See `Trigger Options`
- `prompt`   See `Trigger Options`
- `count`    See `Trigger Options`
- `enabled`  See `Trigger Options`
- `id`       The ID of the trigger

Do not change the ID of a trigger.

##

***trigger.Trigger.new(regex, options, callback)***
Creates a new trigger object. Note that the trigger will not work without being
part of at least one trigger group

- `regex`    A string describing the regex to match against incoming data
- `options`  A table of options (See `Trigger Options` at the top
- `callback` Lua function to call when match is found. Parameters are a table
             of matches and the line that got matched (See `/help line`)
- Returns a `Trigger` object

##

***trigger.Trigger.isTrigger(object)***
Tests whether a given table is a trigger

- `object` The table to test
- Returns `true` if `object` is a `Trigger`. `false` otherwise.

##

***Trigger:enable()***
Enabled the trigger

##

***Trigger:disable()***
Disables the trigger

##

***Trigger:setEnabled(enabled)***
Sets the `enabled` status of the trigger

- `enabled` Whether the trigger should be enabled

##

***Trigger:isEnabled()***
Returns whether the trigger is enabled

- Returns `true` if the trigger is enabled. `false` otherwise.

##

***Trigger:checkLine(line)***
Runs the trigger against a given line. If the trigger matches, the callback
will be executed, count will be lowered, etc.
Mainly used internally.

- `line` A `Line` object to check (See `/help line`)

## TriggerGroup
A `TriggerGroup` represents a collection of triggers. By default, there is only one trigger group available.

It has the following attributes:

- `id`       The ID of the trigger group
- `triggers` A table of triggers contained in this group

Do not modify any of these attributes.

##

***trigger.TriggerGroup.new(id)***
Creates a new `TriggerGroup` object. Note that the trigger group will not be
registered internally and thus will not function. Prefer to use `trigger.addGroup`

- `id` The ID of the new `TriggerGroup`
- Returns a `TriggerGroup`

##

***TriggerGroup:add(regex_or_trigger, options, callback)***
Adds a trigger to the group.
If `regex_or_trigger` is a `Trigger` object, then it will be added to the group
and the other two parameters are ignored. Otherwise, a new trigger will be
created.

- `regex_or_trigger` The trigger object to add or a regex string to create a new trigger
- `options`          Options for the new trigger
- `callback`         Callback for the new trigger

##

***TriggerGroup:get(id)***
Gets a trigger from the group

- `id` ID of the trigger
- Returns the `Trigger` if found in the group. `nil` otherwise.

##

***TriggerGroup:getTriggers()***
Returns a table of all the triggers in the group.

- Returns a shallow copy of the `triggers` attribute

##

***TriggerGroup:remove(id)***
Removes a trigger from the group

- `id` ID of the trigger to remove

##

***TriggerGroup:clear()***
Removes all triggers from the group

##

***TriggerGroup:checkLine(line)***
Dispatches `Trigger:checkLine` calls to all contained triggers.
Mainly used internally.

- `line` The `Line` object to pass to the triggers (See `/help line`)
