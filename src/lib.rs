use audio::Player;
use lazy_static::lazy_static;
use libtelnet_rs::bytes::Bytes;
use libtelnet_rs::events::TelnetEvents;
use log::{error, info};
use notify::Watcher;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{env, fs, thread};
use ui::HelpHandler;

mod audio;
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

use crate::event::{Event, QuitMethod};
use crate::io::{FSMonitor, SaveData};
use crate::model::{Servers, HIDE_TOPBAR, READER_MODE, SCROLL_SPLIT};
use crate::session::{Session, SessionBuilder};
use crate::timer::{spawn_timer_thread, TimerEvent};
use crate::tools::patch::migrate_v2_settings_and_servers;
use crate::ui::{spawn_input_thread, UiWrapper, UserInterface};
use event::EventHandler;
use getopts::Matches;
use model::{Connection, Settings, CONFIRM_QUIT, LOGGING_ENABLED, SAVE_HISTORY};
use net::check_latest_version;
use tools::register_panic_hook;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), env!("GIT_DESCRIBE"));
pub const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(all(not(test), not(debug_assertions)))]
const XDG_DATA_DIR: &str = "~/.local/share/blightmud";

#[cfg(all(not(test), not(debug_assertions)))]
const XDG_CONFIG_DIR: &str = "~/.config/blightmud";

type TelnetData = Option<Bytes>;

lazy_static! {
    pub static ref DATA_DIR: PathBuf = {
        #[cfg(all(not(test), not(debug_assertions)))]
        {
            let data_dir = if cfg!(target_os = "macos") && MACOS_DEPRECATED_DIR.exists() {
                MACOS_DEPRECATED_DIR.to_path_buf()
            } else {
                PathBuf::from(crate::lua::util::expand_tilde(XDG_DATA_DIR).as_ref())
            };

            let _ = std::fs::create_dir_all(&data_dir);
            data_dir
        }

        #[cfg(test)]
        {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".run/test/data")
        }

        #[cfg(all(not(test), debug_assertions))]
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".run/data")
    };
    pub static ref CONFIG_DIR: PathBuf = {
        #[cfg(all(not(test), not(debug_assertions)))]
        {
            let config_dir = if cfg!(target_os = "macos") && MACOS_DEPRECATED_DIR.exists() {
                MACOS_DEPRECATED_DIR.to_path_buf()
            } else {
                PathBuf::from(crate::lua::util::expand_tilde(XDG_CONFIG_DIR).as_ref())
            };

            let _ = std::fs::create_dir_all(&config_dir);
            config_dir
        }

        #[cfg(test)]
        {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".run/test/config")
        }

        #[cfg(all(not(test), debug_assertions))]
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".run/config")
    };
    pub static ref MACOS_DEPRECATED_DIR: PathBuf = {
        use crate::lua::util;
        PathBuf::from(util::expand_tilde("~/Library/Application Support/blightmud").as_ref())
    };
}

fn register_terminal_resize_listener(session: Session) -> thread::JoinHandle<()> {
    let mut signals =
        signal_hook::iterator::Signals::new(&[signal_hook::consts::SIGWINCH]).unwrap();
    let main_thread_writer = session.main_writer;
    thread::Builder::new()
        .name("signal-thread".to_string())
        .spawn(move || {
            for _ in signals.forever() {
                if let Err(err) = main_thread_writer.send(Event::Redraw) {
                    error!("Resize listener failed: {}", err);
                }
            }
        })
        .unwrap()
}

fn start_logging(log_level: log::LevelFilter) -> std::io::Result<()> {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log_level
    };

    let logpath = DATA_DIR.clone().join("logs");
    std::fs::create_dir_all(&logpath)?;

    let logfile = logpath.join("log.txt");

    simple_logging::log_to_file(logfile.to_str().unwrap(), log_level)?;

    Ok(())
}

#[derive(Default)]
pub struct RuntimeConfig {
    pub reader_mode: bool,
    pub headless_mode: bool,
    pub verbose: bool,
    pub world: Option<String>,
    pub use_tts: bool,
    pub tls: bool,
    pub no_verify: bool,
    pub connect: Option<String>,
    pub no_panic_hook: bool,
}

