# Top area

The top of the screen is an ordered stack of "top rows" — each one is a
single line built from a bar character, an optional prefix badge, and a
body. Two built-in rows ship by default:

```
tab_indicator   ═══ Blightmud ══ [main] │ (chat·3) │ (combat) ════════
host_status     ═ swmud.org:7777 [tag1][tag2] ═══════════════════════
```

Both are shaped through `/set` toggles and the Lua API below. Scripts
can also append their own rows for character vitals, room exits, etc.

##

## Built-in rows

### `tab_indicator`
Hidden until 2+ tabs are configured. See `/help tabs` for the Lua API
to create tabs, add filters, and switch between them.

- `/set tab_indicator_visible false`   Hide the row entirely.
- `/set tab_indicator_brand   false`   Drop the `═══ Blightmud ══` prefix.
- `/set tab_indicator_inline  true`    Splice tabs into the `host_status`
                                       row instead of using a dedicated row.

### `host_status`
Renders the connected server and any active tags.

- `/set hide_topbar true`              Hide the row entirely.
- `blight.top_line("text")`            Replace `host:port [tags]` with `text`.
- `blight.top_line("")`                Unbroken bar (no text content).
- `blight.top_line(nil)`               Restore the default `host:port [tags]`.

When `tab_indicator_inline=true` AND there are 2+ tabs AND `hide_topbar`
is off, the `host_status` row is decorated with the tabs segment after
its body — the dedicated `tab_indicator` row is suppressed.

```
═ swmud.org:7777 [tag] [main] │ (chat·3) │ (combat) ═══════════════
═ HP 80/100      [main] │ (chat·3) │ (combat) ═════════════════════
```

`top_line` always wins over the default content; tabs always splice in
when inline mode is active. The two settings compose orthogonally.

##

## Lua API for arbitrary rows

```lua
-- Sugar for the host_status row's body slot.
blight.top_line("HP 80/100")
blight.top_line(nil)        -- restore default
blight.top_line("")         -- unbroken bar

-- Full control over any row by index (0-based) or name.
blight.set_top_row("host_status", {
    body = "HP 80/100",   -- "" ⇒ Empty (bar only), string ⇒ Text
    bar_char = "═",       -- any single character
    prefix = "",          -- "" ⇒ no prefix, string ⇒ plain green, table ⇒ styled
    visible = true,       -- show / hide
    name = "host_status", -- rename (rare)
})

-- Restore a built-in row's body to its dynamic default
-- (HostTags for host_status, TabIndicator for tab_indicator).
blight.reset_top_row("host_status")

-- Append a new row. Returns the assigned name.
blight.add_top_row({
    name = "vitals",
    bar_char = "─",
    prefix = { text = " Vitals ", style = "brand" },
    body = "",
    visible = true,
})

-- Remove a Lua-added row. Built-ins refuse removal.
blight.remove_top_row("vitals")

-- Built-in row names, in display order.
local names = blight.builtin_top_rows()  -- { "tab_indicator", "host_status" }
```

### Field reference

`opts` table passed to `set_top_row` / `add_top_row`:

| Field      | Type                  | Behavior                                |
|------------|-----------------------|-----------------------------------------|
| `name`     | string                | Rename the row.                         |
| `bar_char` | string (1 char)       | Fill character. Default `═`.            |
| `prefix`   | string / table        | `""` clears; string ⇒ Plain green badge;|
|            |                       | table `{text, style="plain"|"brand"}`.  |
| `body`     | string                | `""` ⇒ Empty (unbroken bar);            |
|            |                       | non-empty string ⇒ literal text.        |
| `visible`  | boolean               | Show / hide the row.                    |

##

## Tab visual states
- `[name]`     Active tab. Bold light-green.
- `(name·N)`   Inactive tab with N unread lines. Yellow.
- `(name)`     Inactive tab, no unread. Dim.

With a shortcut hint, the body becomes `<shortcut> - <name>`:
`[F1 - main]`, `(F2 - chat·3)`, `(F3 - combat)`. Hints are set via the
`shortcut` opt on `create_tab` or via `blight.set_tab_shortcut`. See
`/help tabs`.

The unread counter clears when you switch to a tab.

##

## See also
- `/help tabs`        Lua API for creating tabs and filters
- `/help settings`    Full list of `/set` toggles
- `/help status_area` Bottom-of-screen status row controls
