//! The body's session config file (issue #187): named sessions, each a
//! harness + transport (`spawn` a fresh process or `attach` to one already
//! running), validated fail-closed at parse time.
//!
//! Grammar (the issue's own spec):
//!
//! ```text
//! [[session]]
//! name = "alpha"                       # SessionName grammar (ADR 0005); unique in file
//! harness = "opencode"                 # any holler_proto::HARNESS_IDS id; unknown ids are
//!                                       # legal on the wire but `support` says ok:false
//! mode = "spawn"                       # "spawn" (default) | "attach"
//! command = ["opencode", "acp"]        # spawn only; argv, no shell
//! cwd = "/path"                        # optional, spawn only
//! env = { KEY = "value" }              # optional, spawn only
//! interrupt = "acp"                    # optional: "acp" (default) | "http"; http needs endpoint
//! endpoint = "http://127.0.0.1:4096"   # attach: required; spawn: optional (HTTP interrupt fallback)
//! session_id = "ses_..."               # attach: required
//! ```
//!
//! A `command` is any argv, not just a single token — `command[0]` is all
//! this module (and [`crate::query`]'s PATH-resolution probe) ever looks at,
//! so a multi-word spawn command works with zero special-casing. For
//! example, Claude Code has no native `claude acp` subcommand; a session
//! wired to it goes through the community `@agentclientprotocol/
//! claude-agent-acp` npm bridge (not shipped by Anthropic; see the README's
//! "Harness recipes" section and issue #294 for the manual acceptance gate):
//!
//! ```text
//! [[session]]
//! name = "reviewer"
//! harness = "claude"
//! command = ["npx", "-y", "@agentclientprotocol/claude-agent-acp@0.1.5"]
//! ```
//!
//! Every [`ConfigError`] is a fail-closed validation refusal — a duplicate
//! name, bad name grammar, a missing required field for the chosen mode, or
//! an unknown top-level/session key (typos must not pass silently, so both
//! the file's own top level and every `[[session]]` table deny unknown
//! keys). The CLI maps every variant to exit 3 (ADR 0003), including "no
//! config file found" — `body run` refuses a body with no sessions ("every
//! session is explicit" — holler-client README).
//!
//! Discovery order for `body run` (issue #187): `--config PATH` >
//! `HOLLER_CONFIG` > `./sessions.toml` > `./session.toml`. [`discover`] is
//! the pure, testable form (every source passed in explicitly, so tests
//! never mutate real env vars or the process cwd); [`discover_and_load`]
//! wraps it with the real `HOLLER_CONFIG` env read for the CLI.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use holler_proto::{NameError, SessionName};
use serde::Deserialize;

/// How a session was established (the config file's own copy of the
/// distinction — kept separate from [`holler_proto::Mode`] so the wire type
/// stays out of the config-parsing boundary until [`crate::registry`] builds
/// a presence document from it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Spawn a fresh harness process (`command` required).
    Spawn,
    /// Attach to a harness session already running (`endpoint` +
    /// `session_id` required; `command` is never used).
    Attach,
}

/// How this body asks the harness to interrupt an in-flight turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    /// The ACP `session/cancel` request (default).
    Acp,
    /// An HTTP call to `endpoint` (requires `endpoint` to be set).
    Http,
}

/// One validated `[[session]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    /// The session's name (unique within the file; [`SessionName`] grammar).
    pub name: SessionName,
    /// The harness id (`holler_proto::HARNESS_IDS`; unknown ids are legal on
    /// the wire, so this is not validated against the vocabulary here).
    pub harness: String,
    /// `spawn` or `attach`.
    pub mode: SessionMode,
    /// The spawn argv (`command[0]` resolved on `PATH` or as a path). `None`
    /// for `attach` (or dropped with a warning if the file set one anyway).
    pub command: Option<Vec<String>>,
    /// The spawned process's working directory (spawn only).
    pub cwd: Option<String>,
    /// Extra environment variables for the spawned process (spawn only).
    pub env: Option<BTreeMap<String, String>>,
    /// How to interrupt an in-flight turn.
    pub interrupt: Interrupt,
    /// The attach endpoint (required for `attach`; an optional HTTP-interrupt
    /// fallback for `spawn`).
    pub endpoint: Option<String>,
    /// The existing harness session id being attached to (attach only).
    pub session_id: Option<String>,
}

