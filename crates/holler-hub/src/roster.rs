//! The hub's roster (issue #186): **who can be hollered at right now**, driven
//! by periodic presence.
//!
//! The roster is the hub-side authority for the "who's up" question. Each row
//! is one routable session name (`label/session`) a body advertises in a
//! `session/presence`; the row's `state` (the A2A name `idle`/`working`/
//! `input-required`) comes straight from the presence, and its `conn_state`
//! (the transport's `connected`/`reconnecting`/`gone`) is derived by the tri-
//! state TTL sweep from `last_seen` — the dropped-connections memo's thresholds
//! (45 s → reconnecting, 180 s → gone, 360 s → pruned), overridable by env for
//! tests. A successful *any* frame (a `session/update`, a ping, a response)
//! touches `last_seen` and flips `reconnecting` back to `connected` (the
//! holler-server#203 fix: liveness is traffic, not just the heartbeat).
//!
//! The collision policy (revised for holler-server#383) decides what happens
//! when two tokens advertise the same name: a name whose current holder is
//! `connected` rejects the newcomer (the name is held); a holder that has aged
//! into `reconnecting`/`gone` is displaced — the newcomer **replaces** the row
//! (name kept, locators swapped) and the displaced holder's later presence for
//! that name is rejected. Because presence repeats every 15 s, a rejected
//! claim retries itself the moment its holder ages out (the holler-server#243
//! fix, by construction). Same-token re-claims are always accepted.
//!
//! An explicit close / a `circuit/superseded` / a revocation marks the token's
//! rows `gone` **immediately** (the holler-server#80 rule) — no TTL wait.
//!
//! `stalled` is a *display* state only: a `working` row whose `last_update_at`
//! is older than the stall threshold (5 min by default) is shown as `stalled`
//! (in `roster` and `session_busy`), but `state` stays `working`. It is a hint
//! ("look"), never a trigger ("interrupt") — see the README's orchestrator
//! guidance.
//!
//! Rows carry the #142 wire vocabulary (`turn_id` / `last_turn`, the `LAST TURN`
//! column and the `holler wait --after` basis) and the #151 `pending` (the
//! `PENDING` column), both of which are already on the wire in `holler-proto`.
//!
//! This module is pure state: the connection loop (issue #182) calls
//! [`Roster::touch`]/[`Roster::advertise`]/[`Roster::clear`] on a shared
//! instance as frames arrive and circuits open/close, and the control socket
//! reads [`Roster::rows`] (via `control/roster`) and runs the TTL
//! [`Roster::sweep`] on its own interval. The sweep's clock is injectable
//! ([`Clock`]): production uses [`Clock::System`] (the real clock), while the
//! unit tests hand a roster a [`Clock::Manual`] they advance by exact seconds,
//! so the 45/180/360 s thresholds are deterministic (no 15-s real heartbeat, no
//! flaky wall-clock timing).
//!
//! The whole roster is guarded by one `std::sync::Mutex` and every entry point
//! is a **synchronous** function: a connection handler holds the mutex only for
//! the brief span of one presence apply, and the `name_held` warn it may emit
//! is a plain synchronous `eprintln!` (the `holler-proto` log is synchronous),
//! so there is no `await` while the guard is held. That keeps the live path
//! safe to call from the async connection tasks without a `tokio::sync::Mutex`
//! deadlock risk.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use std::sync::atomic::{AtomicU64, Ordering};

use holler_proto::docs::{LastTurn, PendingItem, SessionAd};
use holler_proto::Presence;
use tokio::sync::watch;

/// The TTL thresholds and the stall display threshold, all in whole seconds.
///
/// The defaults are the spec's; the test env overrides (`HOLLER_ROSTER_*_MS`)
/// are read here from [`Config::from_env`] (the production path), and the unit
/// tests construct a `Config` directly (with a short threshold) plus a
/// [`Clock::Manual`] to skip the wall clock entirely.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// `now - last_seen >= reconnect_secs` → the row is `reconnecting`.
    pub reconnect_secs: u64,
    /// `now - last_seen >= gone_secs` → the row is `gone`.
    pub gone_secs: u64,
    /// `now - last_seen >= prune_secs` → the row is removed (default 2×gone).
    pub prune_secs: u64,
    /// The sweep's period (how often the hub runs the sweep).
    pub sweep_ms: u64,
    /// `now - last_update_at >= stall_secs` on a `working` row → `stalled`.
    pub stall_secs: u64,
}

