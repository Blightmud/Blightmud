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
    - `shortcut`  (string) Optional keyboard-shortcut hint shown in the
                            indicator (e.g. `"F2"` renders as
                            `[F2 - chat]`). Display-only — you still
                            have to wire `blight.bind` separately.
    - `gag_main`  (bool)   When `true`, lines that match this tab's filter
                            will appear ONLY in this tab — they're
                            suppressed from `main`. Default `false`
                            (mirror).
    - `history_lines` (number) Approximate scrollback capacity for this tab,
                            in lines. Omit to keep the same depth as `main`
                            (~32k lines). A smaller value bounds the tab's
                            peak memory — e.g. `2000` caps a busy tab to
                            ~2k lines. Each tab's scrollback grows on demand,
                            so an idle tab costs almost nothing regardless of
                            this setting.

```lua
blight.create_tab("chat",   { label = "chat",   shortcut = "F2" })
blight.create_tab("spam",   { gag_main = true,  history_lines = 2000 })
blight.create_tab("combat", { label = "combat", shortcut = "F3" })
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

***blight.remove_tab(name)***
Remove a tab. The `main` tab is reserved and cannot be removed. Removing the
currently-active tab switches you back to `main` first; the removed tab's
scrollback is discarded. Removing a non-existent tab (or `main`) prints an
error and is otherwise a no-op.

```lua
blight.create_tab("scratch", {})
-- …use it for a while…
blight.remove_tab("scratch")   -- gone; returned to main if it was active
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
blight.add_tab_filter("chat",   "tells you|^\\S+ shouts|^\\S+ chats")
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

***blight.set_tab_shortcut(name, shortcut?)***
Set or clear the keyboard-shortcut hint shown alongside the label in
the tab indicator.

- `name`      Tab to update (including `"main"`).
- `shortcut`  Display string, e.g. `"F2"`. Pass `nil` (or omit) to clear.

When set, the tab renders as `[F2 - chat]` (active), `(F2 - chat·3)`
(inactive with unread), or `(F2 - chat)` (inactive). Display-only —
you still wire the actual keypress with `blight.bind`.

```lua
blight.create_tab("chat", { label = "chat", shortcut = "F2" })
blight.bind("f2", function() blight.switch_tab("chat") end)

-- main can't be passed to create_tab (reserved), so set its shortcut
-- after the fact:
blight.set_tab_shortcut("main", "F1")
blight.bind("f1", function() blight.switch_tab("main") end)
```

##

***blight.output_to(name, ...)***
Send strings as a Line directly into the named tab, bypassing filters.
Equivalent to `blight.output(...)` but targeted at a specific tab. If the
named tab is the active one, the line also renders to the screen.

```lua
blight.output_to("combat", "BOSS DOWN: " .. mob_name)
```

##

***blight.tabs()***
Return a snapshot of all tabs as an array of tables:
`{ {name, label, unread, active}, ... }`. Useful for rendering a custom
indicator inside a script-managed status line, or for debugging at the
in-game `:lua` prompt.

```lua
for _, t in ipairs(blight.tabs()) do
  print(string.format("%s%s (unread=%d)",
    t.active and "*" or " ", t.name, t.unread))
end
```

##

***blight.active_tab()***
Returns the name of the currently-active tab as a string.

##

***blight.set_tab_indicator_position(position)***
Choose where the tab indicator lives.

- `"row"`    — dedicated row above the host topbar (default).
- `"inline"` — tabs render alongside `host:port [tags]` on the topbar
               itself, reclaiming the dedicated row.

Persists to `settings.ron` (equivalent to `/set tab_indicator_inline
true|false`) and refreshes the layout immediately.

```lua
blight.set_tab_indicator_position("inline")
-- or back to the dedicated row:
blight.set_tab_indicator_position("row")
```

##

## Tab indicator row

When two or more tabs exist (i.e. you've called `blight.create_tab` at
least once), Blightmud renders a tab indicator on a dedicated row at the
top of the screen. With shortcut hints set on each tab:

```
═══ Blightmud ══ [F1 - main] │ (F2 - chat·3) │ (F3 - combat) ═════
═ host:port [tags] ════════════════════════════════════════════════
... output area ...
```

