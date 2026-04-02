---@meta

--------------------------------------------------------------------------------
-- Line ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---A MUD output/input line passed to trigger and alias callbacks.
---@class Line
Line = {}

---Returns the clean (ANSI-stripped) line content.
---@return string
function Line:line() end

---Returns the raw line content including ANSI escape codes.
---@return string
function Line:raw() end

---Gets or sets the gag flag. When gagged the line is not displayed.
---@param value? boolean
---@return boolean
function Line:gag(value) end

---Gets or sets the TTS gag flag.
---@param value? boolean
---@return boolean
function Line:tts_gag(value) end

---Gets or sets the TTS interrupt flag.
---@param value? boolean
---@return boolean
function Line:tts_interrupt(value) end

---Gets or sets the skip-log flag.
---@param value? boolean
---@return boolean
function Line:skip_log(value) end

---Returns true if this line is a prompt line.
---@return boolean
function Line:prompt() end

---Gets or sets the matched flag.
---@param value? boolean
---@return boolean
function Line:matched(value) end

---Replaces the line content with the given string.
---@param text string
function Line:replace(text) end

---Returns the source of the line.
---Possible values: `"user"` (typed by the user), `"script"` (sent via mud.send/mud.input), or nil.
---@return "user"|"script"|nil
function Line:source() end

---Returns the pending replacement text set by replace(), or nil.
---@return string|nil
function Line:replacement() end

---Gets or sets the ANSI color code used to render this line's tag symbol.
---When set, the tag symbol and a trailing space are rendered in that color before the line content.
---When empty, two plain spaces are rendered instead.
---@param color? string ANSI escape sequence, e.g. `"\x1b[31m"` for red.
---@return string
function Line:tag_color(color) end

---Gets or sets an arbitrary key associated with this line's tag.
---@param key? string
---@return string
function Line:tag_key(key) end

---Gets or sets the character used as the tag symbol. Defaults to `┃` (U+2503).
---Only rendered when a tag color is set.
---@param symbol? string
---@return string
function Line:tag_symbol(symbol) end

--------------------------------------------------------------------------------
-- Regex -----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Options table for regex.new().
---@class RegexOptions
---@field case_insensitive? boolean     Case-insensitive matching (flag `i`).
---@field multi_line? boolean           `^`/`$` match line boundaries (flag `m`).
---@field dot_matches_new_line? boolean `.` also matches `\n` (flag `s`).
---@field swap_greed? boolean           Swap greedy/lazy quantifiers (flag `U`).
---@field ignore_whitespace? boolean    Ignore whitespace and allow `#` comments (flag `x`).

---A compiled regular expression object.
---@class Regex
Regex = {}

---Returns true if the pattern matches anywhere in src.
---@param src string
---@return boolean
function Regex:test(src) end

---Returns capture groups if the pattern matches src, or nil.
---Index 1 is the full match; subsequent indices are capture groups.
---@param src string
---@return string[]|nil
function Regex:match(src) end

---Returns all non-overlapping matches in src, or nil if none found.
---Each entry is a string array where index 1 is the full match.
---@param src string
---@return string[][]|nil
function Regex:match_all(src) end

---Replaces up to count occurrences (0 = all) of the pattern in src.
---@param src string
---@param replace string
---@param count? integer  Number of replacements (0 or omitted = all).
---@return string
function Regex:replace(src, replace, count) end

---Returns the pattern string this Regex was compiled from.
---@return string
function Regex:regex() end

---The regex library global.
---@class RegexLib
RegexLib = {}

---Creates a new Regex from the given pattern and optional options.
---@param pattern string
---@param options? RegexOptions
---@return Regex
function RegexLib.new(pattern, options) end

---Returns true if value is a Regex object.
---@param value any
---@return boolean
function RegexLib.is_regex(value) end

---@type RegexLib
regex = {}

--------------------------------------------------------------------------------
-- ExecResponse ----------------------------------------------------------------
--------------------------------------------------------------------------------

---Result of core.exec().
---@class ExecResponse
ExecResponse = {}

---Returns the exit code of the process, or nil if terminated by a signal.
---@return integer|nil
function ExecResponse:code() end

---Returns the stdout output of the process.
---@return string
function ExecResponse:stdout() end

---Returns the stderr output of the process.
---@return string
function ExecResponse:stderr() end

--------------------------------------------------------------------------------
-- FSEvent ---------------------------------------------------------------------
--------------------------------------------------------------------------------

---Filesystem event passed to the fs.monitor() callback.
---@class FSEvent
---@field paths string[]  List of affected file paths.

--------------------------------------------------------------------------------
-- ConnectionInfo --------------------------------------------------------------
--------------------------------------------------------------------------------

---Connection metadata passed as the third argument to mud.on_connect() callbacks.
---@class ConnectionInfo
---@field host string         The hostname or IP address.
---@field port integer        The port number.
---@field tls boolean         True if the connection uses TLS.
---@field verify_cert boolean True if TLS certificate verification is enabled.
---@field name string|nil     Server name if connecting via a saved server, otherwise nil.
---@field id integer          Connection ID.

--------------------------------------------------------------------------------
-- Server ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---A saved server entry returned by servers.get() / servers.get_all().
---@class Server
---@field name string
---@field host string
---@field port integer
---@field tls boolean
---@field verify_cert boolean