impl Config {
    /// The spec's default thresholds (the dropped-connections memo + the 5-min
    /// stall hint).
    pub const fn default() -> Self {
        Self {
            reconnect_secs: 45,
            gone_secs: 180,
            prune_secs: 360,
            sweep_ms: 30_000,
            stall_secs: 300,
        }
    }

    /// The production config: the spec defaults, but each threshold overridable
    /// by its `HOLLER_ROSTER_*_MS` / `HOLLER_STALL_MS` env var (milliseconds →
    /// whole seconds). An unparseable/absent var falls back to the default.
    pub fn from_env() -> Self {
        let mut c = Self::default();
        c.reconnect_secs = env_ms("HOLLER_ROSTER_RECONNECT_MS", 45_000) / 1000;
        c.gone_secs = env_ms("HOLLER_ROSTER_GONE_MS", 180_000) / 1000;
        c.prune_secs = env_ms("HOLLER_ROSTER_PRUNE_MS", 360_000) / 1000;
        c.sweep_ms = env_ms("HOLLER_ROSTER_SWEEP_MS", 30_000);
        c.stall_secs = env_ms("HOLLER_STALL_MS", 300_000) / 1000;
        c
    }
}

/// The time source the TTL sweep uses.
#[derive(Debug, Clone)]
pub enum Clock {
    /// The real system clock (the hub's production source).
    System,
    /// A test clock whose `now` is the current offset (seconds past a fixed
    /// origin). The unit tests advance it with [`Roster::advance`]. The offset
    /// lives in a shared `AtomicU64` (see [`Roster`]) so `Roster::advance` can
    /// be synchronous.
    Manual {
        /// Shared with the `Roster` that owns this clock (advanced by
        /// [`Roster::advance`]).
        offset: Arc<AtomicU64>,
    },
}

/// One roster row: one routable session name a body advertises.
///
/// `name` keys the row (a `RoutableName`'s string form, `label/session`). The
/// transport's liveness (`conn_state`) is separate from the A2A session state
/// (`state`) — a body can be `connected` but `idle`, or `gone` while still
/// `working` in the last presence.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Row {
    /// The routable name (`label/session`).
    pub name: String,
    /// The harness hosting this session.
    pub harness: String,
    /// How the session was established: `spawn` / `attach`.
    pub mode: String,
    /// The A2A session state (`idle` / `working` / `input-required`) — the
    /// *stored* value. [`Roster::rows`] displays `stalled` in its place when a
    /// `working` row's `last_update_at` has aged past the stall threshold.
    pub state: String,
    /// The transport's liveness: `connected` / `reconnecting` / `gone`.
    pub conn_state: &'static str,
    /// The token that currently holds the row.
    pub token_id: String,
    /// The body's client id.
    pub client_id: String,
    /// The body's hostname.
    pub hostname: String,
    /// The attached harness session id (attach mode only).
    pub harness_session_id: Option<String>,
    /// When the row was first seen (epoch seconds).
    pub first_seen: u64,
    /// When the token was last heard from (epoch seconds) — the TTL's input.
    pub last_seen: u64,
    /// When the in-flight turn last produced a `session/update` (epoch
    /// seconds), or `None` for a non-`working` row — the `stalled` input.
    pub last_update_at: Option<u64>,
    /// The held permission(s)/elicitation(s) (issue #151) — the `PENDING`
    /// column; `--json` carries the full array.
    pub pending: Option<Vec<PendingItem>>,
    /// The JSON-RPC id of the current / most recent `session/prompt` (issue
    /// #142) — the `holler wait --after` basis.
    pub turn_id: Option<String>,
    /// The most recently completed turn (issue #142) — the `LAST TURN` column.
    pub last_turn: Option<LastTurn>,
}

/// The token → client-id / hostname binding a roster needs to attribute rows.
#[derive(Debug, Clone, Default)]
struct TokenInfo {
    client_id: Option<String>,
    hostname: Option<String>,
}

