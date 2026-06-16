# Command line options

Blightmud accepts the following options when started from the command line

- `-c`, `--connect <HOST:PORT>`   Connect to a server on startup
- `-t`, `--tls`                    Use TLS when connecting (only applies with `--connect`)
- `-n`, `--no-verify`             Don't verify the cert for the TLS connection
- `-s`, `--script <PATH>`         Launch using the provided script instead of the defaults
- `-T`, `--tts`                   Use the TTS system (only in builds compiled with TTS)
- `-w`, `--world <WORLD>`         Connect to a predefined world
- `-r`, `--reader-mode`           Force screen reader friendly mode
- `-V`, `--verbose`              Enable verbose logging
- `--no-update-check`            Skip checking for new Blightmud versions at startup
- `--codec <CODEC>`             Specify the codec to use for the MUD (eg. `UTF8`)
- `-v`, `--version`             Print version information and exit
- `-h`, `--help`               Print the help menu and exit

The authoritative list of which options are available for your build is always available by running `blightmud --help`.

The arguments Blightmud was started with can be read from within Lua via `core.command_line()`. See `/help core` for details.