--------------------------------------------------------------------------------
-- Socket ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---A TCP socket connection returned by socket.connect().
---@class Socket
Socket = {}

---Sends a string over the socket.
---@param data string
function Socket:send(data) end

---Closes the socket connection.
function Socket:close() end

--------------------------------------------------------------------------------
-- blight ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Core UI and application control.
---@class BlightLib
BlightLib = {}

---Writes one or more strings to the output buffer.
---@param ... string
function BlightLib.output(...) end

---Returns the current terminal dimensions as (width, height).
---@return integer, integer
function BlightLib.terminal_dimensions() end

---Binds a key sequence to a callback function.
---
---Key formats:
---- Function keys: `"f1"` – `"f12"`
---- Control keys: `"ctrl-a"` – `"ctrl-z"` (case-insensitive)
---- Alt keys: `"alt-X"` or `"Alt-X"` (capitalisation matters for Alt)
---- Raw escape sequences: e.g. `"\x1b[1;5A"` for Ctrl-Up
---
---Note: `ctrl-c` and `ctrl-l` cannot be rebound.
---@param key string
---@param callback fun()
function BlightLib.bind(key, callback) end

---Removes the binding for the given key sequence.
---@param key string
function BlightLib.unbind(key) end

---Sends a UI command string.
---
---Navigation: `"step_left"`, `"step_right"`, `"step_to_start"`, `"step_to_end"`,
---`"step_word_left"`, `"step_word_right"`
---
---Deletion: `"delete"`, `"delete_right"`, `"delete_word_left"`, `"delete_word_right"`,
---`"delete_to_end"`, `"delete_from_start"`
---
---Scrolling: `"scroll_up"`, `"scroll_down"`, `"scroll_top"`, `"scroll_bottom"`
---
---Other: `"complete"` (tab-completion)
---@param cmd string
function BlightLib.ui(cmd) end

---Writes a debug-level log message (visible when RUST_LOG=debug).
---@param ... string
function BlightLib.debug(...) end

---Returns true when running in core (plugin) mode.
---@return boolean
function BlightLib.is_core_mode() end

---Returns true when running in reader mode.
---@return boolean
function BlightLib.is_reader_mode() end

---Gets or sets the status bar height (0–5 lines). Returns the current height.
---@param height? integer  Clamped to 0–5.
---@return integer
function BlightLib.status_height(height) end

---Sets the content of a status bar line.
---Index is 0-based; out-of-range values default to the last/first line.
---@param index integer  0-based line index.
---@param line string
function BlightLib.status_line(index, line) end

---Gets or sets whether tag rendering is enabled. When enabled, each output
---line is prefixed with the line's tag symbol and color (or two spaces if no
---color is set). Defaults to false. Returns the current value.
---@param show? boolean
---@return boolean
function BlightLib.show_tags(show) end

---Returns the application name and version as two separate values.
---@return string, string
function BlightLib.version() end

---Returns the user config directory path.
---@return string
function BlightLib.config_dir() end

---Returns the user data directory path.
---@return string
function BlightLib.data_dir() end

---Registers a callback invoked when the application is about to quit.
---@param callback fun()
function BlightLib.on_quit(callback) end

---Registers a tab-completion callback.
---The callback receives the current prompt text and should return an array of
---completion strings (with the full prefix included), or nil.
---An optional second return value `lock: boolean` prevents subsequent
---completions from running (default false).
---@param callback fun(input: string): string[]|nil, boolean|nil
function BlightLib.on_complete(callback) end

---Registers a callback invoked when the terminal dimensions change.
---@param callback fun(width: integer, height: integer)
function BlightLib.on_dimensions_change(callback) end

---Quits the application.
function BlightLib.quit() end

---Opens a help page by name. Set lock_scroll to true to pin to top.
---@param name string
---@param lock_scroll boolean
function BlightLib.show_help(name, lock_scroll) end

---Searches backwards in the output buffer using a Regex.
---@param re Regex
function BlightLib.find_backward(re) end

---Searches forwards in the output buffer using a Regex.
---@param re Regex
function BlightLib.find_forward(re) end

---@type BlightLib
blight = {}

--------------------------------------------------------------------------------
-- mud -------------------------------------------------------------------------
--------------------------------------------------------------------------------

---Options table for mud.send().
---@class MudSendOptions
---@field gag? boolean      Suppress the echoed line in the output buffer.
---@field skip_log? boolean Do not write this line to the log.

---MUD connection and I/O.
---@class MudLib
MudLib = {}

---Registers a callback that receives every line of MUD output.
---The line object must be returned at the end of the callback.
---@param callback fun(line: Line): Line
function MudLib.add_output_listener(callback) end

---Registers a callback that receives every line of user input.
---The line object must be returned at the end of the callback.
---@param callback fun(line: Line): Line
function MudLib.add_input_listener(callback) end

---Injects a line into the MUD output buffer (shown locally, not sent to server).
---@param msg string
function MudLib.output(msg) end

---Connects to a MUD server.
---@param host string
---@param port integer
---@param tls? boolean         Use TLS (default: false).
---@param verify_cert? boolean Verify the TLS certificate (default: true when tls=true).
---@param name? string         Server name for identification in callbacks.
function MudLib.connect(host, port, tls, verify_cert, name) end

---Disconnects from the current MUD server.
function MudLib.disconnect() end