impl From<Matches> for RuntimeConfig {
    fn from(matches: Matches) -> Self {
        let world = if let Ok(world) = matches.opt_get::<String>("world") {
            world
        } else {
            None
        };
        let connect = if let Ok(connect) = matches.opt_get::<String>("connect") {
            connect
        } else {
            None
        };
        Self {
            reader_mode: matches.opt_present("reader-mode"),
            headless_mode: false,
            verbose: matches.opt_present("verbose"),
            world,
            use_tts: matches.opt_present("tts"),
            tls: matches.opt_present("tls"),
            no_verify: matches.opt_present("no-verify"),
            connect,
            no_panic_hook: false,
        }
    }
}

pub fn start(rt: RuntimeConfig) {
    if !rt.no_panic_hook {
        register_panic_hook(rt.headless_mode);
    }
    let log_level = if rt.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    if let Err(e) = start_logging(log_level) {
        panic!("[!!] Logging failed to start: {:?}", e);
    }

    info!("Starting application");

    let (main_writer, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();
    let timer_writer = spawn_timer_thread(main_writer.clone());

    if let Some(connect) = rt.connect {
        let split: Vec<&str> = connect.split(':').collect();
        let host = split[0];
        let port: u16 = split[1].parse().unwrap();
        let tls = rt.tls;
        let no_verify = rt.no_verify;
        main_writer
            .send(Event::Connect(Connection::new(host, port, tls, !no_verify)))
            .unwrap();
    } else if let Some(world) = rt.world {
        let servers = Servers::try_load().expect("Error loading servers.ron");
        if servers.contains_key(&world) {
            main_writer
                .send(Event::Connect(servers.get(&world).unwrap().clone()))
                .unwrap();
        }
    } else {
        main_writer
            .send(Event::ShowHelp("welcome".to_string(), false))
            .unwrap();
    }

    let mut settings = Settings::try_load().expect("Error loading settings.ron");
    if rt.reader_mode {
        settings.set(READER_MODE, true).unwrap();
        settings.save();
    }

    let dimensions = termion::terminal_size().unwrap_or((100, 100));
    let session = SessionBuilder::new()
        .main_writer(main_writer)
        .timer_writer(timer_writer)
        .screen_dimensions(dimensions)
        .tts_enabled(rt.use_tts)
        .save_history(settings.get(SAVE_HISTORY).unwrap())
        .build();

    if let Err(error) = run(main_thread_read, session, rt.headless_mode) {
        error!("Panic: {}", error.to_string());
        panic!("[!!] Panic: {:?}", error);
    }

    info!("Shutting down");
}

fn run(
    main_thread_read: Receiver<Event>,
    mut session: Session,
    headless: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut transmit_writer: Option<Sender<TelnetData>> = None;
    let help_handler = HelpHandler::new(session.main_writer.clone());
    let mut event_handler = EventHandler::from(&session);

    let mut player = if !cfg!(debug_assertions) {
        Player::new()
    } else {
        Player::disabled()
    };

    let mut screen: Box<dyn UserInterface> = if !headless {
        Box::new(UiWrapper::new(&session, false)?)
    } else {
        Box::new(UiWrapper::headless(&session)?)
    };

    let mut fs_monitor = FSMonitor::new(session.main_writer.clone())?;

    screen.setup()?;

    let _ = spawn_input_thread(session.clone());
    let _ = register_terminal_resize_listener(session.clone());

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
    if cfg!(not(debug_assertions)) {
        migrate_v2_settings_and_servers(session.main_writer.clone());
    }

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
    let mut quit_pending = false;
    while let Ok(event) = main_thread_read.recv() {
        if quit_pending {
            quit_pending = matches!(
                event,
                Event::Quit(_)
                    | Event::UserInputBuffer(..)
                    | Event::TimedEvent(..)
                    | Event::TimerTick(..)
            );
        }

        match event {
            Event::SetPromptInput(line) => {
                if let Ok(mut buffer) = session.command_buffer.lock() {
                    buffer.clear();
                    buffer.set(line);
                    session
                        .main_writer
                        .send(Event::UserInputBuffer(
                            buffer.get_buffer(),
                            buffer.get_pos(),
                        ))
                        .unwrap();
                }
            }
            Event::ServerSend(_)
            | Event::ServerInput(_)
            | Event::Connect(_)
            | Event::Connected(_)
            | Event::Reconnect
            | Event::Disconnect(_) => {
                event_handler.handle_server_events(
                    event.clone(),
                    &mut screen,
                    &mut transmit_writer,
                )?;
                if let Event::Disconnect(_) = event {
                    if headless {
                        session
                            .main_writer
                            .send(Event::Quit(QuitMethod::System))
                            .unwrap();
                    }
                }
            }
            Event::MudOutput(_)
            | Event::Output(_)
            | Event::Prompt(_)
            | Event::Error(_)
            | Event::Info(_)
            | Event::AddTag(_)
            | Event::UserInputBuffer(_, _) => {
                //tts_ctrl.handle_events(event.clone());
                event_handler.handle_output_events(event, &mut screen)?;
            }
            Event::PlayMusic(_, _) | Event::StopMusic | Event::PlaySFX(_, _) | Event::StopSFX => {
                if let Err(err) = audio::handle_audio_event(event, &mut player) {
                    screen.print_error(&err.to_string())
                }
            }
            Event::TTSEnabled(enabled) => session.tts_ctrl.lock().unwrap().enabled(enabled),
            Event::Speak(msg, interupt) => session.tts_ctrl.lock().unwrap().speak(&msg, interupt),
            Event::SpeakStop => session.tts_ctrl.lock().unwrap().flush(),
            Event::TTSEvent(event) => session.tts_ctrl.lock().unwrap().handle(event),
            Event::SettingChanged(name, value) => match name.as_str() {
                READER_MODE => {
                    screen = Box::new(UiWrapper::new_from(screen, &session, value)?);
                }
                HIDE_TOPBAR | SCROLL_SPLIT => {
                    screen.setup()?;
                }
                _ => {}
            },
            Event::StartLogging(world, force) => {
                if Settings::load().get(LOGGING_ENABLED)? || force {
                    session.start_logging(&world)
                }
            }
            Event::StopLogging => {
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
                    if let Some(TelnetEvents::DataSend(data)) = parser.subnegotiation(proto, data) {
                        session.main_writer.send(Event::ServerSend(data)).unwrap();
                    }
                }
            }
            Event::ScrollLock(_)
            | Event::ScrollUp
            | Event::ScrollDown
            | Event::ScrollTop
            | Event::ScrollBottom
            | Event::FindForward(_)
            | Event::FindBackward(_) => {
                event_handler.handle_scroll_events(event, &mut screen)?;
            }
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
                if let Ok(mut script) = session.lua_script.lock() {
                    script.on_reset();
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                    screen.print_info("Clearing scripts...");
                    script.reset((screen.width(), screen.height()))?;
                    screen.print_info("Done");
                }
                session.timer_writer.send(TimerEvent::Clear(true))?;
            }
            Event::ShowHelp(hfile, lock) => {
                help_handler.show_help(&hfile, lock)?;
            }
            Event::AddTimedEvent(duration, count, id, core) => {
                session
                    .timer_writer
                    .send(TimerEvent::Create(duration, count, id, core))?;
            }
            Event::TimedEvent(id) => {
                if let Ok(mut script) = session.lua_script.lock() {
                    script.run_timed_function(id);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
            }
            Event::TimerTick(millis) => {
                if let Ok(mut script) = session.lua_script.lock() {
                    script.tick(millis);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
            }
            Event::DropTimedEvent(id) => {
                session.lua_script.lock().unwrap().remove_timed_function(id);
            }
            Event::ClearTimers => {
                session.timer_writer.send(TimerEvent::Clear(false))?;
            }
            Event::RemoveTimer(idx) => {
                session.timer_writer.send(TimerEvent::Remove(idx))?;
            }
            Event::FSMonitor(path) => {
                fs_monitor.watch(&path, notify::RecursiveMode::Recursive)?;
            }
            Event::FSEvent(e) => {
                if let Ok(script) = session.lua_script.lock() {
                    script.handle_fs_event(e)?;
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
            }
            Event::Redraw => {
                screen.setup()?;
                if let Ok(mut script) = session.lua_script.lock() {
                    script.set_dimensions((screen.width(), screen.height()));
                }
                let prompt_input = session.prompt_input.lock().unwrap();
                screen.print_prompt_input(&prompt_input, prompt_input.len());
            }
            Event::Quit(method) => {
                if Settings::load().get(CONFIRM_QUIT)?
                    && method == QuitMethod::CtrlC
                    && !quit_pending
                {
                    screen.print_info("Confirm quit with ctrl-c");
                    screen.flush();
                    quit_pending = true;
                    continue;
                }
                session.try_disconnect();
                break;
            }
        };
        screen.flush();
    }
    if let Ok(lua) = session.lua_script.lock() {
        lua.on_quit();
        lua.get_output_lines().iter().for_each(|l| {
            screen.print_output(l);
        });
        screen.flush();
    }
    screen.reset()?;
    session.close()?;
    Ok(())
}
