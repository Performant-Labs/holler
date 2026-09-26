//! The session hold registry (issue #442, umbrella #437): the hub's record of
//! which sessions an operator has stopped new work to.
//!
//! # Semantics (pinned by the umbrella's decisions)
//!
//! - A hold is keyed by the routable session name (`<label>/<session>`), not
//!   by a connection, so it survives a body's disconnect, a roster prune, a
//!   re-join and a hub restart. Nothing in this module reads or writes the
//!   roster: whether a session has ever been seen is the caller's question
//!   (`control_hold`), not this registry's.
//! - [`Holds::hold`] on a held session is a no-op that keeps the original
//!   reason and since-time; [`Holds::release`] on an unheld session is a
//!   successful no-op.
//! - The hold is orthogonal to the session's own state; nothing here looks at
//!   it.
//! - **One enforcement point.** [`Holds::check`] is called from exactly one
//!   place that can deliver a prompt, `circuit::dispatch::send_prompt`; the
//!   test `tests/hold_single_path_test.rs` (holler-cli) fails if a second
//!   `session/prompt` sender appears in the hub.
//!
//! # Concurrency
//!
//! The map is behind one `std::sync::Mutex` and every entry point is
//! synchronous with no `await` inside, so a `check` and a `hold` are totally
//! ordered: a prompt whose `check` ran before a `hold` was accepted before
//! the hold took effect; one whose `check` ran after is refused. A `hold`
//! that has returned is visible to every later `check`.
//!
//! # Persistence
//!
//! Holds live in `<state dir>/hub/holds.json`, written atomically (a `0600`
//! temp file in the same directory, then `rename`) the same way the hub's
//! other state files are kept private. The hub never fails to start over it:
//!
//! - **Corrupt file** (unparseable, or a shape this version does not know):
//!   it is moved aside to `holds.json.corrupt-<unix time>` (never
//!   overwritten), a `warn` event (the loudest level the hub has, visible at the default level) is logged, and the hub starts with no holds. The
//!   operator can inspect the moved file; nothing is silently discarded.
//! - **Unreadable file, or a corrupt one that cannot be moved aside**: the
//!   hub starts with no holds and **stops writing** the file for its life, so
//!   it can never overwrite holds it could not read. An `error` is logged.
//! - **Unwritable directory**: the hold still takes effect in memory (a hold
//!   is never dropped because its record could not be saved), an `error` is
//!   logged, and the caller is told `persisted: false` so `holler hold` can
//!   warn that the hold will not survive a restart.
//!
//! The blocking file write happens on the caller's thread, inside a separate
//! persistence lock (never the map lock, so a slow disk never delays the
//! `say` path's `check`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use holler_proto::log::{self, Component, Direction, Event, Severity};
use serde::{Deserialize, Serialize};

use crate::state::HubState;

/// The longest reason kept, in characters. Longer text is cut (not refused):
/// the reason is a note for the next reader, and it is echoed in an error
/// message and on the roster.
pub const MAX_REASON_CHARS: usize = 200;

/// The on-disk format version this build reads and writes.
const FILE_VERSION: u32 = 1;

/// The registry key for a session: `<label>/<session>`, where `label` is the
/// token label its body authenticated as. Every place that checks or sets a
/// hold builds the key with this one function.
pub fn session_key(label: &str, session: &str) -> String {
    format!("{label}/{session}")
}

/// One hold: the operator's reason (if any) and when it was set (RFC 3339).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub since: String,
}

impl HoldInfo {
    /// The wire error a refused prompt carries (`-32011 session_held`).
    pub fn refusal(&self) -> holler_proto::WireError {
        holler_proto::WireError::session_held(self.reason.as_deref(), &self.since)
    }
}

/// The result of [`Holds::hold`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldOutcome {
    /// The hold now in force (the original one when the session was already held).
    pub info: HoldInfo,
    /// `false` when the session was already held (an idempotent repeat).
    pub newly_held: bool,
    /// Whether the change was written to disk (see the module docs).
    pub persisted: bool,
}