Without shortcut hints, the same indicator falls back to
`[main] │ (chat·3) │ (combat)`.

- Active tab: bracketed in bold light-green (`[main]`).
- Non-active tab with unread: yellow parens with `·N` count (`(chat·3)`).
- Non-active tab with no unread: dim parens (`(combat)`).
- Brand prefix (`═══ Blightmud ══`): bold white on a green `═` rule.
  Configurable — set `tab_indicator_brand` to `false` to drop it.

The indicator updates live: switching tabs moves the highlight, and new
matching lines into a non-active tab bump its `·N`.

Three related settings shape the row:

- `tab_indicator_visible` (default `true`) — hide the indicator entirely,
  reclaiming the row for output.
- `tab_indicator_brand`   (default `true`) — when `false`, the row drops
  the `═══ Blightmud ══` prefix and starts with a short `═══` rule before
  the tab list:
  ```
  ═══ [main] │ (chat·3) │ (combat) ═══════════════════════════════════
  ```
- `tab_indicator_inline`  (default `false`) — when `true`, tabs render
  alongside the existing `host:port [tags]` topbar on a single row
  instead of occupying their own row at the top:
  ```
  ═ host:port [tags] [main] │ (chat·3) │ (combat) ════════════════════
  ... output area ...
  ```
  This trades the brand prefix (which the inline row never shows) for
  one reclaimed row of output. From a Lua script you can flip this with
  `blight.set_tab_indicator_position("inline")` (or `"row"` to go back).

```
/set tab_indicator_visible false   # hide indicator entirely
/set tab_indicator_brand   false   # keep dedicated row, drop brand
/set tab_indicator_inline  true    # merge tabs into the host topbar
```

When only the implicit `main` tab exists (no `create_tab` calls have run),
the indicator is hidden automatically — layout matches stock Blightmud.

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

## Routing from triggers

Tab filters and triggers are independent passes over the same inbound
line — a line matched by both fires the trigger AND routes to any tabs
whose filters match. There's no conflict, but also no automatic
coordination: a trigger does not "consume" a line away from the tab
router.

When a pattern is too complex for a single regex filter (or you want
the routing decision to depend on captured groups, state, etc.),
forward the line to a tab from inside the trigger callback using
`blight.output_to(tab, line)`. Pair it with `line:gag(true)` for
"move from main to tab" semantics — see `/help line` for the `Line`
methods.

```lua
-- Forward Imperial-channel chatter to the chat tab, but only when the
-- speaker is on your friends list. Gag from main so it doesn't appear
-- twice.
local friends = { Isadora = true, Break = true }
trigger.add("^<imperial> (\\w+) (.*)", function(m, line)
  if friends[m[2]] then
    blight.output_to("chat", line:line())
    line:gag(true)
  end
end, { gag = false })
```

Compared to `blight.add_tab_filter`:
- Filters are cheaper, regex-only, and have no per-line Lua overhead.
- Triggers are slower but can branch on captures and external state.

Use filters for blanket patterns ("anything that looks like a `tell`")
and triggers for conditional routing ("only when X").

## Example

```lua
-- Set up two tabs alongside main, each with a shortcut hint shown
-- in the indicator. (Main's shortcut is set after the fact because
-- "main" is reserved and not created via create_tab.)
blight.create_tab("chat",   { label = "chat",   shortcut = "F2" })
blight.create_tab("combat", { label = "combat", shortcut = "F3" })
blight.set_tab_shortcut("main", "F1")

blight.add_tab_filter("chat",   "tells you|^\\S+ shouts|^\\S+ chats|^\\S+ gossips")
blight.add_tab_filter("combat", "####### Combat Summary|staggers and falls")

blight.bind("f1", function() blight.switch_tab("main")   end)
blight.bind("f2", function() blight.switch_tab("chat")   end)
blight.bind("f3", function() blight.switch_tab("combat") end)

-- Optional: render tabs alongside the host topbar instead of on their
-- own dedicated row (saves one row of screen real estate).
if blight.set_tab_indicator_position then
  blight.set_tab_indicator_position("inline")
end
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
