//! The state root and each pane directory's files (#650): `holler-cli` reads the old
//! launcher's files and hands `holler_pane::import` plain values and the problems it met, so
//! the engine knows no file format (ADR-0021 section 5).
//!
//! - **The state root** holds one directory per pane. A directory is a pane directory when it
//!   holds at least one of `pane.env`, `last-session`, `sessions.toml` and `owned-sessions`;
//!   its name is the pane's name (`PaneName::parse`) and nothing more. A directory that holds
//!   none is `ignored-entry`; one that holds a pane file under a name that is not a pane name
//!   is `source-unreadable`. Files at the root (the launcher table among them) are not
//!   entries.
//! - **`pane.env`**: `pane_env`'s grammar. **The launcher table**: `launcher`'s.
//! - **`last-session`**: its whole content, trimmed, is one session id; empty is absent.
//! - **`sessions.toml`**: `holler_body::config::load`, the attach-config grammar, never a
//!   second parser. Its error text quotes the file, so a refusal names the file and, for a
//!   row that is refused, the row and the field, and nothing else.
//! - **`owned-sessions`**: one session id per line; blank lines and `#` lines are skipped.
//!
//! A message names a pane file as `<pane>/<file>`, relative to the state root. The verb reads
//! these files and writes none.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use holler_body::config::{load, ConfigError};
use holler_pane::findings::quoted;
use holler_pane::import::files::{EnvValues, FileState, PaneFiles, SessionFiles, SessionsTable};
use holler_pane::import::findings::{Finding, ImportFinding};
use holler_pane::{PaneError, PaneName};

use super::launcher::LauncherTable;
use super::pane_env::{self, EnvError};

const PANE_ENV: &str = "pane.env";
const LAST_SESSION: &str = "last-session";
const SESSIONS_TOML: &str = "sessions.toml";
const OWNED_SESSIONS: &str = "owned-sessions";

/// The files that make a directory of the state root a pane directory.
const PANE_FILES: [&str; 4] = [PANE_ENV, LAST_SESSION, SESSIONS_TOML, OWNED_SESSIONS];

/// What `pane.env` must give a value for, in the message of a line that gives a bad one.
const ENV_VALUES: &str = "OC_MODEL is PROVIDER/MODEL_ID, OC_EFFORT is not empty, the context \
                          ceilings are unsigned integers and OC_DEFAULT_AGENT is an agent key";

/// The directories of the state root.
pub(super) struct Listing {
    /// The pane directories, by name, in name order.
    pub panes: Vec<(PaneName, PathBuf)>,
    /// The findings about no pane: `ignored-entry`, and `source-unreadable` for a directory of
    /// pane files whose name is not a pane name.
    pub others: Vec<Finding>,
}

/// List the directories of the state root `root`. A root that cannot be listed is
/// `unavailable`.
pub(super) fn list(root: &Path) -> Result<Listing, PaneError> {
    let unlisted = |error: std::io::Error| PaneError::Unavailable {
        what: format!(
            "the state root {:?} cannot be listed ({})",
            root.display().to_string(),
            error.kind()
        ),
    };
    let mut listing = Listing {
        panes: Vec::new(),
        others: Vec::new(),
    };
    for entry in std::fs::read_dir(root).map_err(unlisted)? {
        let dir = entry.map_err(unlisted)?.path();
        if dir.is_dir() {
            listing.add(dir);
        }
    }
    listing.panes.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(listing)
}

impl Listing {
    /// Add the directory `dir` as a pane directory, or a finding about no pane.
    fn add(&mut self, dir: PathBuf) {
        let text = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let holds_pane_files = PANE_FILES.iter().any(|file| dir.join(file).exists());
        let name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| PaneName::parse(name).ok());
        match (holds_pane_files, name) {
            (true, Some(name)) => self.panes.push((name, dir)),
            (true, None) => self.others.push(Finding::unattached(
                ImportFinding::SourceUnreadable,
                format!(
                    "the directory {} holds pane files, and its name is not a pane name",
                    quoted(&text)
                ),
            )),
            (false, _) => self.others.push(Finding::unattached(
                ImportFinding::IgnoredEntry,
                format!(
                    "the directory {} holds none of {}, so it is not a pane directory",
                    quoted(&text),
                    PANE_FILES.join(", ")
                ),
            )),
        }
    }
}

