use crate::io::FSEvent;
use crate::lua::ConnectionInfo;
use crate::net::spawn_connect_thread;
use crate::{audio::SourceOptions, model::Regex};
use crate::{
    model::{Connection, Line, PromptMask, TagMask},
    net::{spawn_network_thread, WakingSender},
    session::Session,
    tabs::{TabOpts, MAIN_TAB},
    tts::TTSEvent,
    ui::{TopRowOpts, TopRowSelector, UserInterface},
};
use libmudtelnet::{bytes::Bytes, events::TelnetEvents};
use log::debug;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::{
    error::Error,
    sync::mpsc::{channel, Receiver, Sender},
    thread, time,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum QuitMethod {
    CtrlC,
    Script,
    System,
    Error(String),
}

/// Inner enum for [`Event::TabCommand`] — keeps the tabs API surface from
/// fanning out into many top-level Event variants.
#[derive(Debug, PartialEq, Clone)]
pub enum TabCommand {
    /// Create a new named tab with the supplied label / gag-main settings.
    Create { name: String, opts: TabOpts },
    /// Switch the active tab. Triggers a swap of the screen's History.
    Switch { name: String },
    /// Append a regex-string filter to a tab. Lines that match this regex
    /// (against the line's clean / ANSI-stripped form) are routed into the
    /// tab in addition to main.
    AddFilter { name: String, pattern: String },
    /// Append a regex-string *exclude* to a tab. Any candidate line that
    /// matches this regex is NOT routed to the tab, even when one of its
    /// `AddFilter` patterns also matches. Used as a blocklist hole inside
    /// a broad include rule (e.g. include `^\w+ says\b` but exclude
    /// `^(He|She|Smuggler) says\b`).
    AddExclude { name: String, pattern: String },
    /// Update a tab's display label (for the tab indicator row).
    SetLabel { name: String, label: String },
    /// Update a tab's display-only keyboard-shortcut hint (e.g. `Some("F2")`
    /// shows the tab as `[F2 - chat]` in the indicator). `None` clears it.
    /// Does NOT bind the key — use `blight.bind` for the actual binding.
    SetShortcut {
        name: String,
        shortcut: Option<String>,
    },
    /// Send a line directly into a specific tab, bypassing the filter
    /// machinery. Used by `blight.output_to(name, ...)`.
    OutputTo { name: String, line: Line },
    /// Remove a tab. `main` cannot be removed. Removing the active tab
    /// switches back to `main` first, then drops it (its scrollback is
    /// discarded). Used by `blight.remove_tab(name)`.
    Remove { name: String },
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Event {
    AddTag(String),
    AddTimedEvent(chrono::Duration, Option<u32>, u32, bool),
    ClearTags,
    ClearTimers,
    Connect(Connection),
    Connected(u16),
    ConnectionFailed,
    DisableProto(u8),
    Disconnect,
    DropTimedEvent(u32),
    EnableProto(u8),
    Error(String),
    FindBackward(Regex),
    FindForward(Regex),
    Info(String),
    LoadScript(String),
    EvalScript(String),
    MudOutput(Line),
    Output(Line),
    PlayMusic(String, SourceOptions),
    PlaySFX(String, SourceOptions),
    Prompt(Line),
    ProtoDisabled(u8),
    ProtoEnabled(u8),
    ProtoSubnegRecv(u8, Bytes),
    ProtoSubnegSend(u8, Bytes),
    Quit(QuitMethod),
    QuitConfirmTimeout,
    Reconnect,
    Redraw,
    RemoveTag(String),
    RemoveTimer(u32),
    ResetScript,
    ScrollBottom,
    ScrollDown,
    ScrollLock(bool),
    ScrollTop,
    ScrollUp,
    ServerInput(Line),
    ServerSend(Bytes),
    SettingChanged(String, bool),
    ShowHelp(String, bool),
    ShowTags(bool),
    Speak(String, bool),
    SpeakStop,
    StartLogging(String, bool),
    StatusAreaHeight(u16),
    InputHeight(u16),
    StatusLine(usize, String),
    StopLogging,
    StopMusic,
    StopSFX,
    TopLine(Option<String>),
    /// Mutate fields on an existing top row (built-in or Lua-added).
    SetTopRow(TopRowSelector, TopRowOpts),
    /// Reset a built-in row's body to its dynamic default (host_tags /
    /// tab_indicator). Lua-added rows are unaffected.
    ResetTopRow(TopRowSelector),
    /// Append a new top row. Auto-generates a name when `opts.name` is
    /// unset.
    AddTopRow(TopRowOpts),
    /// Remove a Lua-added top row. Built-ins refuse removal.
    RemoveTopRow(TopRowSelector),
    TTSEnabled(bool),
    TTSEvent(TTSEvent),
    TimedEvent(u32),
    TimerTick(u128),
    SetPromptInput(String),
    SetPromptCursorPos(usize),
    SetPromptMask(PromptMask),
    ClearPromptMask,
    SetTagMask(TagMask),
    SetHistoryCapacity(usize),
    /// Tabs control messages from Lua (`blight.create_tab`,
    /// `blight.switch_tab`, etc.). See [`TabCommand`].
    TabCommand(TabCommand),
    UserInputBuffer(String, usize),
    UserInputCursor(usize),
    FSEvent(FSEvent),
    FSMonitor(String),
    LuaError(String),
}
use anyhow::Result as AResult;
type Result = AResult<()>;

pub struct EventHandler {
    session: Session,
}

impl From<&Session> for EventHandler {
    fn from(session: &Session) -> Self {
        Self {
            session: session.clone(),
        }
    }
}

pub struct BadEventRoutingError;

impl std::fmt::Debug for BadEventRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad Event routing")
    }
}
impl std::fmt::Display for BadEventRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad Event routing")
    }
}