---Reconnects to the last MUD server.
function MudLib.reconnect() end

---Sends a text command to the MUD server.
---@param msg string
---@param options? MudSendOptions
function MudLib.send(msg, options) end

---Sends raw bytes to the MUD server.
---@param bytes integer[]
function MudLib.send_bytes(bytes) end

---Sends a line as if typed by the user (triggers alias processing).
---@param line string
function MudLib.input(line) end

---Registers a callback invoked when a connection is established.
---The callback receives the host string, port number, and a ConnectionInfo object.
---@param callback fun(host: string, port: integer, info: ConnectionInfo)
function MudLib.on_connect(callback) end

---Registers a callback invoked when the connection is closed.
---@param callback fun()
function MudLib.on_disconnect(callback) end

---Returns true if currently connected to a MUD server.
---@return boolean
function MudLib.is_connected() end

---Tags the current connection with a string label (shown in the top bar).
---@param tag string
function MudLib.add_tag(tag) end

---Removes a tag from the current connection.
---@param tag string
function MudLib.remove_tag(tag) end

---Clears all tags from the current connection.
function MudLib.clear_tags() end

---@type MudLib
mud = {}

--------------------------------------------------------------------------------
-- timer -----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Repeating and one-shot timers.
---@class TimerLib
TimerLib = {}

---Adds a timer that fires every `duration` seconds for `count` repetitions.
---Pass 0 for count to repeat indefinitely. Returns the timer ID.
---@param duration number   Interval in seconds.
---@param count integer     Number of firings (0 = infinite).
---@param callback fun()
---@return integer
function TimerLib.add(duration, count, callback) end

---Returns the IDs of all active user-mode timers.
---@return integer[]
function TimerLib.get_ids() end

---Removes all user-mode timers.
function TimerLib.clear() end

---Removes the timer with the given ID.
---@param id integer
function TimerLib.remove(id) end

---Registers a callback invoked on every timer tick (~100 ms).
---@param callback fun(millis: integer)
function TimerLib.on_tick(callback) end

---@type TimerLib
timer = {}

--------------------------------------------------------------------------------
-- core ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---Low-level telnet protocol and system access.
---@class CoreLib
CoreLib = {}

---Enables a telnet protocol option by its numeric ID.
---@param proto integer
function CoreLib.enable_protocol(proto) end

---Disables a telnet protocol option by its numeric ID.
---@param proto integer
function CoreLib.disable_protocol(proto) end

---Registers a callback invoked when a protocol is enabled.
---@param callback fun(proto: integer)
function CoreLib.on_protocol_enabled(callback) end

---Registers a callback invoked when a protocol is disabled.
---@param callback fun(proto: integer)
function CoreLib.on_protocol_disabled(callback) end

---Registers a callback that receives telnet sub-negotiation data.
---@param callback fun(proto: integer, data: integer[])
function CoreLib.subneg_recv(callback) end

---Sends a telnet sub-negotiation packet.
---@param proto integer
---@param bytes integer[]
function CoreLib.subneg_send(proto, bytes) end

---Executes a system command.
---Pass a string to run it via the shell (`sh -c`),
---or a string array `{executable, arg1, ...}` for direct execution.
---@param cmd string|string[]
---@return ExecResponse
function CoreLib.exec(cmd) end

---Returns the current time as a Unix timestamp in milliseconds.
---@return integer
function CoreLib.time() end

---Returns the command-line arguments passed to blightmud as a table.
---@return string[]
function CoreLib.command_line() end

---@type CoreLib
core = {}

--------------------------------------------------------------------------------
-- script ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Script loading and lifecycle.
---@class ScriptLib
ScriptLib = {}

---Loads a Lua script file at the given path.
---@param path string
function ScriptLib.load(path) end

---Resets (reloads) the Lua scripting environment.
function ScriptLib.reset() end

---Registers a callback invoked when the script environment is reset.
---@param callback fun()
function ScriptLib.on_reset(callback) end

---@type ScriptLib
script = {}

--------------------------------------------------------------------------------
-- prompt ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Input prompt access and control.
---@class PromptLib
PromptLib = {}

---Sets the current input prompt text.
---@param text string
function PromptLib.set(text) end

---Returns the current input prompt text.
---@return string
function PromptLib.get() end

---Returns the cursor position in the prompt (1-based).
---@return integer
function PromptLib.get_cursor_pos() end

---Sets the cursor position in the prompt (1-based).
---@param pos integer
function PromptLib.set_cursor_pos(pos) end

---Registers a callback invoked on every prompt input change.
---@param callback fun()
function PromptLib.add_prompt_listener(callback) end

---@type PromptLib
prompt = {}

--------------------------------------------------------------------------------
-- prompt_mask -----------------------------------------------------------------
--------------------------------------------------------------------------------

---Prompt decoration/masking (e.g. for password display).
---@class PromptMaskLib
PromptMaskLib = {}

---Overlays `mask` onto the prompt at the specified character positions (1-based).
---`data` must match the current prompt content exactly; returns false otherwise.
---The mask table maps 1-based character positions to replacement strings.
---@param data string                 The current prompt text (must match exactly).
---@param mask table<integer, string> Map of 1-based positions to overlay strings.
---@return boolean                    True if the mask was applied successfully.
function PromptMaskLib.set(data, mask) end