/// What the files of the pane `name`, in the directory `dir`, say, with its entry of the
/// launcher table.
pub(super) fn read(name: &PaneName, dir: &Path, launcher: &LauncherTable) -> PaneFiles {
    let mut problems = Vec::new();
    let env = kept(read_env(name, dir), &mut problems);
    let launcher = kept(launcher.entry(name), &mut problems);
    let sessions = SessionFiles {
        last: state(text(name, dir, LAST_SESSION), &mut problems, |text| {
            Some(text.trim().to_owned()).filter(|id| !id.is_empty())
        }),
        table: sessions_table(name, dir, &mut problems),
        owned: state(text(name, dir, OWNED_SESSIONS), &mut problems, |text| {
            let ids = text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned);
            Some(ids.collect())
        }),
    };
    PaneFiles {
        name: name.clone(),
        env,
        launcher,
        sessions,
        problems,
    }
}

/// The value of `result`, or `None` with its problem kept.
fn kept<T>(result: Result<T, Finding>, problems: &mut Vec<Finding>) -> Option<T> {
    result.map_err(|problem| problems.push(problem)).ok()
}

/// What a session file held: `value` of its text (`None` is absent, as an empty
/// `last-session` is), absent when it does not exist, unreadable with its problem kept.
fn state<T>(
    text: Result<Option<String>, Finding>,
    problems: &mut Vec<Finding>,
    value: impl FnOnce(&str) -> Option<T>,
) -> FileState<T> {
    match text {
        Err(problem) => {
            problems.push(problem);
            FileState::Unreadable
        }
        Ok(text) => text
            .as_deref()
            .and_then(value)
            .map_or(FileState::Absent, FileState::Present),
    }
}

/// The text of the pane file `file` of `name`: `None` when it does not exist.
fn text(name: &PaneName, dir: &Path, file: &str) -> Result<Option<String>, Finding> {
    match std::fs::read_to_string(dir.join(file)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Finding::about(
            name,
            ImportFinding::SourceUnreadable,
            format!("{name}/{file} cannot be read ({})", error.kind()),
        )),
    }
}

/// What `pane.env` gives, or the problem: a missing file or variable, or a line named by its
/// number alone.
fn read_env(name: &PaneName, dir: &Path) -> Result<EnvValues, Finding> {
    let Some(text) = text(name, dir, PANE_ENV)? else {
        let message = format!(
            "{name}/{PANE_ENV} does not exist, and the record needs the model, effort and \
             context ceilings it sets"
        );
        return Err(Finding::about(name, ImportFinding::SourceMissing, message));
    };
    pane_env::parse(&text).map_err(|error| {
        let (kind, message) = match error {
            EnvError::Grammar(line) => (
                ImportFinding::SourceUnreadable,
                format!("{name}/{PANE_ENV}: line {line} is not of the form [export ]NAME=value"),
            ),
            EnvError::Value(line) => (
                ImportFinding::SourceUnreadable,
                format!(
                    "{name}/{PANE_ENV}: line {line} gives a variable a value it does not take \
                     ({ENV_VALUES})"
                ),
            ),
            EnvError::Missing(variables) => (
                ImportFinding::SourceMissing,
                format!("{name}/{PANE_ENV} does not set {}", variables.join(" or ")),
            ),
        };
        Finding::about(name, kind, message)
    })
}

/// What `sessions.toml` says, read through `holler_body::config::load`.
fn sessions_table(
    name: &PaneName,
    dir: &Path,
    problems: &mut Vec<Finding>,
) -> FileState<SessionsTable> {
    let path = dir.join(SESSIONS_TOML);
    if !path.exists() {
        return FileState::Absent;
    }
    match load(&path) {
        Ok(parsed) => FileState::Present(SessionsTable {
            session_ids: parsed
                .sessions
                .iter()
                .filter_map(|row| row.session_id.clone())
                .collect(),
            harnesses: parsed.sessions.into_iter().map(|row| row.harness).collect(),
        }),
        Err(error) => {
            let message = match error {
                ConfigError::Toml(_) => format!("{name}/{SESSIONS_TOML} is not valid TOML"),
                ConfigError::Invalid { row, field, .. } => format!(
                    "{name}/{SESSIONS_TOML}: [[session]] table {} (counting from 1) has an \
                     invalid `{field}`",
                    row + 1
                ),
                ConfigError::Io(_) | ConfigError::NoConfigFound => {
                    format!("{name}/{SESSIONS_TOML} cannot be read")
                }
            };
            problems.push(Finding::about(
                name,
                ImportFinding::SourceUnreadable,
                message,
            ));
            FileState::Unreadable
        }
    }
}