impl Error for BadEventRoutingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl EventHandler {
    /// Route an output line through the session's TabSet and only render
    /// it via the screen if the active tab received it. This is the
    /// integration point for the tabs feature: lines that match a
    /// non-active tab's filter are appended to that tab's history (for
    /// future scroll-back), and `gag_main=true` tabs suppress the line
    /// from main entirely.
    fn route_and_print(&self, line: &Line, screen: &mut Box<dyn UserInterface>) {
        let (render, indicator_snapshot) = if let Ok(mut tab_set) = self.session.tab_set.lock() {
            let r = tab_set.route(line);
            // When a non-active tab received the line, the indicator's
            // unread count changed and we need to repaint it.
            let snap = if r.indicator_dirty {
                Some(tab_set.list())
            } else {
                None
            };
            (r.render_to_screen, snap)
        } else {
            // If the tab set is poisoned (very unusual), fall back to
            // rendering — better to show too much than too little.
            (true, None)
        };
        if render {
            screen.print_output(line);
        }
        if let Some(snap) = indicator_snapshot {
            // Indicator update is best-effort; don't bubble errors here.
            let _ = screen.set_tab_indicator(snap);
        }
    }

    /// Push a fresh tabs snapshot to the screen indicator. Called after
    /// any TabCommand mutation (Create / Switch / SetLabel / OutputTo)
    /// that changes user-visible state.
    fn refresh_tab_indicator(&self, screen: &mut Box<dyn UserInterface>) {
        if let Ok(tab_set) = self.session.tab_set.lock() {
            let _ = screen.set_tab_indicator(tab_set.list());
        }
    }

