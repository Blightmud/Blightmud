# TTYPE

Blightmud will negotiate TTYPE when available on the current mud.
The defaults are:

- `IAC SB TTYPE SEND "BLIGHTMUD" IAC SE`
- `IAC SB TTYPE SEND "$TERM" IAC SE`
- `IAC SB TTYPE SEND "MTTS 271" IAC SE`

Where $TERM is the value of your TERM environment variable.

You may override TERM and MTTS with the following functions.
More info about MTTS can be found at [](https://tintin.mudhalla.net/protocols/mtts/).

##

***ttype.set_term(new_val)***
Will override the default for TERM with your provided string.

- `new_val` The new TERM value. Eg. "xterm-256color"

##

***ttype.set_mtts(new_val)***
Will override the MTTS value with the provided string.

- `new_val` The new value for MTTS. Eg. "MTTS 137"