---Clears the current prompt mask.
function PromptMaskLib.clear() end

---Returns the current prompt mask table (keys are 1-based character positions).
---@return table<integer, string>
function PromptMaskLib.get() end

---@type PromptMaskLib
prompt_mask = {}

--------------------------------------------------------------------------------
-- settings --------------------------------------------------------------------
--------------------------------------------------------------------------------

---Persistent boolean settings.
---@class SettingsLib
SettingsLib = {}

---Returns a table of all settings and their current values.
---
---Known settings: `logging_enabled`, `mouse_enabled`, `save_history`,
---`command_search`, `smart_history`, `confirm_quit`, `scroll_split`,
---`scroll_lock`, `tts_enabled`, `reader_mode`, `hide_topbar`,
---`echo_input`, `last_command`.
---@return table<string, boolean>
function SettingsLib.list() end

---Returns the value of a named setting.
---@param key string
---@return boolean
function SettingsLib.get(key) end

---Sets a named setting to the given boolean value.
---@param key string
---@param value boolean
function SettingsLib.set(key, value) end

---@type SettingsLib
settings = {}

--------------------------------------------------------------------------------
-- store -----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Key-value storage (session and persistent disk).
---@class StoreLib
StoreLib = {}

---Writes a value to the in-memory session store.
---@param key string
---@param value string
function StoreLib.session_write(key, value) end

---Reads a value from the in-memory session store, or nil if not found.
---@param key string
---@return string|nil
function StoreLib.session_read(key) end

---Writes a value to the persistent on-disk store.
---@param key string
---@param value string
function StoreLib.disk_write(key, value) end

---Reads a value from the persistent on-disk store, or nil if not found.
---@param key string
---@return string|nil
function StoreLib.disk_read(key) end

---@type StoreLib
store = {}

--------------------------------------------------------------------------------
-- servers ---------------------------------------------------------------------
--------------------------------------------------------------------------------

---Saved server management.
---@class ServersLib
ServersLib = {}

---Adds a new saved server entry.
---@param name string
---@param host string
---@param port integer
---@param tls boolean
---@param verify_cert? boolean  Defaults to false.
function ServersLib.add(name, host, port, tls, verify_cert) end

---Removes a saved server entry by name.
---@param name string
function ServersLib.remove(name) end

---Returns the saved server with the given name.
---@param name string
---@return Server
function ServersLib.get(name) end

---Returns all saved servers.
---@return Server[]
function ServersLib.get_all() end

---@type ServersLib
servers = {}

--------------------------------------------------------------------------------
-- log -------------------------------------------------------------------------
--------------------------------------------------------------------------------

---Session logging.
---@class LogLib
LogLib = {}

---Starts logging the session to a file with the given name.
---@param name string
function LogLib.start(name) end

---Stops the current logging session.
function LogLib.stop() end

---@type LogLib
log = {}

--------------------------------------------------------------------------------
-- tts -------------------------------------------------------------------------
--------------------------------------------------------------------------------

---Text-to-speech (only functional when compiled with the `tts` feature).
---@class TtsLib
TtsLib = {}

---Returns true if TTS support was compiled in.
---@return boolean
function TtsLib.is_available() end

---Returns true if TTS is currently enabled.
---@return boolean
function TtsLib.is_enabled() end

---Enables or disables TTS.
---@param enabled boolean
function TtsLib.enable(enabled) end

---Speaks the given message. Set interrupt to true to cut off current speech.
---@param msg string
---@param interrupt? boolean
function TtsLib.speak(msg, interrupt) end

---Speaks a message directly: interrupts current speech, skips the normal queue,
---and the line is not stored in TTS history.
---@param msg string
function TtsLib.speak_direct(msg) end

---Stops the current TTS speech.
function TtsLib.stop() end

---Sets the absolute TTS speech rate.
---@param rate number
function TtsLib.set_rate(rate) end

---Adjusts the TTS speech rate by a relative delta.
---@param rate number
function TtsLib.change_rate(rate) end

---Enables or disables echoing of keypresses via TTS.
---@param enabled boolean
function TtsLib.echo_keypresses(enabled) end

---Steps back by the given number of output lines.
---@param step integer
function TtsLib.step_back(step) end

---Steps forward by the given number of output lines.
---@param step integer
function TtsLib.step_forward(step) end

---Scans back by the given number of output lines.
---@param step integer
function TtsLib.scan_back(step) end

---Scans forward by the given number of output lines.
---@param step integer
function TtsLib.scan_forward(step) end

---Scans backward to the previous input line.
function TtsLib.scan_input_back() end

---Scans forward to the next input line.
function TtsLib.scan_input_forward() end

---Jumps to the beginning of the output buffer.
function TtsLib.step_begin() end

---Jumps to the end of the output buffer.
function TtsLib.step_end() end

---@type TtsLib
tts = {}

--------------------------------------------------------------------------------
-- plugin ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Plugin management.
---@class PluginLib
PluginLib = {}

---Downloads and installs a plugin from the given git URL.
---@param url string
---@param with_submodules boolean
function PluginLib.add(url, with_submodules) end

---Loads (activates) an installed plugin by name.
---Returns a success flag and an error message (empty string on success).
---@param name string
---@return boolean, string
function PluginLib.load(name) end

