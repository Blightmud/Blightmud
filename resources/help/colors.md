# Lua output colors

Colors can be used to add some 'pizzazz' to your custom output when you are
mudding. The following colors are available:

## Foreground colors

- `C_RESET`
- `C_BOLD`
- `C_BLACK`
- `C_RED`
- `C_GREEN`
- `C_YELLOW`
- `C_BLUE`
- `C_MAGENTA`
- `C_CYAN`
- `C_WHITE`

## Background colors

- `BG_BLACK`
- `BG_RED`
- `BG_GREEN`
- `BG_YELLOW`
- `BG_BLUE`
- `BG_MAGENTA`
- `BG_CYAN`
- `BG_WHITE`

## Bright foreground colors

- `C_BBLACK`
- `C_BRED`
- `C_BGREEN`
- `C_BYELLOW`
- `C_BBLUE`
- `C_BMAGENTA`
- `C_BCYAN`
- `C_BWHITE`

## Bright background colors

- `BG_BBLACK`
- `BG_BRED`
- `BG_BGREEN`
- `BG_BYELLOW`
- `BG_BBLUE`
- `BG_BMAGENTA`
- `BG_BCYAN`
- `BG_BWHITE`

## Reset all colors

- `C_RESET`

Colors are best used with `blight.output(msg)`
```lua
blight.output(C_RED .. "Red message" .. C_RESET)
```

Take not that you are at the mercy of your terminals configuration when it
comes to these colors.  If they don't appear as you expect that's something
you'll have to take up with your terminal.

The colors listed above are basic ansi escape codes hidden behind a variable.
If you want to print different things feel free to create your own setups.

```lua
C_ALERT = "\x1b[37;41m"
C_INFO = C_WHITE .. BG_GREEN
blight.output(C_ALERT .. "Panic! The white rabbit is here" .. C_RESET)
blight.output(C_INFO .. "Ok, he left. Everbody relax!" .. C_RESET)
```

There is a lot more you can do with ansi escapes in a terminal. All your ideas should probably work through `blight.output(msg)`

## cformat utility

Concatenating variables everywhere isn't the best coding experience. That's
why we provide the `cformat` utility function, which will return a formatted
string you can use.

```lua
blight.output(cformat('This is some <red>red<reset> text.'))
blight.output(cformat('This is some <red:blue>red on blue<reset> text.'))
blight.output(cformat('This is some <yellow>%s<reset> text.', 'formatted'))
```
