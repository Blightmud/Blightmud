use super::{constants::*, regex::Regex, ui_event::UiEvent};
use crate::event::{Event, QuitMethod, TabCommand};
use crate::io::SaveData;
use crate::tabs::{TabOpts, TabSet};
use crate::ui::{TopPrefix, TopPrefixStyle, TopRowBody, TopRowOpts, TopRowSelector};
use crate::{
    model::{self, Line, TagMask},
    tools::printable_chars::PrintableCharsIterator,
    PROJECT_NAME, VERSION,
};
use log::debug;
use mlua::{
    AnyUserData, FromLua, Function, Result as LuaResult, Table, UserData, UserDataMethods, Variadic,
};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Clone, FromLua)]
pub struct Blight {
    main_writer: Sender<Event>,
    output_lines: Vec<Line>,
    ui_events: Vec<UiEvent>,
    pub screen_dimensions: (u16, u16),
    pub core_mode: bool,
    pub reader_mode: bool,
    pub _tts_enabled: bool,
    tag_mask: TagMask,
    history_capacity: usize,
    /// Reference to the session's TabSet for read-only Lua access via
    /// `blight.tabs()` / `blight.active_tab()`. None in unit tests.
    tab_set: Option<Arc<Mutex<TabSet>>>,
}

impl Blight {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
            ui_events: vec![],
            screen_dimensions: (0, 0),
            core_mode: false,
            reader_mode: false,
            _tts_enabled: false,
            tag_mask: TagMask::default(),
            history_capacity: 32768,
            tab_set: None,
        }
    }

    pub fn set_tab_set(&mut self, tab_set: Arc<Mutex<TabSet>>) {
        self.tab_set = Some(tab_set);
    }

    /// Internal helper for `LuaScript::reset` so the new Lua state can
    /// inherit the same `TabSet` reference without going through the
    /// builder chain again.
    pub fn tab_set_ref(&self) -> Option<Arc<Mutex<TabSet>>> {
        self.tab_set.clone()
    }

    pub fn core_mode(&mut self, mode: bool) {
        self.core_mode = mode;
    }

    pub fn get_output_lines(&mut self) -> Vec<Line> {
        let return_lines = self.output_lines.clone();
        self.output_lines.clear();
        return_lines
    }

    pub fn get_ui_events(&mut self) -> Vec<UiEvent> {
        let events = self.ui_events.clone();
        self.ui_events.clear();
        events
    }
}

/// Accept either a Lua integer (0-based row index) or a string (row name)
/// as a row selector.
fn top_row_selector_from_lua(value: &mlua::Value) -> LuaResult<TopRowSelector> {
    match value {
        mlua::Value::Integer(i) => Ok(TopRowSelector::Index((*i).max(0) as usize)),
        mlua::Value::Number(n) => Ok(TopRowSelector::Index((*n).max(0.0) as usize)),
        mlua::Value::String(s) => Ok(TopRowSelector::Name(s.to_str()?.to_string())),
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "TopRowSelector".to_string(),
            message: Some("expected integer index or string name".to_string()),
        }),
    }
}

/// Parse a Lua table into [`TopRowOpts`]. Recognized fields:
///   - `name`: string — rename the row
///   - `bar_char`: string (single char) — bar fill character
///   - `prefix`: nil | string | table — `nil` clears, string ⇒ Plain style;
///     table `{text, style}` for full control
///   - `body`: nil | string — `nil` ⇒ Empty, string ⇒ Text
///   - `visible`: boolean — show / hide the row
fn top_row_opts_from_lua(table: &mlua::Table) -> LuaResult<TopRowOpts> {
    let mut opts = TopRowOpts::default();
    if let Ok(name) = table.get::<String>("name") {
        opts.name = Some(name);
    }
    if let Ok(c) = table.get::<String>("bar_char") {
        let ch = c.chars().next().ok_or_else(|| {
            mlua::Error::RuntimeError("bar_char must be a single character".to_string())
        })?;
        opts.bar_char = Some(ch);
    }
    if table.contains_key("prefix")? {
        let value: mlua::Value = table.get("prefix")?;
        opts.prefix = Some(top_prefix_from_lua(&value)?);
    }
    if table.contains_key("body")? {
        let value: mlua::Value = table.get("body")?;
        opts.body = Some(match value {
            mlua::Value::Nil => TopRowBody::Empty,
            mlua::Value::String(s) => TopRowBody::Text(s.to_str()?.to_string()),
            other => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: other.type_name(),
                    to: "TopRowBody".to_string(),
                    message: Some("expected nil or string".to_string()),
                });
            }
        });
    }
    if let Ok(v) = table.get::<bool>("visible") {
        opts.visible = Some(v);
    }
    Ok(opts)
}