---Removes an installed plugin by name.
---Returns a success flag and an error message (empty string on success).
---@param name string
---@return boolean, string
function PluginLib.remove(name) end

---Returns the names of all installed plugins.
---@return string[]
function PluginLib.get_all() end

---Updates an installed plugin by name.
---@param name string
function PluginLib.update(name) end

---Adds a plugin to the auto-load list.
---@param name string
function PluginLib.enable(name) end

---Removes a plugin from the auto-load list.
---@param name string
function PluginLib.disable(name) end

---Returns the names of all auto-loaded (enabled) plugins.
---@return string[]
function PluginLib.enabled() end

---Returns the filesystem path for a plugin's directory, or the root plugins dir.
---@param name? string
---@return string
function PluginLib.dir(name) end

---@type PluginLib
plugin = {}

--------------------------------------------------------------------------------
-- fs --------------------------------------------------------------------------
--------------------------------------------------------------------------------

---Filesystem monitoring.
---@class FsLib
FsLib = {}

---Monitors a filesystem path and invokes callback when it changes.
---The monitor has a 5-second de-bounce.
---@param path string
---@param callback fun(event: FSEvent)
function FsLib.monitor(path, callback) end

---@type FsLib
fs = {}

--------------------------------------------------------------------------------
-- audio -----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Audio playback options.
---@class AudioOptions
---@field loop? boolean    Loop the audio (music only). Default: false.
---@field amplify? number  Volume multiplier (1.0 = original). Default: 1.0.

---Audio playback (MP3, WAV, Vorbis, FLAC supported).
---@class AudioLib
AudioLib = {}

---Plays a music file. Only one music track plays at a time.
---@param path string
---@param options? AudioOptions
function AudioLib.play_music(path, options) end

---Stops the currently playing music.
function AudioLib.stop_music() end

---Plays a sound effect.
---@param path string
---@param options? AudioOptions
function AudioLib.play_sfx(path, options) end

---Stops the currently playing sound effect.
function AudioLib.stop_sfx() end

---@type AudioLib
audio = {}

--------------------------------------------------------------------------------
-- socket ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---TCP socket library (send-only; no receive capability).
---@class SocketLib
SocketLib = {}

---Opens a TCP connection to host:port. Returns a Socket, or nil on failure.
---@param host string
---@param port integer
---@return Socket|nil
function SocketLib.connect(host, port) end

---@type SocketLib
socket = {}

--------------------------------------------------------------------------------
-- json ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---JSON encode/decode library (rxi/json.lua, MIT licensed).
---@class JsonLib
JsonLib = {}

---Encodes a Lua value as a JSON string.
---@param value any
---@return string
function JsonLib.encode(value) end

---Decodes a JSON string into a Lua value.
---@param str string
---@return any
function JsonLib.decode(str) end

---@type JsonLib
json = {}

--------------------------------------------------------------------------------
-- search ----------------------------------------------------------------------
--------------------------------------------------------------------------------

---Output buffer search.
---@class SearchLib
SearchLib = {}

---Compiles the pattern and searches backward from the current scroll position.
---@param pattern string  A regex pattern string.
function SearchLib.search(pattern) end

---Repeats the last search upward (backward).
function SearchLib.find_up() end

---Repeats the last search downward (forward).
function SearchLib.find_down() end

---Searches backward for the last input line (requires `echo_input` setting).
function SearchLib.find_last_input() end

---Searches forward for the next input line (requires `echo_input` setting).
function SearchLib.find_next_input() end

---@type SearchLib
search = {}

--------------------------------------------------------------------------------
-- history ---------------------------------------------------------------------
--------------------------------------------------------------------------------

---Command history navigation.
---@class HistoryLib
HistoryLib = {}
---Navigates to the previous command in history.
---With the `command_search` setting enabled, filters by the current prompt prefix.
function HistoryLib.previous_command() end

---Navigates to the next command in history.
function HistoryLib.next_command() end

---@type HistoryLib
history = {}

--------------------------------------------------------------------------------
-- spellcheck ------------------------------------------------------------------
--------------------------------------------------------------------------------

---Hunspell-based spell checking.
---@class SpellcheckLib
SpellcheckLib = {}
---Initializes the spell checker with Hunspell AFF and dictionary files.
---Must be called before check() or suggest().
---@param aff_path string   Path to the `.aff` affix file.
---@param dict_path string  Path to the `.dic` dictionary file.
function SpellcheckLib.init(aff_path, dict_path) end

---Returns true if the word is spelled correctly.
---Raises an error if init() has not been called.
---@param word string
---@return boolean
function SpellcheckLib.check(word) end

---Returns a list of spelling suggestions for the given word.
---Raises an error if init() has not been called.
---@param word string
---@return string[]
function SpellcheckLib.suggest(word) end

---@type SpellcheckLib
spellcheck = {}

--------------------------------------------------------------------------------
-- gmcp ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---GMCP (Generic Mud Communication Protocol, telnet option 201).
---@class GmcpLib
GmcpLib = {}
---Registers a callback invoked when GMCP negotiation completes.
---If GMCP is already ready the callback is called immediately.
---@param callback fun()
function GmcpLib.on_ready(callback) end

---Sends a raw GMCP message string (e.g. `"Core.Ping"`).
---@param msg string
function GmcpLib.send(msg) end