    pub fn handle_server_events(
        &mut self,
        event: Event,
        screen: &mut Box<dyn UserInterface>,
        transmit_writer: &mut Option<WakingSender>,
    ) -> Result {
        match event {
            Event::ServerSend(data) => {
                debug!("Sending: {:?}", data);
                if let Some(transmit_writer) = transmit_writer {
                    let _ = transmit_writer.send(Some(data));
                } else {
                    screen.print_error("No active session. Use '/connect <host> <port>' to connect. '/help' for more commands.");
                }
                Ok(())
            }
            Event::ServerInput(mut line) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    let mut output_buffer = self.session.output_buffer.lock().unwrap();
                    output_buffer.input_sent();
                    script.on_mud_input(&mut line);
                    if self.session.echo_input.load(Ordering::Relaxed) {
                        screen.print_send(&line);
                    }
                    if let Ok(mut logger) = self.session.logger.lock() {
                        logger.log_line("> ", &line)?;
                    }
                    if !line.flags.matched {
                        if let Ok(mut parser) = self.session.telnet_parser.lock() {
                            if let TelnetEvents::DataSend(buffer) = parser.send_text(line.line()) {
                                let data = if let Some(codec) = self.session._codec {
                                    let text = std::str::from_utf8(&buffer).unwrap_or_default();
                                    let (encoded, _, _) = codec.encode(text);
                                    Bytes::copy_from_slice(&encoded)
                                } else {
                                    buffer
                                };
                                self.session.main_writer.send(Event::ServerSend(data))?;
                            }
                        }
                    }
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Ok(())
            }
            Event::Connect(connection) => {
                self.session.disconnect();
                spawn_connect_thread(self.session.clone(), connection);
                Ok(())
            }
            Event::Connected(id) => {
                let (writer, reader): (Sender<Option<Bytes>>, Receiver<Option<Bytes>>) = channel();
                let (waking_sender_tx, waking_sender_rx): (
                    Sender<WakingSender>,
                    Receiver<WakingSender>,
                ) = channel();

                // Get connection info and take the stream for the event loop
                let (stream, host, port, tls, tls_validation, name) = {
                    let mut conn = self.session.connection.lock().unwrap();
                    let stream = conn.take_stream();
                    (
                        stream,
                        conn.host.clone(),
                        conn.port,
                        conn.tls,
                        conn.tls_validation,
                        conn.name.clone(),
                    )
                };

                let verify_cert = tls_validation == crate::net::CertificateValidation::Enabled;

                if let Some(stream) = stream {
                    // Spawn the single network event loop thread
                    spawn_network_thread(
                        self.session.clone(),
                        stream,
                        tls,
                        &host,
                        tls_validation,
                        writer,
                        reader,
                        waking_sender_tx,
                    );
                    // Wait for the WakingSender from the event loop thread
                    match waking_sender_rx.recv() {
                        Ok(waking_sender) => {
                            transmit_writer.replace(waking_sender);
                        }
                        Err(_) => {
                            screen.print_error("Failed to initialize network connection");
                            return Ok(());
                        }
                    }
                } else {
                    screen.print_error("Failed to get connection stream");
                    return Ok(());
                }

                debug!("Connected to {}:{}", host, port);
                screen.set_host(&host, port)?;
                if let Ok(mut script) = self.session.lua_script.lock() {
                    let info = ConnectionInfo {
                        host: host.clone(),
                        port,
                        tls,
                        verify_cert,
                        name,
                        id,
                    };
                    script.on_connect(info);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Ok(())
            }
            Event::ConnectionFailed => {
                if let Ok(mut script) = self.session.lua_script.lock() {
                    script.on_connection_failed();
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Ok(())
            }
            Event::Disconnect => {
                if self.session.connected() {
                    self.session.disconnect();
                    screen.print_info(&format!(
                        "Disconnecting from: {}:{}",
                        self.session.host(),
                        self.session.port()
                    ));
                    if let Some(transmit_writer) = &transmit_writer {
                        // Ignore error if channel is already closed (event loop already exited)
                        let _ = transmit_writer.send(None);
                    }
                    if let Ok(mut script) = self.session.lua_script.lock() {
                        script.on_disconnect();
                        script.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                    transmit_writer.take();
                    screen.set_host("", 0)?;
                    screen.clear_tags()?;
                    screen.print_prompt(&Line::from(""));
                }
                Ok(())
            }
            Event::Reconnect => {
                let host = self.session.host();
                let port = self.session.port();
                let tls = self.session.tls();
                let verify = self.session.verify_cert();
                if !host.is_empty() && !port > 0 {
                    self.session
                        .main_writer
                        .send(Event::Connect(Connection::new(&host, port, tls, verify)))?;
                } else {
                    screen.print_error("Reconnect to what?");
                }
                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }

    fn log_line(&self, prefix: &str, line: &Line) -> Result {
        if let Ok(mut logger) = self.session.logger.lock() {
            logger.log_line(prefix, line)?;
        }
        Ok(())
    }

    fn log_str(&self, prefix: &str, line: &str) -> Result {
        if let Ok(mut logger) = self.session.logger.lock() {
            logger.log_str(&format!("{prefix}{line}"))?;
        }
        Ok(())
    }

    fn handle_logging(&self, event: Event) -> Result {
        match event {
            Event::MudOutput(line) | Event::Output(line) => self.log_line("", &line),
            Event::Error(line) => self.log_str("[!!] ", &line),
            Event::Info(line) => self.log_str("[**] ", &line),
            Event::Prompt(prompt) => {
                self.log_line("", &prompt)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn handle_output_events(
        &self,
        event: Event,
        screen: &mut Box<dyn UserInterface>,
    ) -> Result {
        self.handle_logging(event.clone())?;
        match event {
            Event::MudOutput(mut line) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.on_mud_output(&mut line);
                    self.route_and_print(&line, screen);
                    let extra_lines = script.get_output_lines();
                    drop(script);
                    extra_lines.iter().for_each(|l| {
                        self.route_and_print(l, screen);
                    });
                }
                Ok(())
            }
            Event::Output(line) => {
                self.route_and_print(&line, screen);
                Ok(())
            }
            Event::Prompt(mut prompt) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.on_mud_output(&mut prompt);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                screen.print_prompt(&prompt);
                Ok(())
            }
            Event::SetPromptMask(mask) => {
                if let Ok(mut command_buffer) = self.session.command_buffer.lock() {
                    let mut lua_ctx = self.session.lua_script.lock().unwrap();
                    let updated_mask_table = command_buffer.set_mask(mask);
                    lua_ctx.set_prompt_mask_content(updated_mask_table);
                    let mut prompt_input = self.session.prompt_input.lock().unwrap();
                    *prompt_input = command_buffer.get_masked_buffer();
                    // Masked string, masked cursor. `get_pos` indexes the raw
                    // buffer and would point into the wrong place as soon as a
                    // mask inserts anything before the cursor.
                    screen.print_prompt_input(&prompt_input, command_buffer.get_masked_pos());
                }
                Ok(())
            }
            Event::ClearPromptMask => {
                if let Ok(mut command_buffer) = self.session.command_buffer.lock() {
                    command_buffer.clear_mask();
                    if let Ok(mut luascript) = self.session.lua_script.lock() {
                        luascript.set_prompt_mask_content(command_buffer.get_mask());
                    }
                    let mut prompt_input = self.session.prompt_input.lock().unwrap();
                    *prompt_input = command_buffer.get_masked_buffer();
                    // The mask was just cleared, so this is the identity — but
                    // it is spelled the same way as the branch above so the
                    // pairing stays obviously correct.
                    screen.print_prompt_input(&prompt_input, command_buffer.get_masked_pos());
                }
                Ok(())
            }
            Event::UserInputBuffer(input_buffer, pos) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.on_prompt_update(&input_buffer);
                }
                let mut prompt_input = self.session.prompt_input.lock().unwrap();
                *prompt_input = input_buffer;
                screen.print_prompt_input(&prompt_input, pos);
                Ok(())
            }
            Event::UserInputCursor(pos) => {
                let prompt_input = self.session.prompt_input.lock().unwrap();
                screen.print_prompt_input(&prompt_input, pos);
                Ok(())
            }
            Event::Error(msg) => {
                screen.print_error(&msg);
                Ok(())
            }
            Event::Info(msg) => {
                screen.print_info(&msg);
                Ok(())
            }
            Event::ClearTags => {
                screen.clear_tags()?;
                Ok(())
            }
            Event::AddTag(tag) => screen.add_tag(&tag),
            Event::RemoveTag(tag) => screen.remove_tag(&tag),
            Event::TabCommand(cmd) => self.handle_tab_command(cmd, screen),
            _ => Err(BadEventRoutingError.into()),
        }
    }

    /// Apply a tabs control message — see [`TabCommand`].
    pub fn handle_tab_command(
        &self,
        cmd: TabCommand,
        screen: &mut Box<dyn UserInterface>,
    ) -> Result {
        match cmd {
            TabCommand::Create { name, opts } => {
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.create(&name, opts) {
                        screen.print_error(&format!("create_tab({name}): {err}"));
                        return Ok(());
                    }
                }
                self.refresh_tab_indicator(screen);
                Ok(())
            }
            TabCommand::AddFilter { name, pattern } => {
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.add_filter(&name, &pattern) {
                        screen.print_error(&format!("add_tab_filter({name}): {err}"));
                    }
                }
                // Visual state unchanged — no indicator refresh needed.
                Ok(())
            }
            TabCommand::AddExclude { name, pattern } => {
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.add_exclude(&name, &pattern) {
                        screen.print_error(&format!("add_tab_exclude_filter({name}): {err}"));
                    }
                }
                // Visual state unchanged — no indicator refresh needed.
                Ok(())
            }
            TabCommand::SetLabel { name, label } => {
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.set_label(&name, &label) {
                        screen.print_error(&format!("set_tab_label({name}): {err}"));
                        return Ok(());
                    }
                }
                self.refresh_tab_indicator(screen);
                Ok(())
            }
            TabCommand::SetShortcut { name, shortcut } => {
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.set_shortcut(&name, shortcut) {
                        screen.print_error(&format!("set_tab_shortcut({name}): {err}"));
                        return Ok(());
                    }
                }
                self.refresh_tab_indicator(screen);
                Ok(())
            }
            TabCommand::Switch { name } => {
                // Two-phase swap: take destination, hand to screen, return old.
                let dest_history = match self.session.tab_set.lock() {
                    Ok(mut tab_set) => match tab_set.take_for_switch(&name) {
                        Ok(Some(h)) => h,
                        Ok(None) => return Ok(()), // no-op: already active
                        Err(err) => {
                            screen.print_error(&format!("switch_tab({name}): {err}"));
                            return Ok(());
                        }
                    },
                    Err(_) => return Ok(()),
                };
                let old_history = match screen.swap_history(dest_history) {
                    Ok(h) => h,
                    Err(err) => {
                        screen.print_error(&format!("switch_tab({name}): swap failed: {err}"));
                        return Ok(());
                    }
                };
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.complete_switch(old_history) {
                        screen.print_error(&format!("switch_tab({name}): complete failed: {err}"));
                        return Ok(());
                    }
                }
                self.refresh_tab_indicator(screen);
                Ok(())
            }
            TabCommand::OutputTo { name, line } => {
                let (render_to_screen, indicator_dirty) = match self.session.tab_set.lock() {
                    Ok(mut tab_set) => match tab_set.output_to(&name, &line) {
                        Ok(active) => (active, !active),
                        Err(err) => {
                            screen.print_error(&format!("output_to({name}): {err}"));
                            return Ok(());
                        }
                    },
                    Err(_) => return Ok(()),
                };
                if render_to_screen {
                    screen.print_output(&line);
                }
                if indicator_dirty {
                    self.refresh_tab_indicator(screen);
                }
                Ok(())
            }
            TabCommand::Remove { name } => {
                // If removing the active tab, switch back to `main` first so the
                // tab's History returns from the screen into the set, where
                // `remove` can drop it. Reuses the switch protocol.
                let is_active = match self.session.tab_set.lock() {
                    Ok(tab_set) => name == tab_set.active_name(),
                    Err(_) => return Ok(()),
                };
                if is_active {
                    let dest_history = match self.session.tab_set.lock() {
                        Ok(mut tab_set) => match tab_set.take_for_switch(MAIN_TAB) {
                            Ok(history) => history,
                            Err(err) => {
                                screen.print_error(&format!("remove_tab({name}): {err}"));
                                return Ok(());
                            }
                        },
                        Err(_) => return Ok(()),
                    };
                    if let Some(dest_history) = dest_history {
                        let old_history = match screen.swap_history(dest_history) {
                            Ok(h) => h,
                            Err(err) => {
                                screen.print_error(&format!(
                                    "remove_tab({name}): swap failed: {err}"
                                ));
                                return Ok(());
                            }
                        };
                        if let Ok(mut tab_set) = self.session.tab_set.lock() {
                            if let Err(err) = tab_set.complete_switch(old_history) {
                                screen.print_error(&format!("remove_tab({name}): {err}"));
                                return Ok(());
                            }
                        }
                    }
                }
                if let Ok(mut tab_set) = self.session.tab_set.lock() {
                    if let Err(err) = tab_set.remove(&name) {
                        screen.print_error(&format!("remove_tab({name}): {err}"));
                        return Ok(());
                    }
                }
                self.refresh_tab_indicator(screen);
                Ok(())
            }
        }
    }

    pub fn handle_scroll_events(
        &self,
        event: Event,
        screen: &mut Box<dyn UserInterface>,
    ) -> Result {
        match event {
            Event::ScrollLock(enabled) => {
                screen.scroll_lock(enabled)?;
                Ok(())
            }
            Event::ScrollUp => {
                screen.scroll_up()?;
                Ok(())
            }
            Event::ScrollDown => {
                screen.scroll_down()?;
                Ok(())
            }
            Event::ScrollTop => {
                screen.scroll_top()?;
                Ok(())
            }
            Event::ScrollBottom => {
                screen.reset_scroll()?;
                Ok(())
            }
            Event::FindForward(pattern) => {
                screen.find_down(&pattern)?;
                Ok(())
            }
            Event::FindBackward(pattern) => {
                screen.find_up(&pattern)?;
                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }
}

pub(crate) fn spawn_quit_confirm_timeout_thread(
    writer: Sender<Event>,
    timeout: time::Duration,
) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("quit-confirm-timeout-thread".to_string())
        .spawn(move || {
            thread::sleep(timeout);
            writer.send(Event::QuitConfirmTimeout).unwrap();
        })
}

