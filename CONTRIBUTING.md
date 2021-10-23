# Contributing guidelines

First of, It's great if you are interested in contributing to the project. I
would love for some more brains and experiences on this project since we all
play very different muds with various levels of telnet protocol support. I
can't test them all and it's bound to get wonky on some of them.

In this document I'll list the basic structure of the project. It's not that
hard to navigate through it with a decent editor but I hope this can help you
if you get confused.

## Data flow
There are essentially 4 core threads running when the client is running.

- *main_thread* (main.rs:run())                           : (acts as a router for all [Events](src/event.rs))
- *transmit_thread* [tcp_stream.rs](src/tcp_stream.rs)    : Handles sending bytes to server
- *receive_thread* [tcp_stream.rs](src/tcp_stream.rs)     : Handles reading bytes from server
- *input_thread* [command.rs](rsc/command.rs)             : Reads keyboard input

All these threads send [events](src/event.rs) through a channel Sender to the
*main_thread* that acts as a router distributing the data to the right handler.

That's the gist of it. If you add new things the safest bet is to route through the *main_thread* using events.

### The Session
The [session](src/session.rs) acts as a container for all
resources and streams and what not that describe a connection. My idea for this
was that if we at any point want to support multiple games in different tabs or
something then the entire mud state should be contained in one session.  The
session must always be safe to clone. And data that needs to be synced across
multiple threads needs to be thread safe. Eg. `rust Arc<Mutex<data>>`

The session object is often the argument passed into `new()` methods on
structs. I did not utilize the `From` trait, which I probably should have. Feel
free to change this where you see fit.

## Where do i start?
There should be a bunch of issues listed on the project. Things labeled
[good first issue](https://github.com/LiquidityC/Blightmud/labels/good%20first%20issue)
should generally be rather simple tasks to get into the project.

### Have an idea for something new?
Is there something you are missing in the client that would be nice to have or
you feel is an essential feature?  Create an issue and start working. We'll get
back to you regarding suggestions or ideas.

### Found a bug?
Create an issue. Already got a PR? Then create the issue and submit your PR.

### Committing
Before you commit please perform the following tasks:

- Considder if what you added is testable. If it is then write a test
- `cargo fmt`
- `cargo clippy`
- `cargo test`

### Naming and casing

#### Rust code:
- Functions: `snake_case`
- Structs: `UpperCamelCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

#### Lua API code:
- Pseudoclasses: `UpperCamelCase`
- Methods: `snake_case`
- Functions: `snake_case`
- Modules: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