/// The mutable roster state, guarded by a `std::sync::Mutex` (see the module
/// docs: no `await` is held across the guard, so the lock is safe in async
/// handlers). `gen` (the change generation) and `cfg` live here — **inside**
/// the guard — so a method that already holds the guard (e.g. `apply_ad`
/// called from within `advertise`) can read them without re-locking the
/// non-re-entrant `std::sync::Mutex` and self-deadlocking.
struct Inner {
    rows: HashMap<String, Row>,
    tokens: HashMap<String, TokenInfo>,
    change_tx: watch::Sender<u64>,
    /// The change-channel generation counter (bumped on any roster change) so
    /// a `wait`-ing CLI (issue #142) notices without polling.
    gen: Arc<AtomicU64>,
    /// The clock, stored inside the guard so the in-lock helpers (`apply_ad`,
    /// `drop_absent`) read `now` from the *roster's* clock — the manual clock
    /// in the unit tests — and never the system clock.
    clock: Clock,
}

impl Inner {
    /// Build the mutable state. `gen` is shared (the `Roster` keeps an `Arc`
    /// to the same counter for [`Roster::subscribe`]'s bookkeeping), and
    /// `clock` is stored so the in-lock helpers can read `now` without
    /// re-locking.
    fn new(gen: Arc<AtomicU64>, clock: Clock) -> Self {
        Self {
            rows: HashMap::new(),
            tokens: HashMap::new(),
            change_tx: watch::Sender::default(),
            gen,
            clock,
        }
    }

    /// The current `now` from this guard's clock (the roster's clock: system
    /// in production, the manual offset in the unit tests). Reading it from the
    /// guard (not a `&self` method on `Roster`) means the in-lock helpers never
    /// re-lock the non-re-entrant mutex.
    fn now_secs(&self) -> u64 {
        match &self.clock {
            Clock::System => crate::token::now_secs(),
            Clock::Manual { offset } => offset.load(Ordering::SeqCst),
        }
    }

    fn change_rx(&self) -> watch::Receiver<u64> {
        self.change_tx.subscribe()
    }
}

/// The hub-wide roster (issue #186), shared across the WS connection tasks
/// (which touch/advertise/clear on frame arrival) and the control socket (which
/// reads it and runs the TTL sweep). Cheap to clone (everything shared is
/// behind an `Arc`).
#[derive(Clone)]
pub struct Roster {
    inner: Arc<StdMutex<Inner>>,
    cfg: Config,
    clock: Clock,
    /// The manual-clock offset, shared with a [`Clock::Manual`]. (The
    /// `Clock` enum carries the same `Arc`; this is the canonical handle
    /// [`Roster::advance`] bumps and [`Roster::now_secs`] reads.)
    manual_offset: Arc<AtomicU64>,
}

impl std::fmt::Debug for Roster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Roster")
            .field("config", &self.cfg)
            .field("clock", &self.clock)
            .finish()
    }
}

impl Roster {
    /// Build a roster with the given config and an **injected** clock (a
    /// [`Clock::Manual`] the caller drives with [`Roster::advance`]). This is
    /// the unit-test path: the roster's `now` is the manual offset (0 at
    /// construction), so the 45/180/360 s thresholds are crossed
    /// deterministically without a real 15-s heartbeat or a flaky wall clock.
    /// The row's clock anchors (`first_seen`/`last_seen`/`last_update_at`) come
    /// from this same clock, so a row is "fresh" until the test advances the
    /// clock past the relevant threshold.
    ///
    /// The production hub uses [`Roster::with_system_clock`] instead, so this
    /// constructor never touches the wall clock.
    pub fn new(cfg: &Config) -> Self {
        let offset = Arc::new(AtomicU64::new(0));
        let gen = Arc::new(AtomicU64::new(0));
        let clock = Clock::Manual {
            offset: offset.clone(),
        };
        Self {
            inner: Arc::new(StdMutex::new(Inner::new(gen, clock.clone()))),
            cfg: *cfg,
            clock,
            manual_offset: offset,
        }
    }

