use lazy_static::lazy_static;
use libtelnet_rs::events::TelnetEvents;
use log::{error, info};
use std::path::PathBuf;
use std::sync::{
    atomic::Ordering,
    mpsc::{channel, Receiver, Sender},
};
use std::{env, fs, thread};
use ui::HelpHandler;

mod event;
mod io;
mod lua;
mod model;
mod net;
mod session;
mod timer;
mod tools;
mod tts;
mod ui;

use crate::event::Event;
use crate::io::SaveData;
use crate::model::Servers;
use crate::session::{Session, SessionBuilder};
use crate::timer::{spawn_timer_thread, TimerEvent};
use crate::ui::{spawn_input_thread, Screen};
use event::EventHandler;
use getopts::Options;
use model::{Connection, Settings, LOGGING_ENABLED, MOUSE_ENABLED, SAVE_HISTORY, SETTINGS};
use net::check_latest_version;
use tools::register_panic_hook;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROJECT_NAME: &str = "Blightmud";

#[cfg(not(debug_assertions))]
const XDG_DATA_DIR: &str = "~/.local/share/blightmud";
#[cfg(not(debug_assertions))]
const XDG_CONFIG_DIR: &str = "~/.config/blightmud";

type TelnetData = Option<Vec<u8>>;

lazy_static! {
    pub static ref DATA_DIR: PathBuf = {
        #[cfg(not(debug_assertions))]
        {
            let data_dir = if cfg!(target_os = "macos") && MACOS_DEPRECATED_DIR.exists() {
                MACOS_DEPRECATED_DIR.to_path_buf()
            } else {
                PathBuf::from(crate::lua::util::expand_tilde(XDG_DATA_DIR).as_ref())
            };

            let _ = std::fs::create_dir_all(&data_dir);
            data_dir
        }

        #[cfg(debug_assertions)]
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    };
    pub static ref CONFIG_DIR: PathBuf = {
        #[cfg(not(debug_assertions))]
        {
            let config_dir = if cfg!(target_os = "macos") && MACOS_DEPRECATED_DIR.exists() {
                MACOS_DEPRECATED_DIR.to_path_buf()
            } else {
                PathBuf::from(crate::lua::util::expand_tilde(XDG_CONFIG_DIR).as_ref())
            };

            let _ = std::fs::create_dir_all(&config_dir);
            config_dir
        }

        #[cfg(debug_assertions)]
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    };
    pub static ref MACOS_DEPRECATED_DIR: PathBuf = {
        use crate::lua::util;
        PathBuf::from(util::expand_tilde("~/Library/Application Support/blightmud").as_ref())
    };
}

fn register_terminal_resize_listener(session: Session) -> thread::JoinHandle<()> {
    let signals = signal_hook::iterator::Signals::new(&[signal_hook::SIGWINCH]).unwrap();
    let main_thread_writer = session.main_writer;
    thread::spawn(move || {
        for _ in signals.forever() {
            if let Err(err) = main_thread_writer.send(Event::Redraw) {
                error!("Reize listener failed: {}", err);
            }
        }
    })
}

fn start_logging() -> std::io::Result<()> {
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;

    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;

    let logpath = DATA_DIR.clone().join("logs");
    std::fs::create_dir_all(&logpath)?;

    let logfile = logpath.join("log.txt");

    simple_logging::log_to_file(logfile.to_str().unwrap(), log_level)?;

    Ok(())
}

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

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    let program = &args[0];
    opts.optopt("c", "connect", "Connect to server", "HOST:PORT");
    opts.optflag(
        "t",
        "tls",
        "Use tls when connecting to a server (only applies in combination with --connect)",
    );
    opts.optflag(
        "T",
        "tts",
        "Use the TTS system when playing a MUD (for visually impaired users)",
    );
    opts.optopt("w", "world", "Connect to a predefined world", "WORLD");
    opts.optflag("h", "help", "Print help menu");
    opts.optflag("v", "version", "Print version information");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_help(program, opts);
        return;
    }

    if matches.opt_present("v") {
        print_version();
        return;
    }

    register_panic_hook();
    if let Err(e) = start_logging() {
        panic!("[!!] Logging failed to start: {:?}", e);
    }

    info!("Starting application");

    let (main_writer, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();
    let timer_writer = spawn_timer_thread(main_writer.clone());

    if let Ok(Some(connect)) = matches.opt_get::<String>("c") {
        if connect.contains(':') {
            let split: Vec<&str> = connect.split(':').collect();
            let host = split[0];
            let port: u16 = split[1].parse().unwrap();
            let tls = matches.opt_present("tls");
            main_writer
                .send(Event::Connect(Connection::new(host, port, tls)))
                .unwrap();
        } else {
            print_help(program, opts);
            return;
        }
    } else if let Ok(Some(world)) = matches.opt_get::<String>("w") {
        let servers = Servers::load();
        if servers.contains_key(&world) {
            main_writer
                .send(Event::Connect(servers.get(&world).unwrap().clone()))
                .unwrap();
        }
    } else {
        main_writer
            .send(Event::ShowHelp("welcome".to_string()))
            .unwrap();
    }

    let settings = Settings::load();
    let dimensions = termion::terminal_size().unwrap();
    let session = SessionBuilder::new()
        .main_writer(main_writer)
        .timer_writer(timer_writer)
        .screen_dimensions(dimensions)
        .tts_enabled(matches.opt_present("tts"))
        .save_history(settings.get(SAVE_HISTORY).unwrap())
        .build();

    if let Err(error) = run(main_thread_read, session, settings) {
        error!("Panic: {}", error.to_string());
        panic!("[!!] Panic: {:?}", error);
    }

    info!("Shutting down");
}