/// The result of [`Holds::release`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseOutcome {
    /// `false` when the session was not held (an idempotent repeat).
    pub was_held: bool,
    /// Whether the change was written to disk.
    pub persisted: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileDoc {
    version: u32,
    holds: BTreeMap<String, HoldInfo>,
}

struct Persist {
    /// `None`: an in-memory registry (unit tests, a `Registry::new()`).
    path: Option<PathBuf>,
    /// Set when the existing file could not be read or moved aside: writing
    /// would overwrite holds this hub could not see.
    blocked: bool,
    /// Set while the file on disk is behind memory (a write failed). Every
    /// later change, including an idempotent repeat, retries the write and
    /// reports `persisted` from this, never from whether the call changed
    /// anything.
    unsaved: bool,
}

struct Shared {
    map: Mutex<BTreeMap<String, HoldInfo>>,
    persist: Mutex<Persist>,
}

/// The hold registry. Cheap to clone (shared behind an `Arc`).
#[derive(Clone)]
pub struct Holds {
    shared: Arc<Shared>,
}

impl Default for Holds {
    fn default() -> Self {
        Self::in_memory()
    }
}

/// Trim, drop control characters (an operator's text is echoed to other
/// people's terminals) and cap at [`MAX_REASON_CHARS`]; an empty result is
/// no reason at all.
pub fn sanitize_reason(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    Some(cleaned.chars().take(MAX_REASON_CHARS).collect())
}