    /// Build a roster with the **real system clock** (the production path).
    /// Construction is synchronous; the hub then hands the shared `Roster` to
    /// the connection and control tasks, and [`Roster::advance`] is a no-op (a
    /// system-clock roster's `now` is always the wall clock).
    pub fn with_system_clock(cfg: &Config) -> Self {
        let gen = Arc::new(AtomicU64::new(0));
        let clock = Clock::System;
        Self {
            inner: Arc::new(StdMutex::new(Inner::new(gen, clock.clone()))),
            cfg: *cfg,
            clock,
            manual_offset: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Advance the injected clock by `secs`. A no-op for a
    /// [`Roster::with_system_clock`] roster (production); a [`Roster::new`]
    /// roster advances its offset by exactly `secs`, so the TTL thresholds are
    /// crossed deterministically.
    pub fn advance(&self, secs: u64) {
        if matches!(self.clock, Clock::Manual { .. }) {
            self.manual_offset.fetch_add(secs, Ordering::SeqCst);
        }
    }

    /// Bind a token to a client id (the token store knows this once a body has
    /// joined; the roster learns it when a live circuit authenticates).
    /// Idempotent.
    pub fn set_token(&self, token_id: &str, client_id: &str) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).bind_token(token_id, Some(client_id), None);
    }

    /// Record a hostname for a token (from the hello exchange).
    pub fn set_hostname(&self, token_id: &str, hostname: &str) {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).bind_token(token_id, None, Some(hostname));
    }

    /// A `session/presence` from a body: replace (not merge) that token's set
    /// of rows. A row in the frame is upserted (`connected`, `last_seen` =
    /// now); a row the token previously held but which is absent from the frame
    /// goes `gone` **immediately**. Returns the names **rejected** by the
    /// collision policy (a connected different-token holder blocks them) — the
    /// rest of the frame was applied.
    pub fn advertise(&self, token_id: &str, p: &Presence) -> Vec<String> {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.bind_token(token_id, None, Some(&p.hostname));
        let info = inner.tokens.get(token_id).cloned().unwrap_or_default();
        let incoming: HashSet<String> = p.sessions.iter().map(|s| s.name.clone()).collect();
        // The newest ad timestamp in this frame (the `last_update_at` base):
        // the body emits one presence per body, so its sessions share one
        // clock and this is the frame's "now" on the body's wall.
        let base = p
            .sessions
            .iter()
            .filter_map(|s| parse_rfc3339_secs(s.last_update_at.as_deref()))
            .max();
        let mut rejected = Vec::new();
        let mut changed = 0usize;
        for s in &p.sessions {
            if let Some(name) = inner.apply_ad(token_id, &info, s, base) {
                rejected.push(name);
            } else {
                changed += 1;
            }
        }
        // Rows this token held but which the frame dropped go `gone` now.
        changed += inner.drop_absent(token_id, &incoming, base);
        if changed > 0 {
            inner.bump_gen();
        }
        rejected
    }

    /// *Any* frame from a body (a presence, an update, a ping, a response)
    /// touches `last_seen` on all of that token's rows and flips a
    /// `reconnecting` row back to `connected` (holler-server#203). A touch for
    /// an unknown token is a silent no-op (never a panic); a touch never
    /// refreshes another token's rows.
    pub fn touch(&self, token_id: &str, _method: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = inner.now_secs();
        let mut touched = 0usize;
        for row in inner.rows.values_mut() {
            if row.token_id != token_id {
                continue; // never refresh another token's row.
            }
            row.last_seen = now;
            if row.conn_state != "connected" {
                row.conn_state = "connected";
            }
            touched += 1;
        }
        if touched > 0 {
            inner.bump_gen();
        }
    }

    /// An explicit close / `circuit/superseded` / a revocation: every row of
    /// that token goes `gone` **immediately** (holler-server#80) — no TTL wait.
    /// `gone` rows are hidden from the default listing but linger until the TTL
    /// prunes them.
    pub fn clear(&self, token_id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = inner.now_secs();
        let mut cleared = 0usize;
        for row in inner.rows.values_mut() {
            if row.token_id == token_id {
                row.conn_state = "gone";
                row.last_seen = now;
                cleared += 1;
            }
        }
        if cleared > 0 {
            inner.bump_gen();
        }
    }

    /// Run the tri-state TTL sweep at `now` (the injected clock): a row
    /// `≥ reconnect` old is `reconnecting`, `≥ gone` old is `gone`, `≥ prune`
    /// old is removed. Idempotent per direction (a `gone` row is not re-
    /// reported as a change just because it is still gone). Bumps the
    /// change-channel generation when anything moved, so a `wait`-ing CLI
    /// notices.
    pub fn sweep(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = inner.now_secs();
        let mut changed = 0usize;
        let mut pruned = Vec::new();
        for (name, row) in inner.rows.iter_mut() {
            let age = now.saturating_sub(row.last_seen);
            match row.conn_state {
                "connected" if age >= self.cfg.reconnect_secs => {
                    row.conn_state = "reconnecting";
                    changed += 1;
                }
                "connected" | "reconnecting" if age >= self.cfg.gone_secs => {
                    row.conn_state = "gone";
                    changed += 1;
                }
                "gone" if age >= self.cfg.prune_secs => pruned.push(name.clone()),
                _ => {}
            }
        }
        for name in pruned {
            inner.rows.remove(&name);
            changed += 1;
        }
        if changed > 0 {
            inner.bump_gen();
        }
    }

    /// The rows, sorted by name. `all` of `None` excludes `gone` rows (the
    /// default roster listing); `Some(true)` includes them. Each returned row
    /// has `stalled` applied to its `state` display where applicable.
    pub fn rows(&self, all: Option<bool>) -> Vec<Row> {
        let (rows, now) = {
            let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let rows: Vec<Row> = inner.rows.values().cloned().collect();
            (rows, inner.now_secs())
        };
        let mut out = Vec::new();
        for row in rows {
            if row.conn_state == "gone" && !all.unwrap_or(false) {
                continue;
            }
            out.push(self.stalled_display(&row, now));
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// The change channel's receiver (issue #142's `holler wait` selects on
    /// this without polling).
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).change_rx()
    }

    /// The sweep's period in milliseconds (the hub's sweep task sleeps this
    /// long between [`Roster::sweep`] calls).
    pub fn sweep_ms(&self) -> u64 {
        self.cfg.sweep_ms
    }

    // -- internals -----------------------------------------------------------

    /// The row as it should be *displayed*: a `working` row whose
    /// `last_update_at` has aged past the stall threshold shows as `stalled`
    /// (a `connected` row only — a `gone`/`reconnecting` row's `last_update_at`
    /// says nothing about the harness). The stored row keeps `working`.
    fn stalled_display(&self, row: &Row, now: u64) -> Row {
        if row.state == "working" && row.conn_state == "connected" {
            if let Some(ts) = row.last_update_at {
                if now.saturating_sub(ts) >= self.cfg.stall_secs {
                    let mut d = row.clone();
                    d.state = "stalled".to_string();
                    return d;
                }
            }
        }
        row.clone()
    }
}

