use std::env;

use blightmud::{RuntimeConfig, PROJECT_NAME, VERSION};
use getopts::Options;

fn print_help(program: &str, opts: Options) {
    let brief = format!(
        "USAGE: {} [options]\n\n{} {}",
        program, PROJECT_NAME, VERSION
    );
    print!("{}", opts.usage(&brief));
}

fn print_version() {
    println!(
        "{} v{} {}",
        PROJECT_NAME,
        VERSION,
        if cfg!(debug_assertions) {
            "[DEBUG]"
        } else {
            ""
        }
    );
}

fn setup_options() -> Options {
    let mut opts = Options::new();
    opts.optopt("c", "connect", "Connect to server", "HOST:PORT");
    opts.optflag(
        "t",
        "tls",
        "Use tls when connecting to a server (only applies in combination with --connect)",
    );
    opts.optflag(
        "n",
        "no-verify",
        "Don't verify the cert for the TLS connection",
    );
    opts.optflag(
        "T",
        "tts",
        "Use the TTS system when playing a MUD (for visually impaired users)",
    );
    opts.optopt("w", "world", "Connect to a predefined world", "WORLD");
    opts.optflag("h", "help", "Print help menu");
    opts.optflag("v", "version", "Print version information");
    opts.optflag("V", "verbose", "Enable verbose logging");
    opts.optflag("r", "reader-mode", "Force screen reader friendly mode");
    //opts.optflag("H", "headless-mode", "Runs Blightmud without a TUI");

    opts
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];
    let opts = setup_options();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f.to_string()),
    };

    if matches.opt_present("h") {
        print_help(program, opts);
        return;
    } else if matches.opt_present("v") {
        print_version();
        return;
    }

    let rt = RuntimeConfig::from(matches);

    if let Some(connect) = &rt.connect {
        if !connect.contains(&":") {
            print_help(program, opts);
            return;
        }
    }

    blightmud::start(rt);
}