/// The result of parsing + validating one config file: the session list plus
/// any non-fatal warnings (currently just "attach with `command`: ignored").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedConfig {
    pub sessions: Vec<SessionConfig>,
    pub warnings: Vec<String>,
}

/// Why a config file was refused. Every variant is a fail-closed validation
/// refusal the CLI maps to exit 3 (ADR 0003) — there is no "warning that
/// still exits 0" case here (that's [`ParsedConfig::warnings`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The file failed to parse as TOML, or violated a `deny_unknown_fields`
    /// gate (an unknown top-level or per-session key). The message is the
    /// `toml` crate's own (it already names the offending key/line).
    Toml(String),
    /// A per-session validation failure: `row` is the 0-based index into the
    /// file's `[[session]]` array, `name` is that row's (possibly invalid)
    /// `name` value, `field` is the offending key, `reason` is human-
    /// readable.
    Invalid { row: usize, name: String, field: &'static str, reason: String },
    /// No config file was found via any discovery source.
    NoConfigFound,
    /// The file could not be read (permissions, not a file, etc).
    Io(String),
}

impl ConfigError {
    /// The CLI's one-line stderr reason (ADR 0003).
    pub fn message(&self) -> String {
        match self {
            ConfigError::Toml(m) => format!("config: {m}"),
            ConfigError::Invalid { row, name, field, reason } => {
                format!("config: session[{row}] {name:?}: `{field}`: {reason}")
            }
            ConfigError::NoConfigFound => {
                "no session config found — create sessions.toml or pass --config".to_string()
            }
            ConfigError::Io(m) => format!("config: {m}"),
        }
    }
}

/// The raw file shape the `toml` crate deserializes into, before validation.
/// `deny_unknown_fields` at this level means any key besides `[[session]]`
/// is a parse error (an unknown top-level key must not pass silently).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[serde(default, rename = "session")]
    session: Vec<RawSession>,
}

/// One raw `[[session]]` table, before validation. `deny_unknown_fields`
/// catches a typo'd key (e.g. `harnes`) as a parse error rather than a
/// silently-ignored field.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSession {
    name: String,
    harness: String,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    command: Option<Vec<String>>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    env: Option<BTreeMap<String, String>>,
    #[serde(default)]
    interrupt: Option<String>,
    #[serde(default)]
    endpoint: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

/// Parse + validate a config file's contents (the TOML text, already read).
///
/// Validation happens in file order, row by row, and stops at the first
/// failure (the issue's spec: "exit 3 with the offending row and field
/// named" — one refusal, not an accumulated list). The one non-fatal case —
/// `attach` with a `command` set — is dropped and recorded in
/// [`ParsedConfig::warnings`] rather than refused.
pub fn parse(contents: &str) -> Result<ParsedConfig, ConfigError> {
    let raw: RawFile = toml::from_str(contents).map_err(|e| ConfigError::Toml(e.to_string()))?;

    let mut seen = std::collections::BTreeSet::new();
    let mut sessions = Vec::with_capacity(raw.session.len());
    let mut warnings = Vec::new();

    for (row, r) in raw.session.into_iter().enumerate() {
        let name = parse_name(row, &r.name)?;
        if !seen.insert(name.as_str().to_string()) {
            return Err(ConfigError::Invalid {
                row,
                name: r.name.clone(),
                field: "name",
                reason: "duplicate session name".to_string(),
            });
        }

        let mode = parse_mode(row, &r.name, r.mode.as_deref())?;
        let interrupt = parse_interrupt(row, &r.name, r.interrupt.as_deref())?;

        let mut command = r.command.clone();
        match mode {
            SessionMode::Spawn => {
                if command.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "command",
                        reason: "spawn mode requires `command`".to_string(),
                    });
                }
            }
            SessionMode::Attach => {
                if r.endpoint.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "endpoint",
                        reason: "attach mode requires `endpoint`".to_string(),
                    });
                }
                if r.session_id.is_none() {
                    return Err(ConfigError::Invalid {
                        row,
                        name: r.name.clone(),
                        field: "session_id",
                        reason: "attach mode requires `session_id`".to_string(),
                    });
                }
                if command.take().is_some() {
                    warnings.push(format!(
                        "session {:?} (row {row}): attach mode ignores `command` — never spawned",
                        r.name
                    ));
                }
            }
        }

        if matches!(interrupt, Interrupt::Http) && r.endpoint.is_none() {
            return Err(ConfigError::Invalid {
                row,
                name: r.name.clone(),
                field: "endpoint",
                reason: "interrupt=\"http\" requires `endpoint`".to_string(),
            });
        }

        sessions.push(SessionConfig {
            name,
            harness: r.harness,
            mode,
            command,
            cwd: r.cwd,
            env: r.env,
            interrupt,
            endpoint: r.endpoint,
            session_id: r.session_id,
        });
    }

    Ok(ParsedConfig { sessions, warnings })
}

