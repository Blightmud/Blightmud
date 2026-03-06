# pulldown-cmark-blightmud

> This repo is cloned and copied from [mdcat]. Since the original crate was yanked from crates.io

Render [pulldown-cmark] events to TTY.

This library backs the [blightmud] mud client for help docs and code rendering to terminal.

It supports:

- All common mark syntax.
- Standard ANSI formatting with OCS-8 hyperlinks.
- Inline images on terminal emulators with either the iTerm2 or the Kitty protocol, as well as on Terminology.
- Jump marks in iTerm2.

It does not support commonmark footnote extension syntax.

[blightmud]: https://github.com/blightmud/blightmud
[mdcat]: https://github.com/swsnr/mdcat
[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark

## License

Copyright Sebastian Wiesner <sebastian@swsnr.de>

Binaries are subject to the terms of the Mozilla Public
License, v. 2.0, see [LICENSE](LICENSE).

Most of the source is subject to the terms of the Mozilla Public
License, v. 2.0, see [LICENSE](LICENSE), unless otherwise noted;
some files are subject to the terms of the Apache 2.0 license,
see <http://www.apache.org/licenses/LICENSE-2.0>
