# Tags

Tags are visual markers that can be attached to output lines. When tag rendering
is enabled, each line is prefixed with a colored symbol (or two plain spaces if
the line has no tag color set). Tags can be used to visually group, highlight,
or filter lines in the output view.

## Enabling tag rendering

Tag rendering is off by default. Enable it with `blight.show_tags()`:

```lua
blight.show_tags(true)   -- enable
blight.show_tags(false)  -- disable
blight.show_tags()       -- returns current state (bool)
```

A convenient keybinding to toggle tags (default):

```lua
blight.bind("ctrl-t", function()
    blight.show_tags(not blight.show_tags())
end)
```

## Setting tags on lines

Tags are set on `Line` objects, typically inside a trigger callback. Three
properties make up a tag:

***line:tag_color(color) -> string***

Get or set the ANSI color used to render the tag symbol. Setting a color
activates the tag. When no color is set the tag symbol is not rendered.

- `color`  An ANSI escape sequence, e.g. `"\x1b[31m"` or a constant `C_RED`.

***line:tag_symbol(char) -> string***

Get or set the character used as the tag symbol. Defaults to `┃` (U+2503,
BOX DRAWINGS HEAVY VERTICAL). Only rendered when a tag color is set.

***line:tag_key(key) -> string***

Get or set an arbitrary string key associated with this line's tag. Useful for
filtering lines.

### Example

```lua
trigger.add("^You receive", {}, function (_, line)
    line:tag_color(C_GREEN)
    line:tag_symbol("$")
    line:tag_key("income")
end)

trigger.add("^You spend", {}, function (_, line)
    line:tag_color(C_RED)
    line:tag_symbol("$")
    line:tag_key("expense")
end)
```

## Filtering lines by tag

Tag filters hide lines whose tag matches the given value. Filters apply to
existing lines in the view as well as new ones. Multiple filters can be active
simultaneously — a line is hidden if *any* filter matches it.

***blight.filter_tag_color(color)***
Hide lines whose tag color equals `color`. Pass `nil` to clear.

***blight.filter_tag_key(key)***
Hide lines whose tag key equals `key`. Pass `nil` to clear.

***blight.filter_tag_symbol(symbol)***
Hide lines whose tag symbol equals `symbol`. Pass `nil` to clear.

***blight.filter_tag_reverse(val) -> bool***
Get or set the reverse flag. When `false` (default), matching lines are hidden.
When `true`, non-matching lines are hidden — only lines that match the filter
are shown. Pass no argument to get the current value without changing it.

***blight.filter_tag_reset()***
Clear all active tag filters, including the reverse flag.

### Example

```lua
-- Hide all income lines
blight.filter_tag_key("income")

-- Show them again
blight.filter_tag_key(nil)

-- Show ONLY combat lines (hide everything else)
blight.filter_tag_key("combat")
blight.filter_tag_reverse(true)

-- Back to normal
blight.filter_tag_reset()
```

## See also

- `/help line`      — Full Line object reference
- `/help blight`    — Full blight module reference
- `/help bindings`  — Keybinding examples
- `/help colors`    — Available color constants