impl Inner {
    /// Bump the published generation (a roster change a `wait`-er notices).
    /// Called only while the roster's guard is already held (never re-locks, so
    /// it cannot self-deadlock the non-re-entrant `std::sync::Mutex`).
    fn bump_gen(&mut self) {
        let next = self.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.change_tx.send_replace(next);
    }

    /// Apply one advertised session (upsert / re-claim / collision) on the
    /// held state. Returns `Some(name)` when the collision policy rejected it.
    /// Called only while the roster's guard is already held (the helpers it
    /// calls — [`Inner::upsert_row`], [`Inner::bump_gen`], and the `Roster`
    /// row-builder / `name_held` warn — never re-lock, so there is no
    /// self-deadlock).
    fn apply_ad(&mut self, token_id: &str, info: &TokenInfo, s: &SessionAd, base: Option<u64>) -> Option<String> {
        let now = self.now_secs();
        if let Some(existing) = self.rows.get(&s.name) {
            if existing.token_id == token_id {
                // Same token: a re-claim is always accepted (a locator refresh).
                self.upsert_row(token_id, info, s, existing.first_seen, now, base);
                return None;
            }
            // A different token: held iff the current holder is `connected`;
            // a `reconnecting`/`gone` holder is displaced (the #383 rule).
            if existing.conn_state == "connected" {
                self.log_name_held(&s.name, existing);
                return Some(s.name.clone());
            }
            self.upsert_row(token_id, info, s, existing.first_seen, now, base);
            return None;
        }
        // A brand-new name.
        self.rows
            .insert(s.name.clone(), self.build_row(token_id, info, s, now, now, base));
        None
    }