---Registers a callback for the given GMCP module name.
---The callback receives the raw JSON body string as its argument.
---If a cached value exists the callback is called immediately.
---@param module string   GMCP module name (e.g. `"Char.Vitals"`).
---@param callback fun(data: string)
function GmcpLib.receive(module, callback) end

---Sends `Core.Supports.Add` for the given module name.
---@param module string
function GmcpLib.register(module) end

---Sends `Core.Supports.Remove` for the given module name.
---@param module string
function GmcpLib.unregister(module) end

---Enables or disables printing all received GMCP messages to the output buffer.
---@param enabled boolean
function GmcpLib.echo(enabled) end

---@type GmcpLib
gmcp = {}

--------------------------------------------------------------------------------
-- msdp ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---MSDP (Mud Server Data Protocol, telnet option 69).
---@class MsdpLib
MsdpLib = {}
---Returns the cached value of a server variable, or nil.
---@param key string
---@return string|table|nil
function MsdpLib.get(key) end

---Sends a variable/value pair to the server.
---@param var string
---@param val string
function MsdpLib.set(var, val) end

---Registers a callback for updates to a specific variable.
---Called immediately if a cached value exists.
---@param variable string
---@param callback fun(value: any)
function MsdpLib.register(variable, callback) end

---Requests the server to report updates for the given variable(s).
---@param value string|string[]
function MsdpLib.report(value) end

---Stops the server from reporting updates for the given variable(s).
---@param value string|string[]
function MsdpLib.unreport(value) end

---Sends a LIST request for the given list name.
---@param list string
function MsdpLib.list(list) end

---Sends a SEND request for the given variable(s).
---@param var string|string[]
function MsdpLib.send(var) end

---Registers a callback invoked when MSDP negotiation completes.
---@param callback fun()
function MsdpLib.on_ready(callback) end

---@type MsdpLib
msdp = {}

--------------------------------------------------------------------------------
-- mssp ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---MSSP (Mud Server Status Protocol, telnet option 70).
---@class MsspLib
MsspLib = {}
---Returns the cached MSSP data table.
---@return table<string, string>
function MsspLib.get() end

---Prints all received MSSP values to the output buffer.
function MsspLib.print() end

---@type MsspLib
mssp = {}

--------------------------------------------------------------------------------
-- ttype -----------------------------------------------------------------------
--------------------------------------------------------------------------------

---TTYPE (Terminal Type, telnet option 24) negotiation.
---@class TtypeLib
---@field MTTS_ANSI? integer           ANSI color support (0x001).
---@field MTTS_VT100? integer          VT100 support (0x002).
---@field MTTS_UTF8? integer           UTF-8 support (0x004).
---@field MTTS_256_COLOR? integer      256-color support (0x008).
---@field MTTS_MOUSE_TRACKING? integer Mouse tracking support (0x010).
---@field MTTS_OSC_COLOR? integer      OSC color palette / true color (0x020).
---@field MTTS_SCREEN_READER? integer  Screen reader in use (0x040).
---@field MTTS_PROXY? integer          Proxy connection (0x080).
---@field MTTS_TRUE_COLOR? integer     True-color support (0x100).
---@field MTTS_MNES? integer           Mud New-Env Standard enabled (0x200).
---@field MTTS_MSLP? integer           Mud Server Link Protocol enabled (0x400).
TtypeLib = {}
---Overrides the terminal type string sent during negotiation.
---Must be called before connecting.
---@param term string  e.g. `"xterm-256color"`
function TtypeLib.set_term(term) end

---Replaces the entire MTTS bitmask value.
---Must be called before connecting.
---@param mtts integer
function TtypeLib.set_mtts(mtts) end

---ORs an MTTS flag into the current bitmask.
---Disables automatic screen-reader detection when MTTS_SCREEN_READER is set.
---@param option integer  One of the `ttype.MTTS_*` constants.
function TtypeLib.add_option(option) end

---ANDs out an MTTS flag from the current bitmask.
---Disables automatic screen-reader detection when MTTS_SCREEN_READER is cleared.
---@param option integer  One of the `ttype.MTTS_*` constants.
function TtypeLib.rem_option(option) end

---@type TtypeLib
ttype = {}

--------------------------------------------------------------------------------
-- alias -----------------------------------------------------------------------
--------------------------------------------------------------------------------

---An individual alias that matches input lines with a regex.
---@class Alias
---@field id integer
---@field regex Regex
---@field enabled boolean
Alias = {}
---Creates a new Alias. re may be a pattern string or a Regex object.
---@param re string|Regex
---@param callback fun(matches: string[], line: Line)
---@return Alias
function Alias.new(re, callback) end

---Returns true if obj is an Alias instance.
---@param obj any
---@return boolean
function Alias.is_alias(obj) end

---Enables this alias.
function Alias:enable() end

---Disables this alias.
function Alias:disable() end

---Sets the enabled state.
---@param flag boolean
function Alias:set_enabled(flag) end

---Returns true if the alias is enabled.
---@return boolean
function Alias:is_enabled() end

---A group of aliases that can be enabled/disabled together.
---@class AliasGroup
---@field id integer
---@field enabled boolean
---@field aliases table<integer, Alias>
AliasGroup = {}
---Creates a new AliasGroup with the given id.
---@param id integer
---@return AliasGroup
function AliasGroup.new(id) end

