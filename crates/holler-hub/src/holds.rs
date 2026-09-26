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
//! Holds live in `<state dir>/hub/holds.json`, written atomically at `0600`
//! through the hub's one state-file writer,
//! `holler_proto::atomic_file::write_atomic` (a temp file in the same
//! directory, then `rename`; #483). The hub never fails to start over it:
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

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

mod grants;
use grants::Grants;
pub use grants::{GrantError, MintError, DEFAULT_TTL, MAX_TTL};

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
    /// Which hold this call lifted (`operator` or `default`), if any. A release
    /// lifts the top hold only: the operator hold when there is one, else the
    /// default hold.
    pub lifted: Option<Kind>,
    /// `true` when the session is still held after this call (a default hold
    /// was under the operator hold that was just lifted).
    pub still_held: bool,
    /// Whether the change was written to disk.
    pub persisted: bool,
}

/// Which layer of hold is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Set by `holler hold` (issue #437).
    Operator,
    /// Applied when a session joins (`hub serve --join-held`, issue #460).
    Default,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Operator => "operator",
            Kind::Default => "default",
        }
    }
}

/// The reason recorded on a default hold.
pub const JOIN_REASON: &str = "held on join";

/// The result of [`Holds::mint_grant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantOutcome {
    pub grant: String,
    pub ttl: Duration,
    /// `true` when the session carries a default hold, i.e. the grant does
    /// something. A grant for a session with no default hold is still valid but
    /// unnecessary.
    pub default_held: bool,
    /// `true` when an operator hold is also set: a grant cannot lift that, so
    /// the prompt it is meant for will still be refused.
    pub operator_held: bool,
}

/// The state file. `holds` is the operator layer (its original shape);
/// `default_holds` (issue #460) is additive.
#[derive(Debug, Default, Serialize, Deserialize)]
struct FileDoc {
    version: u32,
    holds: BTreeMap<String, HoldInfo>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    default_holds: BTreeMap<String, HoldInfo>,
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

/// Everything the map lock guards.
#[derive(Default)]
struct State {
    operator: BTreeMap<String, HoldInfo>,
    default: BTreeMap<String, HoldInfo>,
    /// Keys whose default hold was released in this hub process: a release is
    /// not undone by the next presence. Bounded by the operator's own releases;
    /// nothing is recorded for the sessions that merely join. Empty again after
    /// a restart, when `--join-held` applies to them afresh.
    released: HashSet<String>,
    grants: Grants,
}

struct Shared {
    state: Mutex<State>,
    persist: Mutex<Persist>,
    /// `--join-held` patterns (empty: sessions do not join held).
    join_held: Mutex<Vec<String>>,
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

/// `*` (any run of characters, `/` included) and `?` (any one character)
/// against the whole of `text`. The only wildcards `--join-held` supports.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pattern.chars().collect(), text.chars().collect());
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
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

/// Which hold refuses `key`, and why it does (or the grant that was not
/// honoured). `consume` decides whether a valid grant is used up.
fn decide(state: &mut State, key: &str, grant: Option<&str>, consume: bool) -> Result<(), holler_proto::WireError> {
    // An operator hold beats a grant: a valid grant is neither honoured nor consumed.
    if let Some(op) = state.operator.get(key) {
        return Err(op.refusal().with_hold_kind(Kind::Operator.as_str()));
    }
    if let Some(id) = grant {
        let now = Instant::now();
        let verdict = if consume { state.grants.consume(id, key, now) } else { state.grants.check(id, key, now) };
        return verdict.map_err(|e| holler_proto::WireError::invalid_grant(e.reason()));
    }
    if let Some(def) = state.default.get(key) {
        return Err(def.refusal().with_hold_kind(Kind::Default.as_str()));
    }
    Ok(())
}

impl Holds {
    /// A registry that never touches disk.
    pub fn in_memory() -> Self {
        Self::with_persist(State::default(), Persist { path: None, blocked: false, unsaved: false })
    }