fn emit(severity: Severity, method: &'static str, fields: Vec<(&'static str, String)>) {
    log::emit(&Event {
        component: Component::Control,
        severity,
        direction: Direction::Local,
        method,
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

impl Holds {
    /// A registry that never touches disk.
    pub fn in_memory() -> Self {
        Self::with_persist(BTreeMap::new(), Persist { path: None, blocked: false, unsaved: false })
    }

    fn with_persist(map: BTreeMap<String, HoldInfo>, persist: Persist) -> Self {
        Self { shared: Arc::new(Shared { map: Mutex::new(map), persist: Mutex::new(persist) }) }
    }

    /// Load the registry from `<state dir>/hub/holds.json`. Never fails: see
    /// the module docs for what a missing, corrupt or unreadable file means.
    pub fn load(state: &HubState) -> Self {
        let path = state.hub_dir.join("holds.json");
        let (map, blocked) = read_file(&path);
        Self::with_persist(map, Persist { path: Some(path), blocked, unsaved: blocked })
    }

    /// The hold on `key`, if any. The only thing the prompt path calls.
    pub fn check(&self, key: &str) -> Option<HoldInfo> {
        self.shared.map.lock().unwrap_or_else(PoisonError::into_inner).get(key).cloned()
    }

    /// Every key currently held, with its hold (sorted by key).
    pub fn snapshot(&self) -> BTreeMap<String, HoldInfo> {
        self.shared.map.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The roster fields for `key`.
    pub fn row_hold(&self, key: &str) -> holler_proto::SessionHold {
        match self.check(key) {
            Some(h) => holler_proto::SessionHold::held(h.reason, h.since),
            None => holler_proto::SessionHold::default(),
        }
    }

    /// Hold `key`. Repeating keeps the original reason and since-time.
    pub fn hold(&self, key: &str, reason: Option<&str>) -> HoldOutcome {
        let (info, newly_held) = {
            let mut map = self.shared.map.lock().unwrap_or_else(PoisonError::into_inner);
            match map.get(key) {
                Some(existing) => (existing.clone(), false),
                None => {
                    let info = HoldInfo { reason: reason.and_then(sanitize_reason), since: log::timestamp() };
                    map.insert(key.to_owned(), info.clone());
                    (info, true)
                }
            }
        };
        let persisted = self.sync(newly_held);
        if newly_held {
            emit(Severity::Info, "session_held", vec![("session", key.to_owned())]);
        }
        HoldOutcome { info, newly_held, persisted }
    }

    /// Release `key`. Releasing a session that is not held is a no-op.
    pub fn release(&self, key: &str) -> ReleaseOutcome {
        let was_held = self.shared.map.lock().unwrap_or_else(PoisonError::into_inner).remove(key).is_some();
        let persisted = self.sync(was_held);
        if was_held {
            emit(Severity::Info, "session_released", vec![("session", key.to_owned())]);
        }
        ReleaseOutcome { was_held, persisted }
    }

    /// Bring the file up to date with memory when `changed` (or when an earlier
    /// write failed and the file is still behind), and report whether it is.
    fn sync(&self, changed: bool) -> bool {
        let behind = self.shared.persist.lock().unwrap_or_else(PoisonError::into_inner).unsaved;
        if changed || behind {
            self.persist()
        } else {
            true
        }
    }

    /// Write the current map. Serialised by the persistence lock, and the
    /// snapshot is taken *after* acquiring it, so whichever writer runs last
    /// writes the newest state no matter how the writers interleaved.
    fn persist(&self) -> bool {
        let mut persist = self.shared.persist.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(path) = persist.path.clone() else { return true };
        if persist.blocked {
            persist.unsaved = true;
            return false;
        }
        let doc = FileDoc { version: FILE_VERSION, holds: self.snapshot() };
        match write_atomic(&path, &doc) {
            Ok(()) => {
                persist.unsaved = false;
                true
            }
            Err(e) => {
                persist.unsaved = true;
                emit(
                    Severity::Warn,
                    "hold_state_write_failed",
                    vec![
                        ("path", path.display().to_string()),
                        ("error", e.to_string()),
                        ("effect", "the hold is in force in memory but will not survive a hub restart".to_owned()),
                    ],
                );
                false
            }
        }
    }
}

/// Read the state file. Returns the holds and whether writing must stay
/// blocked (the file exists but could not be read or set aside).
fn read_file(path: &std::path::Path) -> (BTreeMap<String, HoldInfo>, bool) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (BTreeMap::new(), false),
        Err(e) => {
            emit(
                Severity::Warn,
                "hold_state_unreadable",
                vec![
                    ("path", path.display().to_string()),
                    ("error", e.to_string()),
                    ("effect", "starting with no holds; the file will not be overwritten".to_owned()),
                ],
            );
            return (BTreeMap::new(), true);
        }
    };
    match serde_json::from_slice::<FileDoc>(&bytes) {
        Ok(doc) if doc.version == FILE_VERSION => (doc.holds, false),
        Ok(doc) => set_aside(path, &format!("unknown hold state version {}", doc.version)),
        Err(e) => set_aside(path, &format!("not valid hold state: {e}")),
    }
}

/// Move an unusable file out of the way (never overwriting an earlier one).
fn set_aside(path: &std::path::Path, why: &str) -> (BTreeMap<String, HoldInfo>, bool) {
    let stamp = holler_proto::now_secs();
    let mut target = path.with_extension(format!("json.corrupt-{stamp}"));
    let mut n = 1u32;
    while target.exists() {
        target = path.with_extension(format!("json.corrupt-{stamp}-{n}"));
        n += 1;
    }
    match std::fs::rename(path, &target) {
        Ok(()) => {
            emit(
                Severity::Warn,
                "hold_state_corrupt",
                vec![
                    ("path", path.display().to_string()),
                    ("problem", why.to_owned()),
                    ("moved_to", target.display().to_string()),
                    ("effect", "starting with no holds; re-hold the sessions that need it".to_owned()),
                ],
            );
            (BTreeMap::new(), false)
        }
        Err(e) => {
            emit(
                Severity::Warn,
                "hold_state_corrupt",
                vec![
                    ("path", path.display().to_string()),
                    ("problem", why.to_owned()),
                    ("error", format!("could not move it aside: {e}")),
                    ("effect", "starting with no holds; the file will not be overwritten".to_owned()),
                ],
            );
            (BTreeMap::new(), true)
        }
    }
}

/// Write `doc` to `path` atomically: a `0600` temp file beside it, synced,
/// then renamed over it.
fn write_atomic(path: &std::path::Path, doc: &FileDoc) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("json.tmp");
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&tmp)?;
    let body = serde_json::to_vec_pretty(doc).map_err(std::io::Error::other)?;
    file.write_all(&body)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #442
mod tests {
    use super::*;

    fn state_in(dir: &std::path::Path) -> HubState {
        let s = HubState::from_root(dir.to_path_buf());
        std::fs::create_dir_all(&s.hub_dir).unwrap();
        s
    }

    #[test]
    fn hold_and_release_are_idempotent_and_keep_the_first_reason() {
        let h = Holds::in_memory();
        let first = h.hold("io/alpha", Some("freeze"));
        assert!(first.newly_held);
        let again = h.hold("io/alpha", Some("a different reason"));
        assert!(!again.newly_held);
        assert_eq!(again.info, first.info, "a repeat keeps the original reason and since");
        assert!(h.release("io/alpha").was_held);
        assert!(!h.release("io/alpha").was_held);
        assert!(!h.release("io/never-held").was_held);
        assert!(h.check("io/alpha").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_repeated_hold_after_a_failed_write_retries_and_reports_the_truth() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        let h = Holds::load(&state);
        std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        let can_write_anyway = std::fs::File::create(state.hub_dir.join("probe")).is_ok();
        let first = h.hold("io/alpha", Some("x"));
        let repeat = h.hold("io/alpha", Some("x"));
        let rel_repeat = h.release("io/never");
        if !can_write_anyway {
            assert!(!first.persisted);
            assert!(!repeat.persisted, "a repeat must not claim the hold is on disk when it is not");
            assert!(!rel_repeat.persisted, "nor may a no-op release");
        }
        std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let healed = h.hold("io/alpha", Some("x"));
        assert!(healed.persisted && !healed.newly_held, "the repeat retries the write once the directory is writable");
        assert!(Holds::load(&state).check("io/alpha").is_some());
    }

    #[test]
    fn a_hold_is_per_session() {
        let h = Holds::in_memory();
        h.hold("io/alpha", None);
        assert!(h.check("io/alpha").is_some());
        assert!(h.check("io/beta").is_none());
        assert!(h.check("other/alpha").is_none());
    }

    #[test]
    fn reason_is_sanitised() {
        assert_eq!(sanitize_reason("  freeze  "), Some("freeze".into()));
        assert_eq!(sanitize_reason("\u{1b}[31mred\u{7}"), Some("[31mred".into()));
        assert_eq!(sanitize_reason("   "), None);
        assert_eq!(sanitize_reason(&"x".repeat(500)).map(|r| r.chars().count()), Some(MAX_REASON_CHARS));
    }

    #[test]
    fn holds_survive_a_reload_and_the_file_is_private() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        let h = Holds::load(&state);
        let out = h.hold("io/alpha", Some("freeze"));
        assert!(out.persisted);
        h.hold("io/beta", None);
        h.release("io/beta");
        let reloaded = Holds::load(&state);
        assert_eq!(reloaded.check("io/alpha"), Some(out.info));
        assert!(reloaded.check("io/beta").is_none());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(state.hub_dir.join("holds.json")).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert!(!state.hub_dir.join("holds.json.tmp").exists(), "no temp file is left behind");
    }

    #[test]
    fn a_corrupt_file_is_moved_aside_and_the_hub_starts() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        std::fs::write(state.hub_dir.join("holds.json"), b"{ not json").unwrap();
        let h = Holds::load(&state);
        assert!(h.snapshot().is_empty());
        let aside: Vec<_> = std::fs::read_dir(&state.hub_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("holds.json.corrupt-"))
            .collect();
        assert_eq!(aside.len(), 1, "the corrupt file is kept, not discarded");
        assert_eq!(std::fs::read(aside[0].path()).unwrap(), b"{ not json");
        // The registry works and writes a fresh, valid file.
        assert!(h.hold("io/alpha", None).persisted);
        assert!(Holds::load(&state).check("io/alpha").is_some());
    }

    #[test]
    fn an_unknown_version_is_treated_as_corrupt_not_silently_read() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        std::fs::write(state.hub_dir.join("holds.json"), br#"{"version":99,"holds":{}}"#).unwrap();
        let h = Holds::load(&state);
        assert!(h.snapshot().is_empty());
        assert!(!state.hub_dir.join("holds.json").exists(), "moved aside");
    }

    #[cfg(unix)]
    #[test]
    fn an_unwritable_directory_keeps_the_hold_in_memory_and_says_so() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        let h = Holds::load(&state);
        std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
        // Root ignores directory modes; skip the assertion there.
        let writable_anyway = std::fs::File::create(state.hub_dir.join("probe")).is_ok();
        let out = h.hold("io/alpha", Some("freeze"));
        std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(h.check("io/alpha").is_some(), "the hold is in force even though it could not be saved");
        if !writable_anyway {
            assert!(!out.persisted);
        }
        // Once the directory is writable again the next change persists everything.
        assert!(h.hold("io/beta", None).persisted);
        let reloaded = Holds::load(&state);
        assert!(reloaded.check("io/alpha").is_some() && reloaded.check("io/beta").is_some());
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_is_never_overwritten() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        let file = state.hub_dir.join("holds.json");
        std::fs::write(&file, br#"{"version":1,"holds":{"io/old":{"since":"2026-01-01T00:00:00Z"}}}"#).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&file).is_ok() {
            return; // running as a user that can read anything
        }
        let h = Holds::load(&state);
        assert!(h.snapshot().is_empty());
        assert!(!h.hold("io/alpha", None).persisted, "writing is blocked so the unreadable holds are not lost");
        assert!(h.check("io/alpha").is_some());
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(std::fs::read_to_string(&file).unwrap().contains("io/old"));
    }

    /// A `check` that runs after a `hold` has returned always sees it, and one
    /// that ran before it may not; racing threads never observe a hold that
    /// then disappears without a release. Repeated many times.
    #[test]
    fn concurrent_checks_and_holds_are_totally_ordered() {
        for round in 0..200 {
            let h = Holds::in_memory();
            let barrier = Arc::new(std::sync::Barrier::new(5));
            let held_at = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut checkers = Vec::new();
            for _ in 0..4 {
                let (h, b, flag) = (h.clone(), barrier.clone(), held_at.clone());
                checkers.push(std::thread::spawn(move || {
                    b.wait();
                    let mut seen_held = false;
                    for _ in 0..200 {
                        // Read the flag *before* the check: if `hold` had already
                        // returned, the check must see it.
                        let returned = flag.load(std::sync::atomic::Ordering::SeqCst);
                        let now = h.check("io/alpha").is_some();
                        assert!(!returned || now, "a check after hold() returned missed the hold (round {round})");
                        assert!(!seen_held || now, "a hold vanished without a release (round {round})");
                        seen_held |= now;
                    }
                }));
            }
            barrier.wait();
            h.hold("io/alpha", None);
            held_at.store(true, std::sync::atomic::Ordering::SeqCst);
            for c in checkers {
                c.join().unwrap();
            }
        }
    }

    /// Two writers holding and releasing the same session at once leave the
    /// file equal to the final in-memory state.
    #[test]
    fn racing_writers_leave_the_file_equal_to_memory() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(dir.path());
        for _ in 0..30 {
            let h = Holds::load(&state);
            let mut joins = Vec::new();
            for t in 0..4 {
                let h = h.clone();
                joins.push(std::thread::spawn(move || {
                    for i in 0..10 {
                        if (i + t) % 2 == 0 {
                            h.hold("io/alpha", Some("x"));
                        } else {
                            h.release("io/alpha");
                        }
                    }
                }));
            }
            for j in joins {
                j.join().unwrap();
            }
            assert_eq!(Holds::load(&state).snapshot(), h.snapshot());
        }
    }
}
