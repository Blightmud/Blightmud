#[cfg(not(debug_assertions))]
use dirs;

use lazy_static::lazy_static;
use libtelnet_rs::{events::TelnetEvents, telnet::op_option as opt};
use log::{debug, error, info};
use signal_hook;
use std::path::PathBuf;
use std::sync::{
    atomic::Ordering,
    mpsc::{channel, Receiver, Sender},
};
use std::thread;
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

const VERSION: &str = env!("CARGO_PKG_VERSION");

type TelnetData = Option<Vec<u8>>;
type BlightResult = Result<(), Box<dyn std::error::Error>>;

lazy_static! {
    static ref DATA_DIR: PathBuf = {
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

fn start_logging() {
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;

    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;

    let logpath = DATA_DIR.clone().join("logs");
    let _ = std::fs::create_dir(&logpath);

    let logfile = logpath.join("log.txt");

    simple_logging::log_to_file(logfile.to_str().unwrap(), log_level).unwrap();
}

fn main() {
    start_logging();
    info!("Starting application");

    let (main_writer, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();
    let timer_writer = spawn_timer_thread(main_writer.clone());

    let session = SessionBuilder::new()
        .main_writer(main_writer)
        .timer_writer(timer_writer)
        .build();

    let _input_thread = spawn_input_thread(session.clone());
    let _signal_thread = register_terminal_resize_listener(session.clone());

    if let Err(error) = run(main_thread_read, session) {
        error!("Panic: {}", error.to_string());
        println!("[!!] Panic: {}", error.to_string());
    }

    info!("Shutting down");
}

fn run(main_thread_read: Receiver<Event>, mut session: Session) -> BlightResult {
    let mut screen = Screen::new()?;
    screen.setup()?;

    let mut transmit_writer: Option<Sender<TelnetData>> = None;
    let help_handler = HelpHandler::new(session.main_writer.clone());

    session.send_event(Event::ShowHelp("welcome".to_string()));

    let mut event_handler = EventHandler::from(&session);

    let mut saved_servers = Servers::load()?;

    loop {
        if session.terminate.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(event) = main_thread_read.recv() {
            match event {
                Event::ServerSend(_)
                | Event::ServerInput(_, _)
                | Event::Connect(_)
                | Event::Connected
                | Event::Disconnect => {
                    event_handler.handle_server_events(event, &mut screen, &mut transmit_writer)?;
                }
                Event::AddServer(_, _)
                | Event::RemoveServer(_)
                | Event::LoadServer(_)
                | Event::ListServers => {
                    event_handler.handle_store_events(event, &mut saved_servers)?;
                }
                Event::MudOutput(_)
                | Event::Output(_)
                | Event::Prompt
                | Event::UserInputBuffer(_, _) => {
                    event_handler.handle_output_events(event, &mut screen)?;
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
                                session.lua_script.lock().unwrap().on_gmcp_ready();
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
                    script.receive_gmcp(&msg);
                }
                Event::ScrollUp => screen.scroll_up()?,
                Event::ScrollDown => screen.scroll_down()?,
                Event::ScrollBottom => screen.reset_scroll()?,
                Event::Error(msg) => {
                    screen.print_error(&msg);
                }
                Event::Info(msg) => {
                    screen.print_info(&msg);
                }
                Event::LoadScript(path) => {
                    info!("Loading script: {}", path);
                    let mut lua = session.lua_script.lock().unwrap();
                    if let Err(err) = lua.load_script(&path) {
                        screen.print_error(&format!("Failed to load file: {}", err));
                    } else {
                        screen.print_info(&format!("Loaded script: {}", path));
                        if session.connected.load(Ordering::Relaxed) {
                            lua.on_connect();
                            lua.on_gmcp_ready();
                        }
                    }
                }
                Event::ResetScript => {
                    info!("Clearing scripts");
                    screen.print_info("Clearing scripts...");
                    if let Ok(mut script) = session.lua_script.lock() {
                        script.reset();
                        screen.print_info("Done");
                    }
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
                    session.lua_script.lock().unwrap().run_timed_function(id);
                }
                Event::DropTimedEvent(id) => {
                    session.lua_script.lock().unwrap().remove_timed_function(id);
                }
                Event::Redraw => {
                    screen.setup()?;
                    screen.reset_scroll()?;
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
    session.close()?;
    Ok(())
}
