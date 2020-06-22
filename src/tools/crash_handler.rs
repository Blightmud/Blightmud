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

        println!("\x1b[31m");
        println!(
            r#"
        Blightmud crashed !!!

        Well this is embarrassing... I guess no software is flawless

        Since this is an open source project that values your privacy as a user we don't collect or
        automatically submit any information anywhere. It would however be a great help if you took
        a minute to submit a bug report on github.

        [URL]: https://github.com/liquidityc/blightmud/issues"#
        );
        println!("        [CRASH_LOG]: {}", file_path);

        println!(
            r#"
        If available, please attach the created crash log to the issue.  Then we'll get around to
        fixing your problem as fast as we can.

        Br,
        Linus Probert and all the contributors
        "#
        );
        println!("\x1b[0m");
    }));
}
