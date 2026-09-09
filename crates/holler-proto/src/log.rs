//! Debug logging for both roles (story #144).
//!
//! A small, dependency-free logging core that the `holler` binary installs
//! once at startup and the hub/body code uses to emit structured events. It
//! has **no tokio / network / async dependency** (only `serde`/`serde_json`
//! and `time` for the emission timestamp) so it lives in `holler-proto`,
//! which both roles already link.
//!
//! Two orthogonal dials:
//!
//! - **`DebugLevel`** — `none` / `quiet` / `noisy` — controls whether
//!   *debug* events are emitted. `quiet` emits debug lines that carry only
//!   the *shape* of a frame (direction, method, short ids); `noisy` adds the
//!   full **redacted** JSON-RPC frame.
//! - **`LogFormat`** — `text` / `json` — the *rendering* of every line. The
//!   severity of an event (`debug`/`info`/`warn`) is **independent** of the
//!   debug level: `info`/`warn` are always emitted (operational facts like
//!   *listening, connected, dropped, lockout, pepper_generated*); only
//!   `debug` events are gated by the level.
//!
//! Both dials are resolved the same way: the **CLI flag wins over the
//! environment** (`HOLLER_DEBUG`, `HOLLER_LOG_FORMAT`), defaults
//! `none`/`text`, and an **unrecognised value fails the process** (exit 3,
//! fail-closed policy refusal — see ADR 0003 exit codes) rather than silently
//! falling through.
//!
//! All log output goes to **stderr**; **stdout is reserved for command
//! output** (`--json`, …). The startup banner (`logging_started`) is emitted
//! before any blocking I/O so the resolved settings are visible even when the
//! later work hangs.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use serde_json::{Map, Value};

// --- dials -----------------------------------------------------------------

/// How much *debug* detail to emit. `info`/`warn` are unaffected (always on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugLevel {
    /// No debug output (the default).
    None,
    /// Debug lines carry the frame *shape* only (direction, method, ids).
    Quiet,
    /// `quiet` plus the full redacted JSON-RPC frame on the line.
    Noisy,
}

impl DebugLevel {
    /// The default level: silent about debug.
    pub const DEFAULT: DebugLevel = DebugLevel::None;

    /// Parse a raw flag/env value. `None` for a value that is not
    /// `none`/`quiet`/`noisy` (the caller turns that into a fail-closed exit).
    pub fn parse(s: &str) -> Option<DebugLevel> {
        match s {
            "none" => Some(DebugLevel::None),
            "quiet" => Some(DebugLevel::Quiet),
            "noisy" => Some(DebugLevel::Noisy),
            _ => None,
        }
    }

    /// The value's spelling, for the banner / JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            DebugLevel::None => "none",
            DebugLevel::Quiet => "quiet",
            DebugLevel::Noisy => "noisy",
        }
    }
}

/// How log lines are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-oriented single lines (fixed-width columns).
    Text,
    /// One JSON object per line (machine-parseable).
    Json,
}

impl LogFormat {
    /// The default format: `text`.
    pub const DEFAULT: LogFormat = LogFormat::Text;

    /// Parse a raw flag/env value. `None` for a value that is not
    /// `text`/`json` (the caller turns that into a fail-closed exit).
    pub fn parse(s: &str) -> Option<LogFormat> {
        match s {
            "text" => Some(LogFormat::Text),
            "json" => Some(LogFormat::Json),
            _ => None,
        }
    }

    /// The value's spelling, for the banner / JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogFormat::Text => "text",
            LogFormat::Json => "json",
        }
    }
}

// --- severity ----------------------------------------------------------------

/// The severity of a single event. Independent of [`DebugLevel`]: `Debug`
/// events are gated by the level, `Info`/`Warn` are always emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Debug,
    Info,
    Warn,
}

impl Severity {
    fn as_str(&self) -> &'static str {
        match self {
            Severity::Debug => "DEBUG",
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
        }
    }
}

// --- direction & component ---------------------------------------------------

/// Which side of the circuit a wire event concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// body → hub.
    Out,
    /// hub → body.
    In,
    /// Local (not a frame crossing the wire).
    Local,
}

