//! The registry file (D2): one versioned JSON document, loaded once at startup and saved
//! whole on every write.
//!
//! The pane registry's file is `<state dir>/hub/panes.json`, beside `holds.json`:
//!
//! ```text
//! {"version": 1, "cursor": <head>,
//!  "entries": [{"cursor": <c>, "name": "<pane>", "pane": {<the Pane record>} or null}, ...]}
//! ```
//!
//! - `cursor` is the change feed's head, so cursors keep increasing across a restart.
//! - An entry is the last change filed under a name, a `PaneEvent`: a put carries the record
//!   and a delete leaves a tombstone (`"pane": null`). A deletion therefore survives a
//!   restart, and a watcher that resumes across one still learns of it (`feed.rs`, rule 4).
//!   Entries are sorted by name.
//!
//! [`load_doc`] and [`save_doc`] are generic over the entry type, and their checks read an
//! entry through [`RegistryEntry`]. #661's `profiles.json` therefore calls them from
//! `profile/` with `ProfileEvent` entries, without editing this directory.
//!
//! # Saving
//!
//! [`save_doc`] creates the directory when it is missing, then writes through the hub's one
//! state-file writer, `holler_proto::atomic_file::write_atomic`, at `0600`. A reader sees
//! either the old document or the new one, never a partial file. As with the hub's other
//! state files there is no fsync (#483).
//!
//! # Loading fails closed (D3)
//!
//! A missing file is an empty registry. Anything else that keeps the file from being read
//! back exactly is a [`Problem`]:
//!
//! - an unreadable file;
//! - a document that does not parse, including a record with a field this build does not
//!   know (the records refuse unknown fields);
//! - a version other than [`VERSION`];
//! - an entry that fails a check: a name filed twice, an entry whose name differs from its
//!   record's, a cursor of 0 or past the head, two entries with one cursor, or a record at
//!   generation 0.
//!
//! The caller fails closed on a problem and never rewrites or moves the file.
//!
//! **A problem never quotes the file.** serde's messages can quote file content (a value of
//! the wrong type, the name of an unknown field). A problem is therefore built only from an
//! error category, a line and a column, a version number, a record's name (a validated pane
//! name) and an I/O error, so it is safe to log and to send to a client.

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::path::Path;

use holler_pane::Cursor;
use holler_proto::atomic_file::write_atomic;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;

use super::RegistryEntry;

/// The document version this build reads and writes.
pub(crate) const VERSION: u64 = 1;

/// The mode of a registry file: hub state, readable by the hub's own user only.
const FILE_MODE: u32 = 0o600;

/// A registry file: its version, the change feed's head, and one entry per name, sorted
/// by name.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Doc<E> {
    pub(crate) version: u64,
    pub(crate) cursor: Cursor,
    pub(crate) entries: Vec<E>,
}

/// The version alone. It is read before the whole document, so a file written by a newer
/// build is reported by its version rather than as a shape this build does not know.
#[derive(Deserialize)]
struct Versioned {
    version: u64,
}

/// Why a registry file cannot be used: it cannot be read back (a corrupt or unreadable
/// file), or a save failed. It is built only from the parts the module docs list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Problem(String);

impl Problem {
    /// The `what` of the error the problem causes: the registry (`label`), its file, and
    /// the problem.
    pub(crate) fn what(&self, label: &str, path: &Path) -> String {
        format!("{label} {}: {self}", path.display())
    }

    /// A document that does not parse, described by the error's category and position
    /// only. serde's own message is never used: it can quote the file.
    fn parse(err: &serde_json::Error) -> Self {
        let kind = match err.classify() {
            Category::Syntax => "not valid JSON",
            Category::Eof => "JSON that ends early",
            Category::Data => "a value this hub cannot read",
            Category::Io => "a read error",
        };
        Self(format!(
            "{kind} at line {}, column {}",
            err.line(),
            err.column()
        ))
    }

    /// A save that failed.
    fn not_written(err: &io::Error) -> Self {
        Self(format!("not written: {err}"))
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Read the registry file at `path`. `Ok(None)` when there is no file, the document when
/// it reads back and passes every check, and the [`Problem`] otherwise.
pub(crate) fn load_doc<E>(path: &Path) -> Result<Option<Doc<E>>, Problem>
where
    E: RegistryEntry + DeserializeOwned,
{
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(Problem(format!("cannot be read: {}", err.kind()))),
    };
    let Versioned { version } = serde_json::from_slice(&bytes).map_err(|e| Problem::parse(&e))?;
    if version != VERSION {
        return Err(Problem(format!(
            "version {version} is not one this hub reads (it reads version {VERSION})"
        )));
    }
    let doc: Doc<E> = serde_json::from_slice(&bytes).map_err(|e| Problem::parse(&e))?;
    check(&doc)?;
    Ok(Some(doc))
}

/// Write `doc` to `path`: create the directory when it is missing, then replace the file
/// atomically at `0600`. On failure the file is as it was.
pub(crate) fn save_doc<E: Serialize>(path: &Path, doc: &Doc<E>) -> Result<(), Problem> {
    let bytes = serde_json::to_vec_pretty(doc)
        .map_err(|_| Problem("not written: the document could not be encoded".to_owned()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Problem::not_written(&e))?;
    }
    write_atomic(path, &bytes, FILE_MODE).map_err(|e| Problem::not_written(&e))
}

/// The checks across a parsed document's entries. The version was checked before parsing.
fn check<E: RegistryEntry>(doc: &Doc<E>) -> Result<(), Problem> {
    let mut names = BTreeSet::new();
    let mut cursors = BTreeSet::new();
    for entry in &doc.entries {
        check_entry(entry, doc.cursor)?;
        if !names.insert(entry.name()) {
            return Err(Problem(format!("{} has more than one entry", entry.name())));
        }
        if !cursors.insert(entry.cursor()) {
            return Err(Problem(format!(
                "more than one entry has cursor {}",
                entry.cursor().0
            )));
        }
    }
    Ok(())
}

/// The checks of one entry: its cursor names a change at or before the head, and its
/// record, when it has one, is filed under the record's own name at a generation of at
/// least 1.
fn check_entry<E: RegistryEntry>(entry: &E, head: Cursor) -> Result<(), Problem> {
    let (name, cursor) = (entry.name(), entry.cursor().0);
    if cursor == 0 || cursor > head.0 {
        return Err(Problem(format!(
            "the entry {name} has cursor {cursor}, outside 1..={}",
            head.0
        )));
    }
    match entry.record() {
        Some((record, _)) if record != name => Err(Problem(format!(
            "the entry {name} holds the record of {record}"
        ))),
        Some((_, 0)) => Err(Problem(format!("the record {name} is at generation 0"))),
        _ => Ok(()),
    }
}