    /// Replace an existing row with the new advertisement (upsert / re-claim /
    /// displacement). `first_seen` is preserved across a re-claim.
    fn upsert_row(
        &mut self,
        token_id: &str,
        info: &TokenInfo,
        s: &SessionAd,
        first_seen: u64,
        now: u64,
        base: Option<u64>,
    ) {
        self.rows
            .insert(s.name.clone(), self.build_row(token_id, info, s, first_seen, now, base));
    }

    /// Mark the rows this token previously held but the frame dropped as
    /// `gone` immediately. Returns the count changed. Their `last_update_at` is
    /// re-anchored on the *dropping* frame's clock (not the stale base the row
    /// was built with) so a dropped `working` row does not instantly read as
    /// `stalled` on display.
    fn drop_absent(&mut self, token_id: &str, incoming: &HashSet<String>, _base: Option<u64>) -> usize {
        let now = self.now_secs();
        let mut gone = 0usize;
        for (name, row) in self.rows.iter_mut() {
            if row.token_id == token_id && !incoming.contains(name) {
                row.conn_state = "gone";
                row.last_seen = now;
                // Re-anchor `last_update_at` on this frame's clock so a dropped
                // `working` row does not read as instantly `stalled` on display.
                if row.state == "working" {
                    row.last_update_at = Some(now);
                }
                gone += 1;
            }
        }
        gone
    }

    /// Build a fresh row from an advertisement + the token's binding. The row's
    /// clock anchors (`first_seen`, `last_seen`, and the `last_update_at` base)
    /// come from the roster's clock so they are mutually consistent (and
    /// correct under an injected test clock); the ad's `last_update_at` only
    /// supplies the *offset* within the presence batch (see
    /// [`Inner::last_update_secs`]).
    fn build_row(
        &self,
        token_id: &str,
        info: &TokenInfo,
        s: &SessionAd,
        first_seen: u64,
        now: u64,
        base: Option<u64>,
    ) -> Row {
        Row {
            name: s.name.clone(),
            harness: s.harness.clone(),
            mode: s.mode.as_str().to_string(),
            state: s.state.as_str().to_string(),
            conn_state: "connected",
            token_id: token_id.to_string(),
            client_id: info.client_id.clone().unwrap_or_default(),
            hostname: info.hostname.clone().unwrap_or_default(),
            harness_session_id: s.harness_session_id.clone(),
            first_seen,
            last_seen: now,
            last_update_at: self.last_update_secs(now, base, s),
            pending: s.pending.clone(),
            turn_id: s.turn_id.clone(),
            last_turn: s.last_turn.clone(),
        }
    }

    /// The row's `last_update_at` on the roster's clock. The roster anchors on
    /// `now` (its own clock, not the body's wall clock — which a test injects
    /// differently); the body's `last_update_at` supplies only the offset
    /// `base - t` (how far back in this presence batch the turn last updated),
    /// where `base` is the newest ad timestamp in the frame (falling back to
    /// `now` when the ad — or the whole frame — carries no timestamp). A
    /// timestamp newer than `base` (a skew) clamps to `now`; an absent offset
    /// (an idle row) is `now` (fresh), never `None` (which would show
    /// `stalled`). This is exact under the real clock and deterministic under
    /// an injected one.
    fn last_update_secs(&self, now: u64, base: Option<u64>, s: &SessionAd) -> Option<u64> {
        let Some(ts) = parse_rfc3339_secs(s.last_update_at.as_deref()) else {
            return Some(now);
        };
        let anchor = base.unwrap_or(now);
        Some(now.saturating_sub(anchor.saturating_sub(ts)))
    }

    /// Log the spec's `roster name_held` warn (a connected holder blocked a
    /// different token's claim). Synchronous (the `holler-proto` log is a plain
    /// `eprintln!`), so it is safe to call while holding the roster mutex.
    fn log_name_held(&self, name: &str, holder: &Row) {
        holler_proto::log::emit(&holler_proto::log::Event {
            component: holler_proto::log::Component::Roster,
            severity: holler_proto::log::Severity::Warn,
            direction: holler_proto::log::Direction::Local,
            method: "roster",
            id: None,
            peer: None,
            fields: vec![
                ("name", name.to_string()),
                ("holder", holder.token_id.clone()),
            ],
            frame: None,
        });
    }

