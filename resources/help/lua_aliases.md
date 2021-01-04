# Aliases

Aliases allow you to trigger a callback function when a certain command is typed.

## Creating an Alias

***alias.add(regex, callback) -> id***
Creates an alias which when triggered runs the provided callback function.

- `regex`    A regular expression to match as the command name.
- `callback` A Lua function that gets called when the regex is matched.
- Returns an Alias object (see below)

##

***alias.get(id)***
Fetches an alias by ud

- `id`      The id of the alias to get
- Returns the Alias with the given id or `nil` if not found

##

***alias.get_group(id)***
Gets an alias group by id

- `id`  The id of the alias group
- Returns the `AliasGroup` with the give id or `nil` if not found

##

***alias.remove(id)***
Remove the alias with the give ind. If the alias exists in multiple groups it
will be removed from all of them.

- `id`  The id of the alias to remove

##

***alias.clear()***
Remove all aliases. If groups are being used then they will all be cleared.

##

***alias.add_group()***
Creates a new alias group

- Returns the newly created `AliasGroup`

## Alias

The alias object represents an individial alias. It has the following
attributes:

- `regex`       A regex object used for matching (See `/help regex`)
- `callback`    The callback function
- `enabled`     Enabled status of the alias
- `id`          The id of the alias

Do not change the id of an Alias.

##

***alias.Alias.new(regex, callback)***
Creates a new alias object. Note that this has no effect if it's not a part of
an alias group.

- `regex`       A regular expression as a string
- `callback`    The callback function for this alias
- Returns a `AliasObject`

##

***alias.Alias.is_alias(object)***
Tests wether a given table is an alias.

- Returns true or false

##

***Alias:enable()***
Enable the alias

##

***Alias:disable()***
Disable the alias

##

***Alias:set_enabled(enabled)***
Sets the `enabled` status of the alias

- `enabled`     True or false

##

***Alias:is_enabled()***
Check if an alias is enabled

- Returns true or false

##

***Alias:check_line(line)***
Runs the alias against a given line. If the alias matches, the callback
will be executed, count will be lowered, etc.
Mainly for internal use.

- `line`    A `Line` object (See `/help line`)

## AliasGroup
An `AliasGroup` represents a collection of aliases. By default, there is only
one alias group available.

It has the following attributes:

- `id`      The id of the alias group
- `aliases` A table of aliases contained in the group.

Don't modify these attributes directly

##

***alias.AliasGroup.new(id)***
Creates a new `AliasGroup` object. Note that the alias group will not be
registered internally and thus will not function. Prefer to use `alias.add_group`

- `id`  The id of the new `AliasGroup`
- Returns an `AliasGroup`

##

***AliasGroup:add(regex_or_alias[, callback])***
Adds an alias to the group.

The second argument is optional if the first is an `Alias`.

- `regex_or_alias`  An alias object or a regex string to create a new alias
- `callback`        A callback function (optional)

##

***AliasGroup:get(id)***
Gets an alias from the group

- `id`  The id of the alias to get
- Returns the `Alias` if it exists in the group or `nil`

##

***AliasGroup:get_aliases()***
Returns a table of all aliases in the group.

- Returns the aliases contained in the group

##

***AliasGroup:clear()***
Removes all aliases from the group

##

***AliasGroup:check_line(line)***

Dispatches `Alias:check_line` calls to all contained aliases.
Mainly used internally.