impl Direction {
    fn as_str(&self) -> &'static str {
        match self {
            Direction::Out => "->",
            Direction::In => "<-",
            Direction::Local => "--",
        }
    }
}

/// The component emitting an event (column value / JSON `component`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    /// JSON-RPC frames on the WebSocket (the codec / dispatcher).
    Wire,
    /// The hub's registry (sessions, roster, peers).
    Registry,
    /// The roster.
    Roster,
    /// The talk log (append-only per-session record).
    Talklog,
    /// Token / pepper handling.
    Token,
    /// The control socket / local IPC.
    Control,
    /// The ACP agent bridge.
    Acp,
    /// HTTP attach surface.
    HttpAttach,
    /// A body session / turn.
    Session,
    /// The CLI itself (parsing, startup, fail-closed refusals).
    Cli,
}

impl Component {
    /// The component's column spelling (used both as the JSON `component`
    /// value and, left-padded to 12 chars, as the text column).
    pub fn as_str(&self) -> &'static str {
        match self {
            Component::Wire => "wire",
            Component::Registry => "registry",
            Component::Roster => "roster",
            Component::Talklog => "talklog",
            Component::Token => "token",
            Component::Control => "control",
            Component::Acp => "acp",
            Component::HttpAttach => "http_attach",
            Component::Session => "session",
            Component::Cli => "cli",
        }
    }

    /// The text-format column: the name left-padded to 12 chars (the
    /// reference layout, e.g. `wire` → `"      wire"`).
    pub fn column(&self) -> String {
        let s = self.as_str();
        if s.len() >= 12 {
            s.to_owned()
        } else {
            let pad = 12 - s.len();
            let padding = " ".repeat(pad);
            format!("{padding}{s}")
        }
    }
}

// --- event -------------------------------------------------------------------

/// One structured log event. The `frame` field is the **already-redacted**
/// wire frame string; it is only rendered when the level is `noisy`.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub component: Component,
    pub severity: Severity,
    pub direction: Direction,
    /// The method name (`session/prompt`), or a local event name (`lockout`).
    pub method: &'static str,
    /// Correlation id (short form rendered), if any.
    pub id: Option<&'static str>,
    /// Peer client id, if any.
    pub peer: Option<&'static str>,
    /// Free-form structured fields, in insertion order.
    pub fields: Vec<(&'static str, String)>,
    /// The redacted wire frame; `Some` only at `noisy`.
    pub frame: Option<String>,
}

impl Event {
    /// Render the event to a single line in the given format. The redacted
    /// frame (if present) is appended at `noisy`.
    pub fn render(&self, config: &Config) -> String {
        if self.severity == Severity::Debug && config.debug == DebugLevel::None {
            return String::new();
        }
        match config.format {
            LogFormat::Json => render_json(self),
            LogFormat::Text => render_text(self),
        }
    }
}

/// The configured, resolved logging settings for the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub debug: DebugLevel,
    pub format: LogFormat,
}

static CONFIG: OnceLock<Config> = OnceLock::new();
/// `0` = banner not yet emitted; set to `1` in `emit_banner`. Atomic so a
/// re-entrant call (or a second install) cannot double-print the banner.
static BANNER_EMITTED: AtomicU8 = AtomicU8::new(0);

/// Resolve the settings, **fail-closed**: the CLI flag beats the environment,
/// and an invalid value is an `Err` (the caller turns it into exit 3).
///
/// A `flag_*` of `None` — or an **empty** `Some("")` — means "the flag was not
/// effectively given", so the env (then the `none`/`text` default) decides.
/// The empty-string arm matters because the CLI captures these dials as
/// `default_value = ""` (to keep a typed-environment-free `Option` while still
/// distinguishing "given" from "absent"); an absent flag therefore arrives as
/// `Some("")` and must not be mistaken for the invalid value `""`.
pub fn resolve(
    flag_debug: Option<&str>,
    flag_format: Option<&str>,
    env_debug: Option<&str>,
    env_format: Option<&str>,
) -> Result<Config, String> {
    let debug_raw = flag_debug
        .filter(|s| !s.is_empty())
        .or(env_debug)
        .unwrap_or("none");
    let debug = match DebugLevel::parse(debug_raw) {
        Some(d) => d,
        None => {
            return Err(format!(
                "invalid --debug {debug_raw:?}: expected one of none|quiet|noisy"
            ))
        }
    };
    let format_raw = flag_format
        .filter(|s| !s.is_empty())
        .or(env_format)
        .unwrap_or("text");
    let format = match LogFormat::parse(format_raw) {
        Some(f) => f,
        None => {
            return Err(format!(
                "invalid --log-format {format_raw:?}: expected one of text|json"
            ))
        }
    };
    Ok(Config { debug, format })
}