    fn with_persist(state: State, persist: Persist) -> Self {
        Self {
            shared: Arc::new(Shared { state: Mutex::new(state), persist: Mutex::new(persist), join_held: Mutex::new(Vec::new()) }),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.shared.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Load the registry from `<state dir>/hub/holds.json`. Never fails: see
    /// the module docs for what a missing, corrupt or unreadable file means.
    pub fn load(state: &HubState) -> Self {
        let path = state.hub_dir.join("holds.json");
        let (doc, blocked) = read_file(&path);
        let st = State { operator: doc.holds, default: doc.default_holds, ..State::default() };
        Self::with_persist(st, Persist { path: Some(path), blocked, unsaved: blocked })
    }

    /// Make sessions whose `<label>/<session>` name matches any of `patterns`
    /// join held (issue #460). Call once, at startup; an empty list is the
    /// default (sessions join open).
    pub fn set_join_held(&self, patterns: Vec<String>) {
        *self.shared.join_held.lock().unwrap_or_else(PoisonError::into_inner) = patterns;
    }

    /// The hold that is refusing `key` (operator first), if any.
    pub fn check(&self, key: &str) -> Option<(HoldInfo, Kind)> {
        let st = self.state();
        match (st.operator.get(key), st.default.get(key)) {
            (Some(op), _) => Some((op.clone(), Kind::Operator)),
            (None, Some(def)) => Some((def.clone(), Kind::Default)),
            (None, None) => None,
        }
    }

    /// Every key with any hold (sorted).
    pub fn keys(&self) -> Vec<String> {
        let st = self.state();
        let mut keys: Vec<String> = st.operator.keys().chain(st.default.keys()).cloned().collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// The operator holds (the state file's original layer).
    pub fn snapshot(&self) -> BTreeMap<String, HoldInfo> {
        self.state().operator.clone()
    }

    /// The roster fields for `key`.
    pub fn row_hold(&self, key: &str) -> holler_proto::SessionHold {
        let st = self.state();
        match (st.operator.get(key), st.default.get(key)) {
            (Some(op), def) => {
                let mut h = holler_proto::SessionHold::held(op.reason.clone(), op.since.clone()).of_kind(Kind::Operator.as_str());
                h.hold_default = def.is_some();
                h
            }
            (None, Some(def)) => holler_proto::SessionHold::held(def.reason.clone(), def.since.clone()).of_kind(Kind::Default.as_str()),
            (None, None) => holler_proto::SessionHold::default(),
        }
    }

    /// **The enforcement decision** (issue #442, #460), called from
    /// `circuit::dispatch::send_prompt` and nowhere else that delivers: `Ok` to
    /// deliver the prompt, or the refusal to send instead. A valid `grant` for
    /// this session is consumed here, in the same critical section that
    /// decides, so of two racing senders only one can be admitted with it.
    pub fn admit(&self, key: &str, grant: Option<&str>) -> Result<(), holler_proto::WireError> {
        decide(&mut self.state(), key, grant, true)
    }

    /// What [`Holds::admit`] would answer right now, without consuming
    /// anything: the fast path that keeps a refused prompt from leaving
    /// side effects behind. Never the authority (a grant can be consumed
    /// between this and `admit`).
    pub fn would_refuse(&self, key: &str, grant: Option<&str>) -> Option<holler_proto::WireError> {
        decide(&mut self.state(), key, grant, false).err()
    }

    /// Hold `key` (the operator layer). Repeating keeps the original reason and
    /// since-time.
    pub fn hold(&self, key: &str, reason: Option<&str>) -> HoldOutcome {
        let (info, newly_held) = {
            let mut st = self.state();
            match st.operator.get(key) {
                Some(existing) => (existing.clone(), false),
                None => {
                    let info = HoldInfo { reason: reason.and_then(sanitize_reason), since: log::timestamp() };
                    st.operator.insert(key.to_owned(), info.clone());
                    (info, true)
                }
            }
        };
        let persisted = self.sync(newly_held);
        if newly_held {
            emit(Severity::Info, "session_held", vec![("session", key.to_owned()), ("kind", "operator".to_owned())]);
        }
        HoldOutcome { info, newly_held, persisted }
    }

    /// Release `key`: lift the operator hold if there is one, else the default
    /// hold. Releasing a session that is not held is a no-op.
    pub fn release(&self, key: &str) -> ReleaseOutcome {
        let (lifted, still_held) = {
            let mut st = self.state();
            if st.operator.remove(key).is_some() {
                (Some(Kind::Operator), st.default.contains_key(key))
            } else if st.default.remove(key).is_some() {
                st.released.insert(key.to_owned());
                (Some(Kind::Default), false)
            } else {
                (None, false)
            }
        };
        let persisted = self.sync(lifted.is_some());
        if let Some(kind) = lifted {
            emit(Severity::Info, "session_released", vec![("session", key.to_owned()), ("kind", kind.as_str().to_owned())]);
        }
        ReleaseOutcome { was_held: lifted.is_some(), lifted, still_held, persisted }
    }

    /// Mint a one-time grant for `key` valid for `ttl` (issue #460).
    pub fn mint_grant(&self, key: &str, ttl: Duration) -> Result<GrantOutcome, MintError> {
        let mut st = self.state();
        let grant = st.grants.mint(key, ttl, Instant::now())?;
        let out = GrantOutcome {
            grant,
            ttl: ttl.min(grants::MAX_TTL),
            default_held: st.default.contains_key(key),
            operator_held: st.operator.contains_key(key),
        };
        drop(st);
        emit(Severity::Info, "grant_minted", vec![("session", key.to_owned()), ("ttl_ms", out.ttl.as_millis().to_string())]);
        Ok(out)
    }

    /// Live grants for `key` (for tests and status).
    pub fn live_grants(&self, key: &str) -> usize {
        self.state().grants.live_for(key, Instant::now())
    }

    /// A body advertised these sessions (issue #460). A session *joins* when it
    /// matches a `--join-held` pattern and has no default hold, unless one was
    /// released in this process: it then gets a default hold (reason
    /// [`JOIN_REASON`]). Nothing is remembered about sessions that do not
    /// match, so memory grows only with the holds themselves. Returns how many
    /// were held. Cheap when `--join-held` is off: no lock is taken.
    pub fn note_joined(&self, keys: impl IntoIterator<Item = String>) -> usize {
        let patterns = self.shared.join_held.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if patterns.is_empty() {
            return 0;
        }
        let mut newly = Vec::new();
        {
            let mut st = self.state();
            for key in keys {
                if st.default.contains_key(&key) || st.released.contains(&key) {
                    continue;
                }
                if patterns.iter().any(|pat| glob_match(pat, &key)) {
                    st.default.insert(key.clone(), HoldInfo { reason: Some(JOIN_REASON.to_owned()), since: log::timestamp() });
                    newly.push(key);
                }
            }
        }
        if !newly.is_empty() {
            self.sync(true);
            for key in &newly {
                emit(Severity::Info, "session_held", vec![("session", key.clone()), ("kind", "default".to_owned())]);
            }
        }
        newly.len()
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

    /// Write the current holds. Serialised by the persistence lock, and the
    /// snapshot is taken *after* acquiring it, so whichever writer runs last
    /// writes the newest state no matter how the writers interleaved.
    fn persist(&self) -> bool {
        let mut persist = self.shared.persist.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(path) = persist.path.clone() else { return true };
        if persist.blocked {
            persist.unsaved = true;
            return false;
        }
        let doc = {
            let st = self.state();
            FileDoc { version: FILE_VERSION, holds: st.operator.clone(), default_holds: st.default.clone() }
        };
        let body = serde_json::to_vec_pretty(&doc).map_err(std::io::Error::other);
        match body.and_then(|body| holler_proto::atomic_file::write_atomic(&path, &body, 0o600)) {
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
fn read_file(path: &std::path::Path) -> (FileDoc, bool) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (FileDoc::default(), false),
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
            return (FileDoc::default(), true);
        }
    };
    match serde_json::from_slice::<FileDoc>(&bytes) {
        Ok(doc) if doc.version == FILE_VERSION => (doc, false),
        Ok(doc) => set_aside(path, &format!("unknown hold state version {}", doc.version)),
        Err(e) => set_aside(path, &format!("not valid hold state: {e}")),
    }
}

/// Move an unusable file out of the way (never overwriting an earlier one).
fn set_aside(path: &std::path::Path, why: &str) -> (FileDoc, bool) {
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
            (FileDoc::default(), false)
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
            (FileDoc::default(), true)
        }
    }
}

#[cfg(test)]
mod tests;
