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

If Blightmud is in reader mode or TTS is enabled this will be appended to the
MTTS value automatically. You can disabled this beahvior if you want using the
function `ttype.auto_detect_reader_mode`.

## MTTS Options and the default:

The following globals are defined for easier MTTS scripting:
```lua

-- MTTS options:
TTYPE_OPT_MTTS_ANSI =   0x001 -- Ansi support
TTYPE_OPT_MTTS_VT100 =  0x002 -- VT100 support
TTYPE_OPT_MTTS_UTF8 =   0x004 -- UTF-8 Support
TTYPE_OPT_MTTS_256C =   0x008 -- 256 color support
TTYPE_OPT_MTTS_MTRA =   0x010 -- Mouse tracking support
TTYPE_OPT_MTTS_OSCC =   0x020 -- OSC color palette support (true color)
TTYPE_OPT_MTTS_READ =   0x040 -- Client using screen reader
TTYPE_OPT_MTTS_PROX =   0x080 -- This is a proxy connection
TTYPE_OPT_MTTS_TRUC =   0x100 -- True color support
TTYPE_OPT_MTTS_MNES =   0x200 -- Mud New Env Standard enabled
TTYPE_OPT_MTTS_MSLP =   0x400 -- Mud Server Link Protocol enabled

-- Default MTTS value:
local mtts = 0x0
mtts = mtts | TTYPE_OPT_MTTS_VT100
mtts = mtts | TTYPE_OPT_MTTS_ANSI
mtts = mtts | TTYPE_OPT_MTTS_UTF8
mtts = mtts | TTYPE_OPT_MTTS_256C
mtts = mtts | TTYPE_OPT_MTTS_TRUC
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

##

***ttype.auto_detect_reader_mode(val)***
Toggle if TTYPE negotiation should autodetect reader mode or not (default true)

- `val` The new value. True or false