fn run(
    main_thread_read: Receiver<Event>,
    mut session: Session,
    mut settings: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transmit_writer: Option<Sender<TelnetData>> = None;
    let help_handler = HelpHandler::new(session.main_writer.clone());
    let mut event_handler = EventHandler::from(&session);
    let mut saved_servers = Servers::load();

    let mut screen = Screen::new(session.tts_ctrl.clone(), settings.get(MOUSE_ENABLED)?)?;
    screen.setup()?;

    let _input_thread =
        spawn_input_thread(session.clone(), saved_servers.keys().cloned().collect());
    let _signal_thread = register_terminal_resize_listener(session.clone());

    let lua_scripts = {
        fs::read_dir(CONFIG_DIR.as_path())?
            .filter_map(|entry| match entry {
                Ok(file) => {
                    if let Ok(filename) = file.file_name().into_string() {
                        if filename.ends_with(".lua") {
                            Some(file.path())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<PathBuf>>()
    };

    for script in lua_scripts {
        session
            .main_writer
            .send(Event::LoadScript(script.to_str().unwrap().to_string()))?;
    }

    check_latest_version(session.main_writer.clone());
    #[cfg(all(not(debug_assertions), target_os = "macos"))]
    {
        if MACOS_DEPRECATED_DIR.exists() {
            let msg = r#"~/Library/Application Support/blightmud will be removed in a future release.
Please move your Lua scripts to ~/.config/blightmud
Please move your data/config/log dirs to ~/.local/share/blightmud
For more info: https://github.com/LiquidityC/Blightmud/issues/173"#;
            for line in msg.lines() {
                session
                    .main_writer
                    .send(Event::Error(line.to_string()))
                    .unwrap();
            }
        }
    }

    loop {
        if session.terminate.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(event) = main_thread_read.recv() {
            match event {
                Event::ServerSend(_)
                | Event::ServerInput(_)
                | Event::Connect(_)
                | Event::Connected
                | Event::Reconnect
                | Event::Disconnect(_) => {
                    event_handler.handle_server_events(event, &mut screen, &mut transmit_writer)?;
                }
                Event::AddServer(_, _)
                | Event::RemoveServer(_)
                | Event::LoadServer(_)
                | Event::ListServers => {
                    event_handler.handle_store_events(event, &mut saved_servers, &mut screen)?;
                }
                Event::MudOutput(_)
                | Event::Output(_)
                | Event::Prompt
                | Event::Error(_)
                | Event::Info(_)
                | Event::InputSent(_)
                | Event::UserInputBuffer(_, _) => {
                    //tts_ctrl.handle_events(event.clone());
                    event_handler.handle_output_events(event, &mut screen)?;
                }

                Event::TTSEnabled(enabled) => session.tts_ctrl.lock().unwrap().enabled(enabled),
                Event::Speak(msg, interupt) => {
                    session.tts_ctrl.lock().unwrap().speak(&msg, interupt)
                }
                Event::SpeakStop => session.tts_ctrl.lock().unwrap().flush(),
                Event::TTSEvent(event) => session.tts_ctrl.lock().unwrap().handle(event),

                Event::ShowSettings => SETTINGS.iter().for_each(|key| {
                    screen.print_info(&format!("{} => {}", key, settings.get(key).unwrap()));
                }),
                Event::ShowSetting(setting) => match settings.get(&setting) {
                    Ok(value) => screen.print_info(&format!("Setting: {} => {}", setting, value)),
                    Err(error) => screen.print_error(&error.to_string()),
                },
                Event::ToggleSetting(setting, toggle) => {
                    let toggle = matches!(toggle.as_str(), "on" | "true" | "enabled");
                    if let Err(error) = settings.set(&setting, toggle) {
                        screen.print_error(&error.to_string());
                    } else {
                        if setting == SAVE_HISTORY {
                            session.set_save_history(toggle);
                        }
                        screen.print_info(&format!("Setting: {} => {}", setting, toggle));
                    }
                }
                Event::StartLogging(world, force) => {
                    if settings.get(LOGGING_ENABLED)? || force {
                        screen.print_info(&format!("Started logging for: {}", world));
                        session.start_logging(&world)
                    }
                }
                Event::StopLogging => {
                    screen.print_info("Logging stopped");
                    session.stop_logging();
                }
                Event::EnableProto(proto) => {
                    if let Ok(mut parser) = session.telnet_parser.lock() {
                        parser.options.support(proto);
                        if session.connected() {
                            if let Some(TelnetEvents::DataSend(data)) = parser._do(proto) {
                                session.main_writer.send(Event::ServerSend(data)).unwrap();
                            }
                        }
                    }
                }
                Event::DisableProto(proto) => {
                    if let Ok(mut parser) = session.telnet_parser.lock() {
                        let mut opt = parser.options.get_option(proto);
                        opt.local = false;
                        opt.remote = false;
                        parser.options.set_option(proto, opt);
                        if session.connected() {
                            if let Some(TelnetEvents::DataSend(data)) = parser._dont(proto) {
                                session.main_writer.send(Event::ServerSend(data)).unwrap();
                            }
                        }
                    }
                }
                Event::ProtoEnabled(proto) => {
                    if let Ok(mut lua) = session.lua_script.lock() {
                        lua.proto_enabled(proto);
                        lua.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                }
                Event::ProtoSubnegRecv(proto, data) => {
                    if let Ok(mut script) = session.lua_script.lock() {
                        script.proto_subneg(proto, &data);
                        script.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                }
                Event::ProtoSubnegSend(proto, data) => {
                    if let Ok(mut parser) = session.telnet_parser.lock() {
                        if let Some(TelnetEvents::DataSend(data)) =
                            parser.subnegotiation(proto, data)
                        {
                            session.main_writer.send(Event::ServerSend(data)).unwrap();
                        }
                    }
                }
                Event::ScrollUp => screen.scroll_up()?,
                Event::ScrollDown => screen.scroll_down()?,
                Event::ScrollBottom => screen.reset_scroll()?,
                Event::StatusAreaHeight(height) => screen.set_status_area_height(height)?,
                Event::StatusLine(index, info) => screen.set_status_line(index, info)?,
                Event::LoadScript(path) => {
                    info!("Loading script: {}", path);
                    let mut lua = session.lua_script.lock().unwrap();
                    if let Err(err) = lua.load_script(&path) {
                        screen.print_error(&format!("Failed to load file: {}", err));
                    } else {
                        screen.print_info(&format!("Loaded script: {}", path));
                        lua.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                }
                Event::ResetScript => {
                    info!("Clearing scripts");
                    screen.print_info("Clearing scripts...");
                    if let Ok(mut script) = session.lua_script.lock() {
                        script.reset((screen.width, screen.height));
                        screen.print_info("Done");
                    }
                    session.timer_writer.send(TimerEvent::Clear)?;
                }
                Event::ShowHelp(hfile) => {
                    help_handler.show_help(&hfile)?;
                }
                Event::AddTimedEvent(duration, count, id) => {
                    session
                        .timer_writer
                        .send(TimerEvent::Create(duration, count, id))?;
                }
                Event::TimedEvent(id) => {
                    if let Ok(mut script) = session.lua_script.lock() {
                        script.run_timed_function(id);
                        script.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                }
                Event::DropTimedEvent(id) => {
                    session.lua_script.lock().unwrap().remove_timed_function(id);
                }
                Event::ClearTimers => {
                    session.timer_writer.send(TimerEvent::Clear)?;
                }
                Event::RemoveTimer(idx) => {
                    session.timer_writer.send(TimerEvent::Remove(idx))?;
                }
                Event::Redraw => {
                    screen.setup()?;
                    if let Ok(mut script) = session.lua_script.lock() {
                        script.set_dimensions((screen.width, screen.height));
                    }
                    let prompt_input = session.prompt_input.lock().unwrap();
                    screen.print_prompt_input(&prompt_input, prompt_input.len());
                }
                Event::Quit => {
                    session.terminate.store(true, Ordering::Relaxed);
                    session.disconnect();
                    break;
                }
            };
            screen.flush();
        }
    }
    screen.reset()?;
    settings.save();
    session.close()?;
    Ok(())
}
