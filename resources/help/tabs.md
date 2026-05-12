# Tabs

Blightmud's tabs feature lets a Lua script create named, independently
scrollable output buffers and bind keys to switch between them — typically
to give a "chat-only" or "combat-only" view while still keeping the full
unfiltered scrollback on the implicit `main` tab.

The `main` tab always exists. Calls to `blight.output(...)` (and unfiltered
MUD output) always land in `main`. Filters route a copy of matching lines
into other tabs you create.

##

***blight.create_tab(name, opts?)***
Create a new named tab.

- `name`     A string identifier for the tab. Cannot be `"main"` (reserved).
- `opts`     Optional table:
    - `label`     (string) Display label, defaults to `name`.
    - `gag_main`  (bool)   When `true`, lines that match this tab's filter
                            will appear ONLY in this tab — they're
                            suppressed from `main`. Default `false`
                            (mirror).

```lua
blight.create_tab("chat",   { label = "chat"   })
blight.create_tab("combat", { label = "combat", gag_main = false })
```

##

***blight.switch_tab(name)***
Switch the active tab. The screen redraws with the destination tab's
scrollback. Per-tab scroll position is preserved across switches.

```lua
blight.bind("f1", function() blight.switch_tab("main")   end)
blight.bind("f2", function() blight.switch_tab("chat")   end)
blight.bind("f3", function() blight.switch_tab("combat") end)
```

##

***blight.add_tab_filter(name, pattern)***
Append a regex-string filter to a tab. Inbound MUD lines whose
ANSI-stripped text matches `pattern` are routed into this tab in
addition to `main` (subject to the tab's `gag_main` setting).

- `name`     Tab to filter into.
- `pattern`  A Rust regex string. Multiple filters may be added per tab;
              they OR together.

```lua
blight.add_tab_filter("chat",   "tells you|^%S+ shouts|^%S+ chats")
blight.add_tab_filter("combat", "####### Combat Summary|staggers and falls")
```

##

***blight.add_tab_exclude_filter(name, pattern)***
Append a regex-string *exclude* to a tab. Any inbound line whose
ANSI-stripped text matches `pattern` is **not** routed into this tab —
even when one of the tab's `add_tab_filter` patterns also matches.
Excludes are a blocklist hole inside a broad include rule.

Rust's `regex` crate has no lookaround, so a single include regex can't
express "match X but not Y". `add_tab_exclude_filter` lets you compose
that across two regexes.

- `name`     Tab to apply the exclude to.
- `pattern`  A Rust regex string. Multiple excludes may be added per
              tab; any one match vetoes routing.

```lua
-- Include every "<Name> says" line in the chat tab…
blight.add_tab_filter("chat", "^[A-Z][a-zA-Z0-9]+ says\\b")
-- …but skip pronouns and known generic-NPC labels we don't want there.
blight.add_tab_exclude_filter("chat", "^(He|She|It|They|We) says\\b")
blight.add_tab_exclude_filter("chat", "^(Smuggler|Salesman|Soldier) says\\b")
```

Excludes apply only to filter-routed lines; `blight.output_to` ignores
them, since that path is for explicit direct routing.

##

***blight.set_tab_label(name, label)***
Update the display label for a tab. Useful for live indicators like
`"chat (3 new)"`.

##

***blight.output_to(name, ...)***
Send strings as a Line directly into the named tab, bypassing filters.
Equivalent to `blight.output(...)` but targeted at a specific tab. If the
named tab is the active one, the line also renders to the screen.

```lua
blight.output_to("combat", "BOSS DOWN: " .. mob_name)
```

##

## Behavior notes

- **Mirror by default.** A line that matches a tab's filter appears in
  *both* the tab and `main`. Set `gag_main = true` on the tab to make
  matches exclusive to that tab.
- **Multiple matching tabs.** If a single line matches filters on multiple
  tabs, it appears in all of them. `gag_main = true` on any matching tab
  suppresses the line from `main`.
- **Per-tab scroll position** is preserved across switches.
- **Unread counters** track how many lines have arrived in each non-active
  tab since you last viewed it. Switching to a tab clears its counter.
- **Scripts using only `blight.output(...)`** continue to work unchanged —
  their output goes to `main`, just like before tabs existed.

## Example

```lua
-- Set up two tabs alongside main
blight.create_tab("chat",   { label = "chat"   })
blight.create_tab("combat", { label = "combat" })

blight.add_tab_filter("chat",   "tells you|^%S+ shouts|^%S+ chats|^%S+ gossips")
blight.add_tab_filter("combat", "####### Combat Summary|staggers and falls")

blight.bind("f1", function() blight.switch_tab("main")   end)
blight.bind("f2", function() blight.switch_tab("chat")   end)
blight.bind("f3", function() blight.switch_tab("combat") end)
```

## Feature detection

To write a script that works on Blightmud builds with AND without tabs,
guard the calls with a presence check:

```lua
if blight.create_tab then
  -- tabs available, configure them
else
  -- fallback / no-op
end
```