fn parse_name(row: usize, raw: &str) -> Result<SessionName, ConfigError> {
    SessionName::parse(raw).map_err(|e: NameError| ConfigError::Invalid {
        row,
        name: raw.to_string(),
        field: "name",
        reason: e.to_string(),
    })
}

fn parse_mode(row: usize, raw_name: &str, mode: Option<&str>) -> Result<SessionMode, ConfigError> {
    match mode {
        None | Some("spawn") => Ok(SessionMode::Spawn),
        Some("attach") => Ok(SessionMode::Attach),
        Some(other) => Err(ConfigError::Invalid {
            row,
            name: raw_name.to_string(),
            field: "mode",
            reason: format!("unknown mode {other:?} (expected \"spawn\" or \"attach\")"),
        }),
    }
}

fn parse_interrupt(row: usize, raw_name: &str, interrupt: Option<&str>) -> Result<Interrupt, ConfigError> {
    match interrupt {
        None | Some("acp") => Ok(Interrupt::Acp),
        Some("http") => Ok(Interrupt::Http),
        Some(other) => Err(ConfigError::Invalid {
            row,
            name: raw_name.to_string(),
            field: "interrupt",
            reason: format!("unknown interrupt {other:?} (expected \"acp\" or \"http\")"),
        }),
    }
}

/// Read + parse + validate the file at `path`.
pub fn load(path: &Path) -> Result<ParsedConfig, ConfigError> {
    let contents =
        std::fs::read_to_string(path).map_err(|e| ConfigError::Io(format!("{}: {e}", path.display())))?;
    parse(&contents)
}

/// Resolve which config file `body run` should read, in the issue's
/// discovery order: `flag` (`--config PATH`) > `env_config` (`HOLLER_CONFIG`,
/// already read by the caller) > `<cwd>/sessions.toml` (if it exists) >
/// `<cwd>/session.toml` (if it exists) > none.
///
/// A `flag` or `env_config` path is returned as-is, unchecked — a path that
/// does not exist is a later [`load`] I/O error, not a discovery miss. The
/// two default filenames are only chosen when they actually exist, so an
/// empty cwd correctly falls through to `None`.
pub fn discover(flag: Option<&Path>, env_config: Option<&Path>, cwd: &Path) -> Option<PathBuf> {
    if let Some(p) = flag {
        return Some(p.to_path_buf());
    }
    if let Some(p) = env_config {
        return Some(p.to_path_buf());
    }
    let sessions_toml = cwd.join("sessions.toml");
    if sessions_toml.is_file() {
        return Some(sessions_toml);
    }
    let session_toml = cwd.join("session.toml");
    if session_toml.is_file() {
        return Some(session_toml);
    }
    None
}

/// The real `body run` entry point: reads `HOLLER_CONFIG` itself (kept out of
/// [`discover`] so that function stays a pure, race-free unit under test),
/// resolves the path, and loads it. `flag` is the `--config` CLI value, if
/// given; `cwd` is the directory relative discovery resolves against.
pub fn discover_and_load(flag: Option<&str>, cwd: &Path) -> Result<ParsedConfig, ConfigError> {
    let flag_path = flag.map(Path::new);
    let env = std::env::var("HOLLER_CONFIG").ok();
    let env_path = env.as_deref().map(Path::new);
    match discover(flag_path, env_path, cwd) {
        Some(path) => load(&path),
        None => Err(ConfigError::NoConfigFound),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #187
mod tests {
    use super::*;

    #[test]
    fn message_names_row_and_field() {
        let e = ConfigError::Invalid {
            row: 2,
            name: "alpha".to_string(),
            field: "command",
            reason: "spawn mode requires `command`".to_string(),
        };
        let m = e.message();
        assert!(m.contains("session[2]"));
        assert!(m.contains("alpha"));
        assert!(m.contains("command"));
    }

    #[test]
    fn no_config_found_hint_mentions_both_flags() {
        let m = ConfigError::NoConfigFound.message();
        assert!(m.contains("sessions.toml"));
        assert!(m.contains("--config"));
    }
}
