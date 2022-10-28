# TTYPE

Blightmud will negotiate TTYPE when available on the current mud.
The defaults are:

- `IAC SB TTYPE SEND "BLIGHTMUD" IAC SE`
- `IAC SB TTYPE SEND "$TERM" IAC SE`
- `IAC SB TTYPE SEND "MTTS 271" IAC SE`

Where $TERM is the value of your TERM environment variable.

You may override TERM and MTTS with the following functions.
More info about MTTS can be found at [](https://tintin.mudhalla.net/protocols/mtts/).

Modifications to the TTYPE MTTS value must be made before you connect to a mud.
Once Blightmud and the server have agreed to use TTYPE the negotation stack
that Blightmud will send is generated. You need to make your changes before
this point.

If Blightmud is in reader mode or TTS is enabled `MTTS_SCREEN_READER` will be
added to the MTTS value automatically. This will not happen if
`MTTS_SCREEN_READER` has been manually toggled through either
`ttype.add_option(MTTS_SCREEN_READER)` or
`ttype.rem_option(MTTS_SCREEN_READER)`.

## MTTS Options and the default:

The following globals are defined for easier MTTS scripting:
```lua
-- MTTS options:
ttype.MTTS_ANSI           =   0x001 -- Ansi support
ttype.MTTS_VT100          =   0x002 -- VT100 support
ttype.MTTS_UTF8           =   0x004 -- UTF-8 Support
ttype.MTTS_256_COLOR      =   0x008 -- 256 color support
ttype.MTTS_MOUSE_TRACKING =   0x010 -- Mouse tracking support
ttype.MTTS_OSC_COLOR      =   0x020 -- OSC color palette support (true color)
ttype.MTTS_SCREEN_READER  =   0x040 -- Client using screen reader
ttype.MTTS_PROXY          =   0x080 -- This is a proxy connection
ttype.MTTS_TRUE_COLOR     =   0x100 -- True color support
ttype.MTTS_MNES           =   0x200 -- Mud New Env Standard enabled
ttype.MTTS_MSLP           =   0x400 -- Mud Server Link Protocol enabled

-- Default MTTS value:
local mtts = 0x0
mtts = mtts | ttype.MTTS_VT100
mtts = mtts | ttype.MTTS_ANSI
mtts = mtts | ttype.MTTS_UTF8
mtts = mtts | ttype.MTTS_256_COLOR
mtts = mtts | ttype.MTTS_TRUE_COLOR
```

##

***ttype.set_term(new_val)***
Will override the default for TERM with your provided string.

- `new_val` The new TERM value. Eg. "xterm-256color"

##

***ttype.set_mtts(new_mtts)***
Will override the MTTS value with the provided number.

- `new_mtts` The new value for MTTS. Eg. 137

##

***ttype.add_option(opt)***
Add an MTTS option to the current MTTS value. See options above

- `opt` The option to add. Eg. `TTYPE_OPT_MTTS_256C`

##

***ttype.rem_option(opt)***
Remove an MTTS option from the current MTTS value. See options above

- `opt` The option to remove. Eg. `TTYPE_OPT_MTTS_256C`
