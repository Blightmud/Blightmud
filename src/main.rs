#[cfg(not(debug_assertions))]
use dirs;

use lazy_static::lazy_static;
use libtelnet_rs::{events::TelnetEvents, telnet::op_option as opt};
use log::{debug, error, info};
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
mod ui;

use crate::event::Event;
use crate::io::SaveData;
use crate::model::Servers;
use crate::session::{Session, SessionBuilder};
use crate::timer::{spawn_timer_thread, TimerEvent};
use crate::ui::{spawn_input_thread, Screen};
use event::EventHandler;
use getopts::Options;
use model::{Connection, Settings, ECHO_GMCP, LOGGING_ENABLED};
use net::check_latest_version;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROJECT_NAME: &str = "Blightmud";

type TelnetData = Option<Vec<u8>>;

lazy_static! {
    pub static ref DATA_DIR: PathBuf = {
        #[cfg(not(debug_assertions))]
        {
            let mut data_dir = dirs::data_dir().unwrap();
            data_dir.push("blightmud");
            let _ = std::fs::create_dir(&data_dir);
            data_dir
        }

        #[cfg(debug_assertions)]
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    };
    pub static ref CONFIG_DIR: PathBuf = {
        #[cfg(not(debug_assertions))]
        {
            let mut config_dir = dirs::config_dir().unwrap();
            config_dir.push("blightmud");
            let _ = std::fs::create_dir(&config_dir);
            config_dir
        }

        #[cfg(debug_assertions)]
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    let program = &args[0];
    opts.optopt("c", "connect", "Connect to server", "HOST:PORT");
    opts.optopt("w", "world", "Connect to a predefined world", "WORLD");
    opts.optflag("h", "help", "Print help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_help(program, opts);
        return;
    }

    if let Err(e) = start_logging() {
        error!("Logging failed to start: {:?}", e);
        println!("[!!] Logging failed to start: {:?}", e);
    }
    
    info!("Starting application");

    let (main_writer, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();
    let timer_writer = spawn_timer_thread(main_writer.clone());

    if let Ok(Some(connect)) = matches.opt_get::<String>("c") {
        if connect.contains(':') {
            let split: Vec<&str> = connect.split(':').collect();
            let host = split[0].to_string();
            let port: u16 = split[1].parse().unwrap();
            main_writer
                .send(Event::Connect(Connection { host, port }))
                .unwrap();
        } else {
            print_help(program, opts);
            return;
        }
    } else if let Ok(Some(world)) = matches.opt_get::<String>("w") {
        let servers = Servers::load().unwrap();
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

    let dimensions = termion::terminal_size().unwrap();
    let session = SessionBuilder::new()
        .main_writer(main_writer)
        .timer_writer(timer_writer)
        .screen_dimensions(dimensions)
        .build();

    let _input_thread = spawn_input_thread(session.clone());
    let _signal_thread = register_terminal_resize_listener(session.clone());

    if let Err(error) = run(main_thread_read, session) {
        error!("Panic: {}", error.to_string());
        println!("[!!] Panic: {}", error.to_string());
    }

    info!("Shutting down");
}

fn run(
    main_thread_read: Receiver<Event>,
    mut session: Session,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut screen = Screen::new()?;
    screen.setup()?;

    let mut transmit_writer: Option<Sender<TelnetData>> = None;
    let help_handler = HelpHandler::new(session.main_writer.clone());
    let mut settings = Settings::load().unwrap();

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

    let mut event_handler = EventHandler::from(&session);

    let mut saved_servers = Servers::load()?;

    check_latest_version(session.main_writer.clone());

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
                | Event::UserInputBuffer(_, _) => {
                    event_handler.handle_output_events(event, &mut screen)?;
                }
                Event::ShowSetting(setting) => match settings.get(&setting) {
                    Ok(value) => screen.print_info(&format!("Setting: {} => {}", setting, value)),
                    Err(error) => screen.print_error(&error.to_string()),
                },
                Event::ToggleSetting(setting, toggle) => {
                    if let Err(error) = settings.set(
                        &setting,
                        match toggle.as_str() {
                            "on" => true,
                            "true" => true,
                            "enabled" => true,
                            _ => false,
                        },
                    ) {
                        screen.print_error(&error.to_string());
                    } else {
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
                Event::ProtoEnabled(proto) => {
                    if let opt::GMCP = proto {
                        let mut parser = session.telnet_parser.lock().unwrap();
                        if let Some(event) = parser.subnegotiation_text(
                            opt::GMCP,
                            &format!(
                                "Core.Hello {{\"Client\":\"Blightmud\",\"Version\":\"{}\"}}",
                                VERSION
                            ),
                        ) {
                            if let TelnetEvents::DataSend(data) = event {
                                debug!("Sending GMCP Core.Hello");
                                session.main_writer.send(Event::ServerSend(data))?;
                                if let Ok(mut lua) = session.lua_script.lock() {
                                    lua.on_gmcp_ready();
                                    lua.get_output_lines().iter().for_each(|l| {
                                        screen.print_output(l);
                                    });
                                }
                            }
                        } else {
                            error!("Failed to send GMCP Core.Hello");
                        }
                    }
                }
                Event::GMCPRegister(msg) => {
                    let mut parser = session.telnet_parser.lock().unwrap();
                    if let Some(TelnetEvents::DataSend(data)) = parser.subnegotiation_text(
                        opt::GMCP,
                        &format!("Core.Supports.Add [\"{} 1\"]", msg),
                    ) {
                        session.main_writer.send(Event::ServerSend(data))?;
                    }
                }
                Event::GMCPReceive(msg) => {
                    let mut script = session.lua_script.lock().unwrap();
                    if let Ok(true) = settings.get(ECHO_GMCP) {
                        screen.print_info(&format!("[GMCP][RECEIVE]: {}", &msg));
                    }
                    script.receive_gmcp(&msg);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Event::GMCPSend(msg) => {
                    let mut parser = session.telnet_parser.lock().unwrap();
                    if let Some(TelnetEvents::DataSend(data)) =
                        parser.subnegotiation_text(opt::GMCP, &msg)
                    {
                        session.main_writer.send(Event::ServerSend(data))?;
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
                        if session.connected.load(Ordering::Relaxed) {
                            lua.on_connect(
                                &session.host.lock().unwrap(),
                                session.port.load(Ordering::Relaxed),
                            );
                            lua.on_gmcp_ready();
                        }
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
                Event::Redraw => {
                    screen.setup()?;
                    screen.reset_scroll()?;
                    if let Ok(mut script) = session.lua_script.lock() {
                        script.set_dimensions((screen.width, screen.height))?;
                    }
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
    settings.save()?;
    session.close()?;
    Ok(())
}