fn top_prefix_from_lua(value: &mlua::Value) -> LuaResult<Option<TopPrefix>> {
    match value {
        mlua::Value::Nil => Ok(None),
        mlua::Value::String(s) => Ok(Some(TopPrefix {
            text: s.to_str()?.to_string(),
            style: TopPrefixStyle::Plain,
        })),
        mlua::Value::Table(t) => {
            let text: String = t.get("text").unwrap_or_default();
            let style: String = t.get("style").unwrap_or_else(|_| "plain".to_string());
            let style = match style.as_str() {
                "brand" | "Brand" => TopPrefixStyle::Brand,
                _ => TopPrefixStyle::Plain,
            };
            Ok(Some(TopPrefix { text, style }))
        }
        other => Err(mlua::Error::FromLuaConversionError {
            from: other.type_name(),
            to: "TopPrefix".to_string(),
            message: Some("expected nil, string, or table {text, style}".to_string()),
        }),
    }
}

impl UserData for Blight {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("output", |ctx, strings: Variadic<String>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.output_lines.push(Line::from(strings.join(" ")));
            Ok(())
        });
        methods.add_function("terminal_dimensions", |ctx, _: ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.screen_dimensions)
        });
        methods.add_function("bind", |ctx, (cmd, callback): (String, mlua::Function)| {
            let bind_table: mlua::Table = ctx.named_registry_value(COMMAND_BINDING_TABLE)?;
            if cmd.to_lowercase().starts_with("alt-") {
                let (_, right) = cmd.split_at(3);
                let mut cmd = "alt".to_string();
                cmd.push_str(right);
                bind_table.set(cmd, callback)?;
            } else {
                bind_table.set(cmd.to_lowercase(), callback)?;
            }
            Ok(())
        });
        methods.add_function("unbind", |ctx, cmd: String| {
            let bind_table: mlua::Table = ctx.named_registry_value(COMMAND_BINDING_TABLE)?;
            bind_table.set(cmd, mlua::Nil)?;
            Ok(())
        });
        methods.add_function("ui", |ctx, cmd: String| -> mlua::Result<()> {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            let event: UiEvent = UiEvent::from(cmd.as_str());
            if let UiEvent::Unknown(cmd) = event {
                this.main_writer
                    .send(Event::Error(format!("Invalid ui command: {cmd}")))
                    .unwrap();
            } else {
                this.ui_events.push(event);
            }
            Ok(())
        });
        methods.add_function("debug", |_, strings: Variadic<String>| {
            debug!("{}", strings.join(" "));
            Ok(())
        });
        methods.add_function("is_core_mode", |ctx, ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.core_mode)
        });
        methods.add_function("is_reader_mode", |ctx, ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.reader_mode)
        });
        methods.add_function("status_height", |ctx, requested: Option<u16>| {
            let height: u16 = if let Some(height) = requested {
                let height = height.clamp(0, 5);
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                this.main_writer
                    .send(Event::StatusAreaHeight(height))
                    .unwrap();
                ctx.set_named_registry_value(STATUS_AREA_HEIGHT, height)?;
                height
            } else {
                ctx.named_registry_value(STATUS_AREA_HEIGHT)?
            };
            Ok(height)
        });
        methods.add_function("status_line", |ctx, (index, line): (usize, String)| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::StatusLine(index, line))
                .unwrap();
            Ok(())
        });
        methods.add_function("top_line", |ctx, line: Option<String>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer.send(Event::TopLine(line)).unwrap();
            Ok(())
        });
        methods.add_function(
            "set_top_row",
            |ctx, (selector, opts): (mlua::Value, mlua::Table)| {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                let sel = top_row_selector_from_lua(&selector)?;
                let opts = top_row_opts_from_lua(&opts)?;
                this.main_writer.send(Event::SetTopRow(sel, opts)).unwrap();
                Ok(())
            },
        );
        methods.add_function("reset_top_row", |ctx, selector: mlua::Value| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            let sel = top_row_selector_from_lua(&selector)?;
            this.main_writer.send(Event::ResetTopRow(sel)).unwrap();
            Ok(())
        });
        methods.add_function("add_top_row", |ctx, opts: mlua::Table| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            let opts = top_row_opts_from_lua(&opts)?;
            this.main_writer.send(Event::AddTopRow(opts)).unwrap();
            Ok(())
        });
        methods.add_function("remove_top_row", |ctx, selector: mlua::Value| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            let sel = top_row_selector_from_lua(&selector)?;
            this.main_writer.send(Event::RemoveTopRow(sel)).unwrap();
            Ok(())
        });
        methods.add_function("top_rows", |ctx, _: ()| -> LuaResult<Table> {
            // Built-in row names are always present and stable. Lua-added
            // rows are tracked by the script itself.
            let arr = ctx.create_table()?;
            arr.push("tab_indicator")?;
            arr.push("host_status")?;
            Ok(arr)
        });
        methods.add_function("version", |_, _: ()| -> LuaResult<(&str, &str)> {
            Ok((PROJECT_NAME, VERSION))
        });
        methods.add_function("config_dir", |_, ()| -> mlua::Result<String> {
            Ok(crate::CONFIG_DIR.to_string_lossy().to_string())
        });
        methods.add_function("data_dir", |_, ()| -> mlua::Result<String> {
            Ok(crate::DATA_DIR.to_string_lossy().to_string())
        });
        methods.add_function("on_quit", |ctx, func: Function| -> mlua::Result<()> {
            let table: Table = ctx.named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)?;
            table.set(table.raw_len() + 1, func)?;
            Ok(())
        });
        methods.add_function(
            "on_complete",
            |ctx, func: mlua::Function| -> mlua::Result<()> {
                let table: Table = ctx.named_registry_value(COMPLETION_CALLBACK_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
        methods.add_function(
            "on_dimensions_change",
            |ctx, func: Function| -> mlua::Result<()> {
                let table: Table =
                    ctx.named_registry_value(BLIGHT_ON_DIMENSIONS_CHANGE_LISTENER_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
        methods.add_function("quit", |ctx, ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::Quit(QuitMethod::Script))
                .unwrap();
            Ok(())
        });
        methods.add_function("show_help", |ctx, (name, lock_scroll): (String, bool)| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::ShowHelp(name, lock_scroll))
                .unwrap();
            Ok(())
        });
        methods.add_function(
            "show_tags",
            |ctx, val: Option<bool>| -> mlua::Result<bool> {
                if let Some(val) = val {
                    let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                    let this = this_aux.borrow::<Blight>()?;
                    this.main_writer.send(Event::ShowTags(val)).unwrap();
                    ctx.set_named_registry_value(SHOW_TAGS, val)?;
                }
                Ok(ctx.named_registry_value(SHOW_TAGS).unwrap_or(false))
            },
        );
        methods.add_function("filter_tag_color", |ctx, color: Option<String>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.tag_mask.color = color.filter(|c| c.as_str().display_width() == 0);
            this.main_writer
                .send(Event::SetTagMask(this.tag_mask.clone()))
                .unwrap();
            Ok(())
        });
        methods.add_function("filter_tag_key", |ctx, key: Option<String>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.tag_mask.key = key;
            this.main_writer
                .send(Event::SetTagMask(this.tag_mask.clone()))
                .unwrap();
            Ok(())
        });
        methods.add_function("filter_tag_symbol", |ctx, symbol: Option<String>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.tag_mask.symbol = symbol.and_then(|s| s.chars().next());
            this.main_writer
                .send(Event::SetTagMask(this.tag_mask.clone()))
                .unwrap();
            Ok(())
        });
        methods.add_function("filter_tag_reverse", |ctx, val: Option<bool>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            if let Some(val) = val {
                this.tag_mask.reverse = val;
                this.main_writer
                    .send(Event::SetTagMask(this.tag_mask.clone()))
                    .unwrap();
            }
            Ok(this.tag_mask.reverse)
        });
        methods.add_function("filter_tag_reset", |ctx, ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.tag_mask = TagMask::default();
            this.main_writer
                .send(Event::SetTagMask(this.tag_mask.clone()))
                .unwrap();
            Ok(())
        });
        methods.add_function("history_capacity", |ctx, capacity: Option<usize>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            if let Some(capacity) = capacity {
                this.history_capacity = capacity;
                this.main_writer
                    .send(Event::SetHistoryCapacity(capacity))
                    .unwrap();
            }
            Ok(this.history_capacity)
        });
        methods.add_function("find_backward", |ctx, re: Regex| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::FindBackward(re.regex))
                .unwrap();
            Ok(())
        });
        methods.add_function("find_forward", |ctx, re: Regex| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer.send(Event::FindForward(re.regex)).unwrap();
            Ok(())
        });

        // ---- Tabs API ----
        //
        // Scripts can create named scrollable output buffers ("tabs") with
        // their own filters, then bind keys to switch between them. The
        // implicit `main` tab always exists; `blight.output(...)` continues
        // to route there. Lines that match a tab's filter are mirrored into
        // that tab in addition to main (unless the tab has gag_main=true,
        // in which case the line skips main entirely).
        //
        // See `/help tabs` for usage.

        methods.add_function(
            "create_tab",
            |ctx, (name, opts): (String, Option<Table>)| -> mlua::Result<()> {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                let tab_opts = if let Some(t) = opts {
                    TabOpts {
                        label: t.get::<Option<String>>("label").unwrap_or(None),
                        shortcut: t.get::<Option<String>>("shortcut").unwrap_or(None),
                        gag_main: t
                            .get::<Option<bool>>("gag_main")
                            .unwrap_or(None)
                            .unwrap_or(false),
                    }
                } else {
                    TabOpts::default()
                };
                this.main_writer
                    .send(Event::TabCommand(TabCommand::Create {
                        name,
                        opts: tab_opts,
                    }))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );

        methods.add_function("switch_tab", |ctx, name: String| -> mlua::Result<()> {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::TabCommand(TabCommand::Switch { name }))
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_function(
            "add_tab_filter",
            |ctx, (name, pattern): (String, String)| -> mlua::Result<()> {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                this.main_writer
                    .send(Event::TabCommand(TabCommand::AddFilter { name, pattern }))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );

        // Append a regex exclude to a tab. Lines matching the exclude are
        // NOT routed to the tab even when an `add_tab_filter` pattern also
        // matches — a blocklist hole inside a broad include rule. Useful
        // because Rust's `regex` crate has no lookaround, so a single
        // include regex can't say "match X but not Y".
        methods.add_function(
            "add_tab_exclude_filter",
            |ctx, (name, pattern): (String, String)| -> mlua::Result<()> {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                this.main_writer
                    .send(Event::TabCommand(TabCommand::AddExclude { name, pattern }))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );

        methods.add_function(
            "set_tab_label",
            |ctx, (name, label): (String, String)| -> mlua::Result<()> {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                this.main_writer
                    .send(Event::TabCommand(TabCommand::SetLabel { name, label }))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );

        // Set or clear the keyboard-shortcut hint shown in the tab
        // indicator. Pass `nil` (or no second arg) to clear an existing
        // hint. Display-only — Blightmud does NOT bind the key for you.
        methods.add_function(
            "set_tab_shortcut",
            |ctx, (name, shortcut): (String, Option<String>)| -> mlua::Result<()> {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                this.main_writer
                    .send(Event::TabCommand(TabCommand::SetShortcut {
                        name,
                        shortcut,
                    }))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );

        methods.add_function(
            "output_to",
            |ctx, (name, strings): (String, Variadic<String>)| -> mlua::Result<()> {
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                let line = Line::from(strings.join(" "));
                this.main_writer
                    .send(Event::TabCommand(TabCommand::OutputTo { name, line }))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );

        // Read-only introspection: snapshot of the current tab set.
        // Returns an array of tables: { {name, label, unread, active}, ... }
        methods.add_function("tabs", |ctx, _: ()| -> mlua::Result<mlua::Table> {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            let arr = ctx.create_table()?;
            if let Some(ts) = &this.tab_set {
                if let Ok(set) = ts.lock() {
                    for (i, tab) in set.list().into_iter().enumerate() {
                        let entry = ctx.create_table()?;
                        entry.set("name", tab.name)?;
                        entry.set("label", tab.label)?;
                        // `shortcut` may be nil — set unconditionally; mlua maps
                        // Option<String> → Lua nil when None.
                        entry.set("shortcut", tab.shortcut)?;
                        entry.set("unread", tab.unread)?;
                        entry.set("active", tab.active)?;
                        arr.set(i + 1, entry)?;
                    }
                }
            }
            Ok(arr)
        });

        // Returns the name of the currently-active tab, or "main" if no
        // tab set is bound (defensive — should only happen in unit tests).
        methods.add_function("active_tab", |ctx, _: ()| -> mlua::Result<String> {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            if let Some(ts) = &this.tab_set {
                if let Ok(set) = ts.lock() {
                    return Ok(set.active_name().to_string());
                }
            }
            Ok("main".to_string())
        });

        // Switch tab indicator placement.
        //   "row"    — dedicated indicator row above the host topbar (default)
        //   "inline" — tabs render alongside `host:port [tags]` on the topbar,
        //              reclaiming the dedicated row
        // Persists to settings.ron (equivalent to setting `tab_indicator_inline`
        // via `/set`) and triggers an immediate layout refresh.
        methods.add_function(
            "set_tab_indicator_position",
            |ctx, position: String| -> mlua::Result<()> {
                let inline = match position.as_str() {
                    "inline" => true,
                    "row" => false,
                    other => {
                        return Err(mlua::Error::external(format!(
                            "set_tab_indicator_position: expected \"row\" or \"inline\", got \"{other}\""
                        )));
                    }
                };
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                let mut settings =
                    model::Settings::try_load().map_err(mlua::Error::external)?;
                settings
                    .set(model::TAB_INDICATOR_INLINE, inline)
                    .map_err(mlua::Error::external)?;
                settings.save();
                this.main_writer
                    .send(Event::SettingChanged(
                        model::TAB_INDICATOR_INLINE.to_string(),
                        inline,
                    ))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        );
    }
}

#[cfg(test)]
mod test_blight {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use mlua::{AnyUserData, Lua};

    use crate::event::{Event, QuitMethod};
    use crate::lua::UiEvent;

    use super::Blight;
    use crate::lua::constants::{
        BLIGHT_ON_DIMENSIONS_CHANGE_LISTENER_TABLE, BLIGHT_ON_QUIT_LISTENER_TABLE,
        COMMAND_BINDING_TABLE, COMPLETION_CALLBACK_TABLE, SHOW_TAGS, STATUS_AREA_HEIGHT,
    };
    use crate::{PROJECT_NAME, VERSION};

    fn get_lua_state() -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let regex = crate::lua::regex::RegexLib {};
        let blight = Blight::new(writer);
        let lua = Lua::new();
        lua.globals().set("regex", regex).unwrap();
        lua.globals().set("blight", blight).unwrap();
        lua.set_named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(
            BLIGHT_ON_DIMENSIONS_CHANGE_LISTENER_TABLE,
            lua.create_table().unwrap(),
        )
        .unwrap();
        lua.set_named_registry_value(COMPLETION_CALLBACK_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(COMMAND_BINDING_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(STATUS_AREA_HEIGHT, 1u16)
            .unwrap();
        lua.set_named_registry_value(SHOW_TAGS, false).unwrap();
        (lua, reader)
    }

    #[test]
    fn test_config_dir() {
        let (lua, _reader) = get_lua_state();
        assert!(lua
            .load("return blight.config_dir()")
            .call::<String>(())
            .unwrap()
            .ends_with(".run/test/config"));
    }

    #[test]
    fn test_data_dir() {
        let (lua, _reader) = get_lua_state();
        assert!(lua
            .load("return blight.data_dir()")
            .call::<String>(())
            .unwrap()
            .ends_with(".run/test/data"));
    }

    #[test]
    fn test_version() {
        let (lua, _reader) = get_lua_state();
        assert_eq!(
            lua.load("return blight.version()")
                .call::<(String, String)>(())
                .unwrap(),
            (PROJECT_NAME.to_string(), VERSION.to_string())
        );
    }

    #[test]
    fn confirm_on_quite_register() {
        let (lua, _reader) = get_lua_state();
        let table: mlua::Table = lua
            .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
            .unwrap();
        assert_eq!(table.raw_len(), 0);
        lua.load("blight.on_quit(function () end)").exec().unwrap();
        let table: mlua::Table = lua
            .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
            .unwrap();
        assert_eq!(table.raw_len(), 1);
    }

    #[test]
    fn on_complete() {
        let (lua, _reader) = get_lua_state();
        let table: mlua::Table = lua.named_registry_value(COMPLETION_CALLBACK_TABLE).unwrap();
        assert_eq!(table.raw_len(), 0);
        lua.load("blight.on_complete(function () end)")
            .exec()
            .unwrap();
        let table: mlua::Table = lua.named_registry_value(COMPLETION_CALLBACK_TABLE).unwrap();
        assert_eq!(table.raw_len(), 1);
    }

    #[test]
    fn on_quit_function() {
        let (lua, _reader) = get_lua_state();
        lua.load("blight.on_quit(function () blight.output(\"on_quit\") end)")
            .exec()
            .unwrap();
        let table: mlua::Table = lua
            .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
            .unwrap();
        for pair in table.pairs::<mlua::Value, mlua::Function>() {
            let (_, cb) = pair.unwrap();
            cb.call::<()>(()).unwrap();
        }
        let blight_aux = lua.globals().get::<AnyUserData>("blight").unwrap();
        let mut blight = blight_aux.borrow_mut::<Blight>().unwrap();
        let lines = blight.get_output_lines();
        let mut it = lines.iter();
        assert_eq!(it.next().unwrap(), &crate::model::Line::from("on_quit"));
    }

    #[test]
    fn quit() {
        let (lua, reader) = get_lua_state();
        lua.load("blight.quit()").exec().unwrap();
        assert_eq!(reader.recv(), Ok(Event::Quit(QuitMethod::Script)));
    }

    #[test]
    fn find() {
        let (lua, reader) = get_lua_state();
        let re = crate::model::Regex::new("test", None).unwrap();
        lua.load(r#"blight.find_forward(regex.new("test"))"#)
            .exec()
            .unwrap();
        assert_eq!(reader.recv(), Ok(Event::FindForward(re.clone())));
        lua.load(r#"blight.find_backward(regex.new("test"))"#)
            .exec()
            .unwrap();
        assert_eq!(reader.recv(), Ok(Event::FindBackward(re)));
    }

    #[test]
    fn show_help() {
        let (lua, reader) = get_lua_state();
        lua.load("blight.show_help(\"test1\", false)")
            .exec()
            .unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::ShowHelp("test1".to_string(), false))
        );
        lua.load("blight.show_help(\"test2\", true)")
            .exec()
            .unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::ShowHelp("test2".to_string(), true))
        );
    }

    #[test]
    fn confirm_ui_events() {
        let (lua, _) = get_lua_state();
        lua.load("blight.ui(\"step_left\")").exec().unwrap();
        lua.load("blight.ui(\"step_right\")").exec().unwrap();
        lua.load("blight.ui(\"scroll_up\")").exec().unwrap();
        lua.load("blight.ui(\"scroll_down\")").exec().unwrap();

        let mut blight: Blight = lua.globals().get("blight").unwrap();
        assert_eq!(
            blight.get_ui_events(),
            vec![
                UiEvent::StepLeft,
                UiEvent::StepRight,
                UiEvent::ScrollUp,
                UiEvent::ScrollDown
            ]
        );
    }

    #[test]
    fn test_bad_ui_event() {
        let (lua, reader) = get_lua_state();
        lua.load("blight.ui(\"schplort\")").exec().unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::Error("Invalid ui command: schplort".to_string()))
        );
    }

    #[test]
    fn test_command_bindings() {
        let (lua, _) = get_lua_state();
        lua.load("blight.bind(\"f1\", function () end)")
            .exec()
            .unwrap();
        let bindings: mlua::Table = lua.named_registry_value(COMMAND_BINDING_TABLE).unwrap();
        assert!(bindings.get::<mlua::Function>("f1").is_ok());
        lua.load("blight.unbind(\"f1\")").exec().unwrap();
        assert!(bindings.get::<mlua::Function>("f1").is_err());
    }

    #[test]
    fn test_command_bindings_alt_with_capitalized_letter() {
        let (lua, _) = get_lua_state();
        lua.load("blight.bind(\"Alt-H\", function () end)")
            .exec()
            .unwrap();
        let bindings: mlua::Table = lua.named_registry_value(COMMAND_BINDING_TABLE).unwrap();
        assert!(bindings.get::<mlua::Function>("alt-H").is_ok());
        assert!(bindings.get::<mlua::Function>("alt-h").is_err());
    }

    #[test]
    fn test_show_tags() {
        let (lua, reader) = get_lua_state();

        // Default is false
        let val = lua
            .load("return blight.show_tags()")
            .call::<bool>(())
            .unwrap();
        assert!(!val);

        // Set to true
        let val = lua
            .load("return blight.show_tags(true)")
            .call::<bool>(())
            .unwrap();
        assert!(val);
        assert_eq!(reader.recv(), Ok(Event::ShowTags(true)));

        // Getter reflects update
        let val = lua
            .load("return blight.show_tags()")
            .call::<bool>(())
            .unwrap();
        assert!(val);

        // Set back to false
        lua.load("blight.show_tags(false)").exec().unwrap();
        assert_eq!(reader.recv(), Ok(Event::ShowTags(false)));
    }

    #[test]
    fn test_filter_tag_reverse() {
        let (lua, reader) = get_lua_state();

        // Default is false
        let val = lua
            .load("return blight.filter_tag_reverse()")
            .call::<bool>(())
            .unwrap();
        assert!(!val);

        // Set to true
        let val = lua
            .load("return blight.filter_tag_reverse(true)")
            .call::<bool>(())
            .unwrap();
        assert!(val);
        // Discard the SetTagMask event
        let _ = reader.recv();

        // Getter reflects update
        let val = lua
            .load("return blight.filter_tag_reverse()")
            .call::<bool>(())
            .unwrap();
        assert!(val);

        // Reset clears reverse too
        lua.load("blight.filter_tag_reset()").exec().unwrap();
        let _ = reader.recv();
        let val = lua
            .load("return blight.filter_tag_reverse()")
            .call::<bool>(())
            .unwrap();
        assert!(!val);
    }

    #[test]
    fn test_status_height() {
        let (lua, _reader) = get_lua_state();
        let height = lua
            .load("return blight.status_height()")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 1);
        let height = lua
            .load("return blight.status_height(3)")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 3);
        let height = lua
            .load("return blight.status_height()")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 3);
        let height = lua
            .load("return blight.status_height(1)")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 1);
        let height = lua
            .load("return blight.status_height()")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 1);
    }

    #[test]
    fn test_history_capacity() {
        let (lua, reader) = get_lua_state();

        // Default value
        let cap = lua
            .load("return blight.history_capacity()")
            .call::<usize>(())
            .unwrap();
        assert_eq!(cap, 32768);

        // Set new value
        let cap = lua
            .load("return blight.history_capacity(5000)")
            .call::<usize>(())
            .unwrap();
        assert_eq!(cap, 5000);
        assert_eq!(reader.recv(), Ok(Event::SetHistoryCapacity(5000)));

        // Getter reflects update
        let cap = lua
            .load("return blight.history_capacity()")
            .call::<usize>(())
            .unwrap();
        assert_eq!(cap, 5000);
    }
}
