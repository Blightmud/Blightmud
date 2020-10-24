use std::env;
use std::panic;

pub fn register_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let meta = human_panic::Metadata {
            version: env!("CARGO_PKG_VERSION").into(),
            name: env!("CARGO_PKG_NAME").into(),
            authors: env!("CARGO_PKG_AUTHORS").replace(":", ", ").into(),
            homepage: env!("CARGO_PKG_HOMEPAGE").into(),
        };

        let file_path = if let Some(path_buf) = human_panic::handle_dump(&meta, panic_info) {
            path_buf.to_string_lossy().into_owned()
        } else {
            "<Unable to create dump file>".to_string()
        };

        // Attempt to reset the terminal since we crashed
        println!("\x1b[2J\x1b[r\x1b[?1049l");

        println!("\x1b[31m");

        r#"
        Blightmud crashed !!!

        Well this is embarrassing... I guess no software is flawless

        Since this is an open source project that values your privacy as a user we don't collect or
        automatically submit any information anywhere. It would however be a great help if you took
        a minute to submit a bug report on github.

        [URL]: https://github.com/liquidityc/blightmud/issues"#
            .to_string()
            .lines()
            .for_each(|line| print!("{}\r\n", line));

        print!("        [CRASH_LOG]: {}\r\n", file_path);

        r#"
        If available, please attach the created crash log to the issue.  Then we'll get around to
        fixing your problem as fast as we can.

        Br,
        Linus Probert and all the contributors
        "#
        .to_string()
        .lines()
        .for_each(|line| print!("{}\r\n", line));

        println!("\x1b[0m");
        std::process::exit(1);
    }));
}