#[cfg(test)]
mod event_test {

    use std::sync::{Arc, Mutex};

    use mockall::predicate::eq;

    use crate::{model::Regex, session::SessionBuilder, timer::TimerEvent};

    use crate::io::MockLogWriter;
    use crate::ui::MockUserInterface;

    use super::*;

    fn build_session() -> (Session, Receiver<Event>, Receiver<TimerEvent>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let (timer_writer, timer_reader): (Sender<TimerEvent>, Receiver<TimerEvent>) = channel();
        let session = SessionBuilder::new()
            .main_writer(writer)
            .timer_writer(timer_writer)
            .screen_dimensions((80, 80))
            .build();

        loop {
            if reader.try_recv().is_err() {
                break;
            }
        }

        (session, reader, timer_reader)
    }

    #[test]
    fn test_event_logging() {
        let (mut session, _reader, _timer_reader) = build_session();
        let mut logger = MockLogWriter::new();
        logger
            .expect_log_str()
            .with(eq("prefix test line"))
            .returning(|_| Ok(()));
        logger
            .expect_log_line()
            .with(eq("prefix "), eq(Line::from("test line")))
            .returning(|_, _| Ok(()));
        session.logger = Arc::new(Mutex::new(logger));
        let handler = EventHandler::from(&session);
        let _ = handler.log_str("prefix ", "test line");
        let _ = handler.log_line("prefix ", &Line::from("test line"));
    }

