![Rust](https://github.com/LiquidityC/blightmud/workflows/Rust/badge.svg)
![Clippy check](https://github.com/LiquidityC/blightmud/workflows/Clippy%20check/badge.svg)
![Security audit](https://github.com/LiquidityC/blightmud/workflows/Security%20audit/badge.svg)
# Blightmud  : A mud client for the terminal

Blightmud has been a passion project of mine for some time. A big user of the old
but great tinyfugue I always wanted to create my own similar mud client. Even
though I don't play much muds these days.

![screenshot](resources/images/demo.gif)

## The name?
The client is written in rust. Some navigating throught the thesaurus brought me to the word 'blight' and here we are.

## Goals
- [x] Completely terminal based
- [x] Telnet:
    - [x] GMCP support
    - [x] MCCP2 support
- [x] Lua scripting:
    - [x] Output and sending
    - [x] Aliases
    - [x] Triggers
    - [x] Prompt triggers
    - [x] Gagging triggers
    - [x] GMCP hooks and sending
    - [x] Timers
- [x] Low resource and fast
- [x] In client help and manuals
    - [x] Markdown support in client
- [x] Tab completion

## Compiling
- Install rust
- Run `cargo build` to compile
- Run `cargo run` to run

## Contributing
All contributions are welcome. Check out [contributing guidelines](CONTRIBUTING.md).

## Side notes
This is my first rust project that has actually grown a bit. Some things might look silly but thanks to rust they should still be safe. Anywho. If you find some antipattern where you have a better idea I'm more then happy to se the PR and learn some more rustier ways.
