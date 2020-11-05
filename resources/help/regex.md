# Regular expressions

Blightmuds lua script offers access to powerful regular expressions. These can
be created and used to match and replace content in strings.

##

***regex.new(pattern)***
Creates a new regular expression.

##

***regex:test(string)***
Checks if a string matches the regular expression.
Returns `true` or `false`.

```lua
local re = regex.new("^This is a \\w+ line$")
assert(re:test("This is a good line"))
assert(re:test("This is a bad line"))
assert(not re:test("This is a good and bad line"))
```

##

***regex:match(string)***
Matches a regex against a string and returns the capture groups in a table.

```lua
local re = regex.new("^a (\\w+) string$")

local matches = re:match("a good string")
assert(matches[1] == "a good string")
assert(matches[2] == "good")

assert(re:match("12345") == nil)
```

##

***regex:replace(string, replace[, count])***
Replaces non overlapping matches of a regex in a string with the provided
replacement.

- `string`  The string to match
- `replace` The replacement pattern
- `count`   Number of replacements to perform from left to right
          Not providing a count or setting it to 0 will replace all occurences.

Returns a new string with matches replaced

```lua
local re = "(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})"
local original = "2012-03-14, 2013-01-01 and 2014-07-05"
assert(re:replace(original, "$m/$d/$y") == "03/14/2012, 01/01/2013 and 07/05/2014")
assert(re:replace(original, "$m/$d/$y", 1) == "03/14/2012, 2013-01-01 and 2014-07-05")
assert(re:replace(original, "$m/$d/$y", 2) == "03/14/2012, 01/01/2013 and 2014-07-05")

local re = "(\\d{4})-(\\d{2})-(\\d{2})"
assert(re:replace(original, "$2/$3/$1") == "03/14/2012, 01/01/2013 and 07/05/2014")
```