    #[test]
    fn test_scrolling() {
        let (session, _reader, _timer_reader) = build_session();
        let mut screen = MockUserInterface::new();
        screen.expect_scroll_up().times(1).returning(|| Ok(()));
        screen.expect_scroll_top().times(1).returning(|| Ok(()));
        screen.expect_scroll_down().times(1).returning(|| Ok(()));
        screen.expect_reset_scroll().times(1).returning(|| Ok(()));
        screen
            .expect_scroll_lock()
            .times(1)
            .with(eq(true))
            .returning(|_| Ok(()));
        screen
            .expect_scroll_lock()
            .times(1)
            .with(eq(false))
            .returning(|_| Ok(()));
        let handler = EventHandler::from(&session);
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        assert!(handler
            .handle_scroll_events(Event::ScrollUp, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollDown, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollTop, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollBottom, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollLock(true), &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollLock(false), &mut screen)
            .is_ok());
    }

    #[test]
    fn test_find() {
        let (session, _reader, _timer_reader) = build_session();
        let re = Regex::new("test", None).unwrap();
        let mut screen = MockUserInterface::new();
        screen
            .expect_find_down()
            .times(1)
            .withf(|other| *other == Regex::new("test", None).unwrap())
            .returning(|_| Ok(()));
        screen
            .expect_find_up()
            .times(1)
            .withf(|other| *other == Regex::new("test", None).unwrap())
            .returning(|_| Ok(()));
        let handler = EventHandler::from(&session);
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        assert!(handler
            .handle_scroll_events(Event::FindBackward(re.clone()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::FindForward(re), &mut screen)
            .is_ok());
    }

    #[test]
    fn test_output() {
        let (mut session, _reader, _timer_reader) = build_session();
        let mut logger = MockLogWriter::new();
        logger.expect_log_line().times(3).returning(|_, _| Ok(()));
        logger.expect_log_str().times(2).returning(|_| Ok(()));
        session.logger = Arc::new(Mutex::new(logger));
        let handler = EventHandler::from(&session);

        let mut screen = MockUserInterface::new();
        screen
            .expect_print_output()
            .with(eq(Line::from("Output line")))
            .times(2)
            .return_const(());
        screen.expect_print_prompt().times(1).return_const(());
        screen.expect_print_prompt_input().times(1).return_const(());
        screen.expect_print_error().times(1).return_const(());
        screen.expect_print_info().times(1).return_const(());

        let line = Line::from("Output line");
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        assert!(handler
            .handle_output_events(Event::MudOutput(line.clone()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Output(line), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Prompt(Line::from("")), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(
                Event::UserInputBuffer(String::from("prompt"), 5),
                &mut screen
            )
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Info("info message".to_string()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Error("error message".to_string()), &mut screen)
            .is_ok());
    }

    #[test]
    fn test_spawn_quit_confirm_timeout_thread() {
        let (session, reader, _) = build_session();

        let handle = spawn_quit_confirm_timeout_thread(
            session.main_writer.clone(),
            time::Duration::from_millis(500),
        )
        .expect("unexpected err spawning quit confirm thread");

        handle
            .join()
            .expect("failed to join on quit confirm thread");
        let event = reader.recv().expect("failed to recv event");
        assert_eq!(event, Event::QuitConfirmTimeout);
    }

    #[test]
    fn test_no_echo() {
        let (mut session, _reader, _) = build_session();

        // We expect log lines to be generated twice: both with and without
        // the echo_input setting enabled.
        let mut logger = MockLogWriter::new();
        logger.expect_log_line().times(2).returning(|_, _| Ok(()));
        session.logger = Arc::new(Mutex::new(logger));

        let input_line = Line::from("Input line");
        let mut screen = MockUserInterface::new();
        // We only expect print_send() to be called on the UI only **one** time,
        // when the echo_input setting is enabled.
        screen
            .expect_print_send()
            .with(eq(input_line.clone()))
            .times(1)
            .return_const(());

        let mut handler = EventHandler::from(&session);
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        let mut send_event = || {
            assert!(handler
                .handle_server_events(
                    Event::ServerInput(input_line.clone()),
                    &mut screen,
                    &mut None
                )
                .is_ok());
        };

        session.echo_input.store(true, Ordering::Relaxed);
        send_event();
        session.echo_input.store(false, Ordering::Relaxed);
        send_event();
    }
}