/// Install (or replace, for tests) the process-wide config. Returns it.
pub fn init(config: Config) -> Config {
    let _ = CONFIG.set(config);
    config
}

/// The installed config, or the defaults (`none`/`text`) if none installed.
pub fn get() -> Config {
    *CONFIG.get_or_init(|| Config {
        debug: DebugLevel::DEFAULT,
        format: LogFormat::DEFAULT,
    })
}

/// Emit the startup banner to stderr, exactly once per process. The line
/// carries the resolved `level`/`format` so the settings are visible before
/// any blocking I/O. Returns the banner line (also written to stderr).
pub fn emit_banner() -> String {
    if BANNER_EMITTED
        .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return String::new();
    }
    let config = get();
    // Always text, regardless of `format`: the banner is the one line a human
    // reads to confirm the settings took effect, and it precedes blocking I/O.
    let line = format!(
        "{} INFO {} -- logging_started level={} format={}",
        timestamp(),
        Component::Cli.column(),
        config.debug.as_str(),
        config.format.as_str()
    );
    eprintln!("{line}");
    line
}

/// The RFC3339 UTC emission timestamp, fixed-width to the microsecond, with a
/// trailing `Z` (e.g. `2026-09-06T20:59:54.712345Z`). This is the wall-clock
/// moment the event is *logged*, independent of any frame's own timestamp.
pub fn timestamp() -> String {
    use time::OffsetDateTime;
    // `time`'s built-in Rfc3339 formatter is second-precision; the spec wants
    // microsecond precision, so the parts are composed explicitly (all UTC).
    let now = OffsetDateTime::now_utc();
    let year = now.year() as u32;
    let month = now.month() as u32;
    let day = now.day() as u32;
    let hour = now.hour();
    let minute = now.minute();
    let second = now.second();
    let micros = (now.time().nanosecond() / 1000) % 1000;
    format!(
        "{year:02}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{micros:03}Z"
    )
}

// --- redaction ---------------------------------------------------------------

/// A redacted value, for any matched secret.
pub const REDACTED: &str = "[redacted]";

/// Field-name substrings that mark a value as a secret (matched
/// case-insensitively against the key).
const SECRET_KEY_SUBSTRINGS: &[&str] = &["secret", "credential", "ticket", "authorization", "pepper"];

/// String-value prefixes that mark a value as a token, regardless of key.
const SECRET_VALUE_PREFIXES: &[&str] = &["hlr_join_", "hlr_live_"];

/// Whether a JSON object key names a secret-bearing field.
pub fn key_is_secret(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SECRET_KEY_SUBSTRINGS.iter().any(|s| lower.contains(s))
}

/// Whether a string value is a bearer token (by prefix).
pub fn value_is_secret(value: &str) -> bool {
    SECRET_VALUE_PREFIXES.iter().any(|p| value.starts_with(p))
}

/// Apply redaction to a JSON value, in place (mutating a clone). Every object
/// whose **key** matches a secret substring, and every **string value** that
/// begins with a token prefix, is replaced with `"[redacted]"` — recursively,
/// at every depth. Non-secret structure (ids, hostnames, session names) is
/// left intact.
pub fn redact(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                let red = key_is_secret(k) || matches!(v, Value::String(s) if value_is_secret(s));
                out.insert(k.clone(), if red {
                    Value::String(REDACTED.to_owned())
                } else {
                    redact(v)
                });
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact).collect()),
        Value::String(s) => {
            if value_is_secret(s) {
                Value::String(REDACTED.to_owned())
            } else {
                value.clone()
            }
        }
        other => other.clone(),
    }
}