---Adds an alias to the group. re may be a pattern string, Regex, or Alias.
---@param re string|Regex|Alias
---@param callback? fun(matches: string[], line: Line)
---@return Alias
function AliasGroup:add(re, callback) end

---Returns the alias with the given id, or nil.
---@param id integer
---@return Alias|nil
function AliasGroup:get(id) end

---Returns all aliases in the group.
---@return table<integer, Alias>
function AliasGroup:get_aliases() end

---Removes the alias with the given id.
---@param id integer
function AliasGroup:remove(id) end

---Removes all aliases from the group.
function AliasGroup:clear() end

---Returns true if the group is enabled.
---@return boolean
function AliasGroup:is_enabled() end

---Sets the enabled state of the group.
---@param flag boolean
function AliasGroup:set_enabled(flag) end

---Enables the group.
function AliasGroup:enable() end

---Disables the group.
function AliasGroup:disable() end

---The alias module global.
---@class AliasModule
---@field Alias? Alias
---@field AliasGroup? AliasGroup
---@field alias_groups? AliasGroup[]
---@field system_alias_groups? AliasGroup[]
AliasModule = {}
---Adds an alias to the default group. Returns the new Alias.
---@param re string|Regex
---@param callback fun(matches: string[], line: Line)
---@return Alias
function AliasModule.add(re, callback) end

---Returns the alias with the given id from any group, or nil.
---@param id integer
---@return Alias|nil
function AliasModule.get(id) end

---Returns the alias group with the given id (default: 1).
---@param id? integer
---@return AliasGroup
function AliasModule.get_group(id) end

---Removes the alias with the given id from all groups.
---@param id integer
function AliasModule.remove(id) end

---Clears all aliases from all groups.
function AliasModule.clear() end

---Creates and returns a new AliasGroup.
---@return AliasGroup
function AliasModule.add_group() end

---@type AliasModule
alias = {}

--------------------------------------------------------------------------------
-- trigger ---------------------------------------------------------------------
--------------------------------------------------------------------------------

---Options table for Trigger.new().
---@class TriggerOptions
---@field gag? boolean     Gag (hide) the matching line.
---@field raw? boolean     Match against the raw ANSI line instead of clean text.
---@field prompt? boolean  Match against prompt lines instead of output lines.
---@field count? integer   Fire at most this many times, then auto-remove.
---@field enabled? boolean Initial enabled state (default: true).

---An individual trigger that matches MUD output lines with a regex.
---@class Trigger
---@field id integer
---@field regex Regex
---@field gag boolean
---@field raw boolean
---@field prompt boolean
---@field count integer|nil
---@field enabled boolean
Trigger = {}
---Creates a new Trigger. re may be a pattern string or a Regex object.
---@param re string|Regex
---@param options TriggerOptions
---@param callback fun(matches: string[], line: Line)
---@return Trigger
function Trigger.new(re, options, callback) end

---Returns true if obj is a Trigger instance.
---@param obj any
---@return boolean
function Trigger.is_trigger(obj) end

---Enables this trigger.
function Trigger:enable() end

---Disables this trigger.
function Trigger:disable() end

---Sets the enabled state.
---@param flag boolean
function Trigger:set_enabled(flag) end

---Returns true if the trigger is enabled.
---@return boolean
function Trigger:is_enabled() end

---A group of triggers that can be enabled/disabled together.
---@class TriggerGroup
---@field id integer
---@field enabled boolean
---@field triggers table<integer, Trigger>
TriggerGroup = {}
---Creates a new TriggerGroup with the given id.
---@param id integer
---@return TriggerGroup
function TriggerGroup.new(id) end

---Adds a trigger to the group. re may be a pattern string, Regex, or Trigger.
---@param re string|Regex|Trigger
---@param options? TriggerOptions
---@param callback? fun(matches: string[], line: Line)
---@return Trigger
function TriggerGroup:add(re, options, callback) end

---Returns the trigger with the given id, or nil.
---@param id integer
---@return Trigger|nil
function TriggerGroup:get(id) end

---Returns all triggers in the group.
---@return table<integer, Trigger>
function TriggerGroup:get_triggers() end

---Removes the trigger with the given id.
---@param id integer
function TriggerGroup:remove(id) end

---Removes all triggers from the group.
function TriggerGroup:clear() end

---Returns true if the group is enabled.
---@return boolean
function TriggerGroup:is_enabled() end

---Sets the enabled state of the group.
---@param flag boolean
function TriggerGroup:set_enabled(flag) end

---Enables the group.
function TriggerGroup:enable() end

---Disables the group.
function TriggerGroup:disable() end

---The trigger module global.
---@class TriggerModule
---@field Trigger? Trigger
---@field TriggerGroup? TriggerGroup
---@field trigger_groups? TriggerGroup[]
---@field system_trigger_groups? TriggerGroup[]
TriggerModule = {}
---Adds a trigger to the default group. Returns the new Trigger.
---@param re string|Regex
---@param options TriggerOptions
---@param callback fun(matches: string[], line: Line)
---@return Trigger
function TriggerModule.add(re, options, callback) end

---Returns the trigger with the given id from any group, or nil.
---@param id integer
---@return Trigger|nil
function TriggerModule.get(id) end

---Returns the trigger group with the given id (default: 1).
---@param id? integer
---@return TriggerGroup
function TriggerModule.get_group(id) end

