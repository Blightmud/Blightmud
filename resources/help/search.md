# Search

This modules allows you to easily search for occurences of text
in the output history of your mud session.

##

***search.search(pattern)***
Searches backwards from current position for a matching pattern.

- `pattern` A string to search for. This can be in `regex` format.

##

***search.find_up()***
Searches upwards from current position for the last `search.search(pattern)`
call.

##

***search.find_down()***
Searches downwards from current position for the last `search.search(pattern)`
call.

##

***search.find_last_input()***
Searches upwards from current position for the previous occurence of an input
string. Eg. `> your input`.

##

***search.find_next_input()***
Searches downwards from current position for the next occurence of an input
string. Eg. `> your input`.

##

By default this module is utilized as follows:
- `/search <pattern>` or `/s <pattern>` will initiate a search
- `ctrl + up/down` will let you step through matches
- `ctrl + pgup/pgdn` will step through output lines

Blightmud will do it's best to attempt to hilite matches. However this can
disrupt mud color coding while searching or be disrupted by mud color encoding.

Stepping through output lines can in some cases match mud output if your mud
outputs similar lines.