/// Redact a raw wire-frame string: parse, redact, re-serialise. A frame that
/// does not parse as JSON is returned verbatim (it will not be a secret
/// shape). This is the entry point the wire code calls before rendering.
pub fn redact_frame(frame: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(frame) else {
        return frame.to_owned();
    };
    serde_json::to_string(&redact(&v)).unwrap_or_else(|_| frame.to_owned())
}

// --- rendering ----------------------------------------------------------------

/// A 12-char id, `0`-padded on the right when short (the reference layout
/// shows 12-char ids such as `c14fb1a960b3`).
fn short_id(id: &str) -> String {
    if id.len() > 12 {
        id.get(..12).unwrap_or(id).to_owned()
    } else {
        id.to_owned()
    }
}

/// Escape control characters (`\n`, `\r`, tabs, and any other non-printable
/// byte) in a field value before it is concatenated into a text-format log
/// line. Values reaching here can be attacker-supplied (e.g. a body's
/// self-declared `hostname`), so a raw newline/carriage-return must never
/// reach `eprintln!` unescaped — that would let the value forge additional,
/// fake log lines (CWE-117 log injection). Printable characters (including
/// non-ASCII text) are passed through untouched; only `char::is_control`
/// characters are rewritten via `char::escape_default` (e.g. `\n` -> the two
/// characters `\` `n`, a raw NUL -> `\u{0}`).
fn escape_field_value(v: &str) -> String {
    if v.chars().any(char::is_control) {
        v.chars()
            .flat_map(|c| {
                if c.is_control() {
                    c.escape_default().collect::<Vec<char>>()
                } else {
                    vec![c]
                }
            })
            .collect()
    } else {
        v.to_owned()
    }
}

fn render_text(ev: &Event) -> String {
    let ts = timestamp();
    let level = ev.severity.as_str();
    let component = ev.component.column();
    let dir = ev.direction.as_str();
    let mut s = format!("{ts} {level} {component} {dir} {}", ev.method);
    if let Some(id) = ev.id {
        s.push_str(&format!(" id={}", short_id(id)));
    }
    if let Some(peer) = ev.peer {
        s.push_str(&format!(" peer={peer}"));
    }
    for (k, v) in &ev.fields {
        s.push_str(&format!(" {k}={}", escape_field_value(v)));
    }
    if let Some(frame) = &ev.frame {
        s.push_str(&format!(" {frame}"));
    }
    s
}

fn render_json(ev: &Event) -> String {
    let mut m = Map::new();
    m.insert("ts".into(), Value::String(timestamp()));
    m.insert("level".into(), Value::String(ev.severity.as_str().to_owned()));
    m.insert("component".into(), Value::String(ev.component.as_str().to_owned()));
    m.insert("dir".into(), Value::String(ev.direction.as_str().to_owned()));
    m.insert("type".into(), Value::String(ev.method.to_owned()));
    if let Some(id) = ev.id {
        m.insert("id".into(), Value::String(short_id(id)));
    }
    if let Some(peer) = ev.peer {
        m.insert("peer".into(), Value::String(peer.to_owned()));
    }
    for (k, v) in &ev.fields {
        m.insert(k.to_string(), Value::String(v.clone()));
    }
    if let Some(frame) = &ev.frame {
        // Nest the (already redacted) frame as an object under `frame`.
        let parsed: Value = serde_json::from_str(frame).unwrap_or(Value::String(frame.clone()));
        m.insert("frame".into(), parsed);
    }
    serde_json::to_string(&Value::Object(m)).unwrap_or_default()
}

// --- emission helper (used by hub/body; keeps the stderr contract in one place)

/// Render and write an event to **stderr** (never stdout). No-op for a
/// `debug` event when the level is `none`.
pub fn emit(ev: &Event) {
    let line = ev.render(&get());
    if line.is_empty() {
        return;
    }
    eprintln!("{line}");
}