    /// Merge a client-id / hostname binding for a token (fill only the fields
    /// given; never clobber a known value with `None`).
    fn bind_token(&mut self, token_id: &str, client_id: Option<&str>, hostname: Option<&str>) {
        let info = self.tokens.entry(token_id.to_string()).or_default();
        if let Some(c) = client_id {
            info.client_id = Some(c.to_string());
        }
        if let Some(h) = hostname {
            info.hostname = Some(h.to_string());
        }
    }
}

/// Parse an RFC3339 timestamp into whole epoch seconds, or `None` (an absent /
/// malformed timestamp is treated as "no update signal" — it never stalls a
/// row on its own). The body emits these from `SystemTime` (UTC, so the offset
/// is `Z`/`+00:00`); the parser accepts that profile and is lenient about a
/// fractional-second suffix. A non-UTC offset would be mis-parsed, but that
/// never happens on this wire — and even then the failure is only a missed
/// *display* hint (the row keeps showing `working`), never a correctness bug.
fn parse_rfc3339_secs(ts: Option<&str>) -> Option<u64> {
    ts.and_then(parse_rfc3339)
}

/// Parse a single RFC3339 `YYYY-MM-DDTHH:MM:SS(.fff)?Z` timestamp to epoch
/// seconds. Returns `None` on any shape the body never produces.
fn parse_rfc3339(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    // YYYY-MM-DD
    let year: i64 = s[0..4].parse().ok()?;
    if bytes[4] != b'-' {
        return None;
    }
    let month: i64 = s[5..7].parse().ok()?;
    if bytes[7] != b'-' {
        return None;
    }
    let day: i64 = s[8..10].parse().ok()?;
    // 'T' separator
    if !matches!(bytes[10], b'T' | b't' | b' ') {
        return None;
    }
    let hour: i64 = s[11..13].parse().ok()?;
    if bytes[13] != b':' {
        return None;
    }
    let minute: i64 = s[14..16].parse().ok()?;
    if bytes[16] != b':' {
        return None;
    }
    let second: i64 = s[17..19].parse().ok()?;
    // Optional fractional seconds: consume `.fff…` if present.
    let mut rest = 19usize;
    if bytes.get(rest) == Some(&b'.') {
        rest += 1;
        while rest < bytes.len() && bytes[rest].is_ascii_digit() {
            rest += 1;
        }
    }
    // Zone: accept a `Z` (or lowercase `z`) or a `+00:00` / `-00:00` offset.
    if matches!(bytes.get(rest), Some(&b'Z') | Some(&b'z')) {
        // (Trailing bytes after the zone, if any, are ignored — the body emits
        // exactly `...Z`.)
    } else if matches!(bytes.get(rest), Some(&b'+') | Some(&b'-')) {
        // A non-Z offset; only treat it as epoch-correct when it is UTC.
        if rest + 5 >= bytes.len() {
            return None;
        }
        let off = s.get(rest + 1..rest + 5)?;
        if off != "00:00" {
            return None;
        }
    } else {
        return None;
    }
    // The body emits post-1970 UTC timestamps, so the value is non-negative;
    // convert the `i64` day/time math to `u64` (rejecting any pre-epoch input).
    let secs = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    u64::try_from(secs).ok()
}

/// Howard Hinnant's `days_from_civil` — whole days since 1970-01-01 for a
/// proleptic Gregorian date. (The standard 6-line algorithm.)
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 {
        year - 1
    } else {
        year
    };
    let era = if y >= 0 {
        y
    } else {
        y - 399
    } / 400;
    let yoe = y - era * 400;
    let m = if month > 2 {
        month - 3
    } else {
        month + 9
    };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Read a millisecond env var (`HOLLER_*_MS`); an absent/empty/unparseable var
/// falls back to `default_ms`.
fn env_ms(key: &str, default_ms: u64) -> u64 {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.trim().parse().unwrap_or(default_ms),
        _ => default_ms,
    }
}
