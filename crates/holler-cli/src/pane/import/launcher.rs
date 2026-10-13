//! The launcher's port and directory table (#650), a format this story defines (ADR-0021
//! section 13): one TOML table per pane, keyed by the pane's name, with `port` (an integer from
//! 1 to 65535), `cwd` (an absolute path) and `command` (an array of strings, an argv; a string
//! is a shell command, which a record never holds). The port is never derived from the name.
//!
//! The file is read once for every pane, with `toml::from_str` into that shape and nothing
//! else, so a file that is not TOML, a value of the wrong type and a key the shape does not
//! have are all one refusal, which every pane reports as `source-unreadable`. Its message
//! names the file and the line and column of the error's span, and never the error's text:
//! `toml`'s message quotes the offending line of the file, which can hold a secret. A pane with
//! no entry, or an entry without its cwd, is `source-missing`. An entry without a port is kept
//! (#831): a pane that runs Claude Code has no server, so which pane needs a port is known only
//! once its tmux session is read, and the join reports a port-less OpenCode pane
//! (`source-missing`, the same words).

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::Path;

use holler_pane::findings::quoted;
use holler_pane::import::files::{LauncherCommand, LauncherEntry};
use holler_pane::import::findings::{Finding, ImportFinding};
use holler_pane::{Argv, PaneName};
use serde::{Deserialize, Deserializer};

/// What the table is, in the sentence a refusal of the whole file gives.
const SHAPE: &str = "it is not TOML, or not one table per pane with port, cwd and command";

/// One entry, as the file holds it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    command: Option<RawCommand>,
}

/// A `command` as the file holds it.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCommand {
    Argv(Vec<String>),
    /// A string. Its text is checked to be a string and never kept: a record never holds a
    /// shell command.
    Shell(#[serde(deserialize_with = "a_string")] ()),
}

/// Read a string and keep nothing of it.
fn a_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(), D::Error> {
    String::deserialize(deserializer).map(drop)
}

/// Why the file gives no entry at all.
enum Unread {
    /// It does not exist.
    Missing,
    /// It cannot be read, or is refused; the sentence says why, with no text of the file.
    Refused(String),
}

/// The launcher table, read once.
pub(super) struct LauncherTable {
    /// The file's name, quoted, as a message names it.
    file: String,
    entries: Result<BTreeMap<String, RawEntry>, Unread>,
}

impl LauncherTable {
    /// Read and parse the table at `path`.
    pub(super) fn read(path: &Path) -> Self {
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let entries = match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|error| refused(&text, &error)),
            Err(error) if error.kind() == ErrorKind::NotFound => Err(Unread::Missing),
            Err(error) => Err(Unread::Refused(format!(
                "cannot be read ({})",
                error.kind()
            ))),
        };
        Self {
            file: quoted(&file),
            entries,
        }
    }

    /// The entry of `pane`, or the finding that says why there is none.
    pub(super) fn entry(&self, pane: &PaneName) -> Result<LauncherEntry, Finding> {
        let file = &self.file;
        let entries = match &self.entries {
            Ok(entries) => entries,
            Err(Unread::Missing) => {
                let message = format!("the launcher table {file} does not exist");
                return Err(Finding::about(pane, ImportFinding::SourceMissing, message));
            }
            Err(Unread::Refused(why)) => {
                let message = format!("the launcher table {file} {why}");
                return Err(Finding::about(
                    pane,
                    ImportFinding::SourceUnreadable,
                    message,
                ));
            }
        };
        let Some(raw) = entries.get(pane.as_str()) else {
            let message = format!("the launcher table {file} has no entry for {pane}");
            return Err(Finding::about(pane, ImportFinding::SourceMissing, message));
        };
        self.checked(pane, raw)
    }

    /// The entry `raw` of `pane`, when it has an absolute cwd and, if it has a port, a port from
    /// 1 to 65535.
    fn checked(&self, pane: &PaneName, raw: &RawEntry) -> Result<LauncherEntry, Finding> {
        let about = format!("the entry for {pane} in the launcher table {}", self.file);
        let Some(cwd) = &raw.cwd else {
            let lacks = match raw.port {
                None => "no port and no cwd",
                Some(_) => "no cwd",
            };
            let message = format!("{about} has {lacks}");
            return Err(Finding::about(pane, ImportFinding::SourceMissing, message));
        };
        let port = raw.port;
        let fault = if port == Some(0) {
            Some("has port 0, and a port runs from 1 to 65535")
        } else if !Path::new(cwd).is_absolute() {
            Some("has a cwd that is not an absolute path")
        } else {
            None
        };
        if let Some(fault) = fault {
            let message = format!("{about} {fault}");
            return Err(Finding::about(
                pane,
                ImportFinding::SourceUnreadable,
                message,
            ));
        }
        Ok(LauncherEntry {
            port,
            about,
            cwd: cwd.clone(),
            command: match &raw.command {
                None => LauncherCommand::Absent,
                Some(RawCommand::Argv(argv)) => LauncherCommand::Argv(Argv::new(argv.clone())),
                Some(RawCommand::Shell(())) => LauncherCommand::Shell,
            },
        })
    }
}

/// The refusal of the whole file: where the error is (the line and column of the start of its
/// span in `text`, 1-based), never what it says.
fn refused(text: &str, error: &toml::de::Error) -> Unread {
    let at = error
        .span()
        .and_then(|span| text.get(..span.start))
        .map(|before| {
            let line = before.matches('\n').count() + 1;
            let column = before.chars().rev().take_while(|&c| c != '\n').count() + 1;
            format!(" at line {line}, column {column}")
        })
        .unwrap_or_default();
    Unread::Refused(format!("cannot be read{at}: {SHAPE}"))
}