---Removes the trigger with the given id from all groups.
---@param id integer
function TriggerModule.remove(id) end

---Clears all triggers from all groups.
function TriggerModule.clear() end

---Creates and returns a new TriggerGroup.
---@return TriggerGroup
function TriggerModule.add_group() end

---@type TriggerModule
trigger = {}

--------------------------------------------------------------------------------
-- Task ------------------------------------------------------------------------
--------------------------------------------------------------------------------

---A coroutine-based asynchronous task.
---@class Task
---@field dead boolean     True when the task has finished or been killed.
---@field started boolean  True after the task has been started.
---@field success boolean  True if the coroutine returned without error.
---@field value any        Return values from the coroutine (after it finishes).
---@field error any        Error value if the coroutine errored (after it finishes).
Task = {}
---Creates and immediately starts a new Task running callable with args.
---Tasks are automatically killed if they do not yield within 2 seconds.
---@param callable fun(...): any
---@param ... any
---@return Task
function Task.spawn(callable, ...) end

---Creates a Task that starts after `time` seconds.
---@param time number
---@param callable fun(...): any
---@param ... any
---@return Task
function Task.spawn_later(time, callable, ...) end

---Returns the currently executing Task, or nil if called from outside a task.
---@return Task|nil
function Task.get_current() end

---Suspends the task for at least `time` seconds.
---@param time number
function Task:sleep(time) end

---Suspends the task until the next tick when no other tasks are running.
function Task:idle() end

---Sends a value to the task; the task receives it as the return of coroutine.yield().
---@param value any
function Task:send(value) end

---Kills (cancels) the task.
function Task:kill() end

---The task module global.
---@class TaskModule
---@field Task? Task
---@field spawn? fun(callable: fun(...): any, ...: any): Task
---@field spawn_later? fun(time: number, callable: fun(...): any, ...: any): Task
---@field yield? fun(): any
TaskModule = {}
---Sleeps the current task for `time` seconds (must be called from within a task).
---@param time number
function TaskModule.sleep(time) end

---Idles the current task until the next available tick.
function TaskModule.idle() end

---Returns the currently executing task, or nil.
---@return Task|nil
function TaskModule.get_current() end

---Returns all currently active tasks.
---@return Task[]
function TaskModule.get_tasks() end

---Returns true if obj is a Task instance.
---@param obj any
---@return boolean
function TaskModule.is_task(obj) end

---@type TaskModule
task = {}

--------------------------------------------------------------------------------
-- Global helpers --------------------------------------------------------------
--------------------------------------------------------------------------------

---Converts a byte array to a string.
---@param bytes integer[]
---@return string
function bytes_to_string(bytes) end

---Formats a string with color tag substitution and string.format patterns.
---
---Color tags use `<NAME>` syntax where NAME is a color name in uppercase:
---`<RED>`, `<GREEN>`, `<BLUE>`, `<YELLOW>`, `<MAGENTA>`, `<CYAN>`, `<WHITE>`,
---`<BLACK>`, `<BOLD>`, `<RESET>`, and bright variants prefixed with `B` (e.g. `<BRED>`).
---Combined foreground/background: `<fg:bg>` (e.g. `<RED:BLACK>`).
---
---```lua
---blight.output(cformat("<GREEN>%s<RESET>", player_name))
---```
---@param msg string
---@param ... any
---@return string
function cformat(msg, ...) end

---@type string
C_RESET = "\x1b[0m"
---@type string
C_BOLD = "\x1b[1m"
---@type string
C_BLACK = "\x1b[30m"
---@type string
C_RED = "\x1b[31m"
---@type string
C_GREEN = "\x1b[32m"
---@type string
C_YELLOW = "\x1b[33m"
---@type string
C_BLUE = "\x1b[34m"
---@type string
C_MAGENTA = "\x1b[35m"
---@type string
C_CYAN = "\x1b[36m"
---@type string
C_WHITE = "\x1b[37m"
---@type string
C_BBLACK = "\x1b[90m"
---@type string
C_BRED = "\x1b[91m"
---@type string
C_BGREEN = "\x1b[92m"
---@type string
C_BYELLOW = "\x1b[93m"
---@type string
C_BBLUE = "\x1b[94m"
---@type string
C_BMAGENTA = "\x1b[95m"
---@type string
C_BCYAN = "\x1b[96m"
---@type string
C_BWHITE = "\x1b[97m"
---@type string
BG_BLACK = "\x1b[40m"
---@type string
BG_RED = "\x1b[41m"
---@type string
BG_GREEN = "\x1b[42m"
---@type string
BG_YELLOW = "\x1b[43m"
---@type string
BG_BLUE = "\x1b[44m"
---@type string
BG_MAGENTA = "\x1b[45m"
---@type string
BG_CYAN = "\x1b[46m"
---@type string
BG_WHITE = "\x1b[47m"
---@type string
BG_BBLACK = "\x1b[100m"
---@type string
BG_BRED = "\x1b[101m"
---@type string
BG_BGREEN = "\x1b[102m"
---@type string
BG_BYELLOW = "\x1b[103m"
---@type string
BG_BBLUE = "\x1b[104m"
---@type string
BG_BMAGENTA = "\x1b[105m"
---@type string
BG_BCYAN = "\x1b[106m"
---@type string
BG_BWHITE = "\x1b[107m"
