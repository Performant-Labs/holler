//! A fleet of **real `holler body run` processes**, each hosting M spawn-mode
//! `stub-acp` sessions, plus the request-rate driver that talks to them
//! (issue #369 harness bullets 2 and 3).
//!
//! This is the other half of `wire.rs`. Where a wire client measures the
//! connection itself at a scale processes cannot reach, a fleet member is the
//! whole real client path — `holler body run` → the ACP driver → a `stub-acp`
//! child over stdio — so `say`/`interrupt`/`roster` traffic driven against it
//! exercises everything except the model, which is exactly what #369 asks for
//! ("deterministic, not model-latency-bound").
//!
//! The driver's rate control is open-loop on purpose: it launches each call at
//! its scheduled instant and records how long the call itself took, rather
//! than waiting for the previous call to finish. A closed-loop driver measures
//! its own back-pressure, not the hub's latency.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::hub::{own_process_group, Hub, StateDir};
use crate::metrics::Samples;
use crate::{kill_tree, Res};

/// One `component=session` `queue_enqueue`/`queue_dequeue`/`queue_full`
/// debug event, read off a [`FleetMember::start_watched`] body's stderr —
/// scenario 3's (#372) real-time queue-depth signal.
#[derive(Debug, Clone)]
pub struct QueueEvent {
    /// Which body emitted it (`FleetMember::start_watched`'s own `label`).
    pub label: String,
    /// The session name within that body (`task.rs`'s `SessionPresence`
    /// name — bare, not `label/name`).
    pub session: String,
    pub kind: QueueEventKind,
    /// The queue's own length, as reported by the body itself, immediately
    /// after this event (0 is a legal, meaningful depth: the queue just
    /// drained to empty).
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueEventKind {
    /// A `--queue` prompt was appended to a busy session's FIFO.
    Enqueue,
    /// The next queued prompt was popped off the front to run.
    Dequeue,
    /// A `--queue` prompt was refused because the FIFO was already at its
    /// hard cap (`QUEUE_CAP = 64`, `holler_body::session_manager`) — this
    /// does *not* change the real depth, so it is reported separately
    /// rather than folded into the depth series.
    Full,
}

/// Read `stderr`'s JSON-formatted lines, forward every queue-depth event to
/// `events`, and drop anything else (frame-shape debug lines, the startup
/// banner, `mailbox_enqueue`/`mailbox_dequeue`, warnings). Runs until the
/// pipe closes (the body process exits or is killed) — the same
/// drain-to-EOF shape `Hub::start`'s own stderr reader uses, so a body that
/// logs a lot never blocks on a full pipe.
fn relay_queue_events(stderr: std::process::ChildStderr, label: &str, events: &Sender<QueueEvent>) {
    let mut reader = std::io::BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        if v.get("component").and_then(serde_json::Value::as_str) != Some("session") {
            continue;
        }
        let kind = match v.get("type").and_then(serde_json::Value::as_str) {
            Some("queue_enqueue") => QueueEventKind::Enqueue,
            Some("queue_dequeue") => QueueEventKind::Dequeue,
            Some("queue_full") => QueueEventKind::Full,
            _ => continue,
        };
        let Some(session) = v.get("name").and_then(serde_json::Value::as_str) else { continue };
        let Some(depth) = v
            .get("depth")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        // A send error means the receiver (the scenario's aggregator) has
        // already hung up — nothing left to do but stop relaying.
        if events
            .send(QueueEvent { label: label.to_string(), session: session.to_string(), kind, depth })
            .is_err()
        {
            return;
        }
    }
}

/// One real body process and the sessions it hosts.
pub struct FleetMember {
    /// The routable session names (`<label>/<session>`) this body advertises.
    pub session_names: Vec<String>,
    child: Child,
    state: StateDir,
    holler_bin: PathBuf,
}

impl FleetMember {
    /// Provision, join and start one body: its own state dir (so its own
    /// on-disk identity), its own token, its own `sessions.toml` pointing every
    /// session at the real `stub-acp` binary.
    pub fn start(hub: &Hub, holler_bin: &Path, stub_acp: &Path, label: &str, sessions: usize) -> Res<Self> {
        // Mint through the hub's own store, then let the real `body join`
        // redeem it — the body must generate and persist *its own* keypair, so
        // this half cannot be short-circuited in-process the way `wire.rs`'s
        // clients can.
        let hub_state = holler_hub::state::HubState::from_root(hub.state().path().to_path_buf());
        let minted = holler_hub::token::mint(label, 24 * 3600, &hub_state)?;
        let hub_key = hex::encode(hub.x25519_pubkey()?);

        let state = StateDir::fresh()?;
        let join = Command::new(holler_bin)
            .env("HOLLER_STATE_DIR", state.path())
            .env("HOLLER_DEBUG", "quiet")
            .args([
                "body",
                "join",
                "--server",
                &hub.ws_url,
                "--token",
                &format!("{}:{}", minted.record.token_id, minted.secret),
                "--hub-key",
                &hub_key,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if !join.status.success() {
            return Err(format!("`body join` for {label} failed: {}", String::from_utf8_lossy(&join.stderr)).into());
        }

        let config = write_sessions_toml(&state, stub_acp, sessions)?;
        let mut cmd = Command::new(holler_bin);
        cmd.env("HOLLER_STATE_DIR", state.path())
            .env("HOLLER_DEBUG", "quiet")
            .env("HOLLER_LOG_FORMAT", "json")
            .args(["body", "run", "--config"])
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        own_process_group(&mut cmd);
        let child = cmd.spawn()?;

        Ok(Self {
            session_names: (0..sessions).map(|i| format!("{label}/s{i}")).collect(),
            child,
            state,
            holler_bin: holler_bin.to_path_buf(),
        })
    }

    /// Like [`Self::start`], but pipes the body's stderr and relays its
    /// `component=session` `queue_enqueue`/`queue_dequeue`/`queue_full`
    /// debug events (issue #197's own instrumentation — see
    /// `crates/holler-body/src/session_manager/task.rs`'s `log_session`) to
    /// `events`, tagged with this body's `label`.
    ///
    /// Scenario 3 (issue #372) is the only caller: it needs the FIFO queue
    /// depth *over time* to answer "does it grow unbounded, or drain?", and
    /// this debug line already carries exactly that (`depth=` after every
    /// enqueue/dequeue) at `HOLLER_DEBUG=quiet` — the same level every other
    /// scenario already runs bodies at. Reusing it means #372 adds no new
    /// instrumentation to `holler-body` at all; it only teaches the harness
    /// to read what the body already says on its own stderr, exactly the way
    /// `Hub::start` already reads the hub's `listening` event off its
    /// stderr.
    pub fn start_watched(
        hub: &Hub,
        holler_bin: &Path,
        stub_acp: &Path,
        label: &str,
        sessions: usize,
        events: Sender<QueueEvent>,
    ) -> Res<Self> {
        let hub_state = holler_hub::state::HubState::from_root(hub.state().path().to_path_buf());
        let minted = holler_hub::token::mint(label, 24 * 3600, &hub_state)?;
        let hub_key = hex::encode(hub.x25519_pubkey()?);

        let state = StateDir::fresh()?;
        let join = Command::new(holler_bin)
            .env("HOLLER_STATE_DIR", state.path())
            .env("HOLLER_DEBUG", "quiet")
            .args([
                "body",
                "join",
                "--server",
                &hub.ws_url,
                "--token",
                &format!("{}:{}", minted.record.token_id, minted.secret),
                "--hub-key",
                &hub_key,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if !join.status.success() {
            return Err(format!("`body join` for {label} failed: {}", String::from_utf8_lossy(&join.stderr)).into());
        }

        let config = write_sessions_toml(&state, stub_acp, sessions)?;
        let mut cmd = Command::new(holler_bin);
        cmd.env("HOLLER_STATE_DIR", state.path())
            .env("HOLLER_DEBUG", "quiet")
            .env("HOLLER_LOG_FORMAT", "json")
            .args(["body", "run", "--config"])
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        own_process_group(&mut cmd);
        let mut child = cmd.spawn()?;

        let stderr = child.stderr.take().ok_or("watched body's stderr was not piped")?;
        let label_owned = label.to_string();
        std::thread::spawn(move || relay_queue_events(stderr, &label_owned, &events));

        Ok(Self {
            session_names: (0..sessions).map(|i| format!("{label}/s{i}")).collect(),
            child,
            state,
            holler_bin: holler_bin.to_path_buf(),
        })
    }

    /// Ask the body to detach, then reap its whole process tree (it owns
    /// `stub-acp` children; killing only the direct child would orphan them).
    pub fn stop(mut self) {
        let _ = Command::new(&self.holler_bin)
            .env("HOLLER_STATE_DIR", self.state.path())
            .env("HOLLER_DEBUG", "quiet")
            .args(["body", "detach"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        kill_tree(&mut self.child);
    }
}

/// Write `<state>/body/sessions.toml` with `sessions` spawn-mode rows, each
/// running the real `stub-acp` binary in place of a model.
fn write_sessions_toml(state: &StateDir, stub_acp: &Path, sessions: usize) -> Res<PathBuf> {
    let stub = stub_acp.to_str().ok_or("stub-acp path is not valid UTF-8")?;
    let mut toml = String::new();
    for i in 0..sessions {
        // `--chunks 2` keeps each stub turn short: the harness is measuring
        // Holler's overhead, not the stub's own streaming.
        let argv = [stub, "--chunks", "2"]
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        toml.push_str(&format!(
            "[[session]]\nname = \"s{i}\"\nharness = \"opencode\"\ncommand = [{argv}]\n"
        ));
    }
    let dir = state.path().join("body");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("sessions.toml");
    std::fs::write(&path, toml)?;
    Ok(path)
}

/// Which call a driver tick issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    Say,
    Interrupt,
    Roster,
}

/// Drive `total_calls` calls across `sessions` at `rate_per_sec`, recording
/// each call's wall-clock latency.
///
/// The mix rotates `say` → `roster` → `say` → `interrupt`, so a run exercises
/// all three of the methods #369 names without needing three separate knobs.
/// `interrupt` is issued against a session that may well be idle — that is a
/// real, answerable call (the hub reports "nothing in flight"), and its
/// latency is still the hub's round-trip latency, which is what is being
/// measured.
pub fn drive_calls(
    holler_bin: &Path,
    hub: &Hub,
    sessions: &[String],
    rate_per_sec: f64,
    total_calls: usize,
) -> Samples {
    let mut samples = Samples::new();
    if sessions.is_empty() || total_calls == 0 {
        return samples;
    }
    let interval = if rate_per_sec > 0.0 {
        Duration::from_secs_f64(1.0 / rate_per_sec)
    } else {
        Duration::ZERO
    };
    let started = Instant::now();

    let outcomes: Vec<Option<Duration>> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(total_calls);
        for n in 0..total_calls {
            let session = match sessions.get(n % sessions.len()) {
                Some(s) => s.clone(),
                None => continue,
            };
            let call = match n % 4 {
                1 => Call::Roster,
                3 => Call::Interrupt,
                _ => Call::Say,
            };
            let due = started + interval.mul_f64(n as f64);
            handles.push(scope.spawn(move || {
                let now = Instant::now();
                if due > now {
                    std::thread::sleep(due - now);
                }
                run_call(holler_bin, hub, call, &session, n)
            }));
        }
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });

    for outcome in outcomes.into_iter().flatten() {
        samples.push(outcome);
    }
    samples
}

/// One call against the real CLI. `None` if the call failed — a failed call's
/// duration is not a latency measurement and must not pollute the statistics
/// (the caller compares `samples.len()` against the calls it asked for to see
/// how many failed).
fn run_call(holler_bin: &Path, hub: &Hub, call: Call, session: &str, n: usize) -> Option<Duration> {
    let text = format!("load call {n}");
    let args: Vec<&str> = match call {
        Call::Say => vec!["say", session, &text],
        Call::Interrupt => vec!["interrupt", session],
        Call::Roster => vec!["roster", "--json"],
    };
    let started = Instant::now();
    let out = Command::new(holler_bin)
        .env("HOLLER_STATE_DIR", hub.state().path())
        .env("HOLLER_DEBUG", "quiet")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let elapsed = started.elapsed();
    // `interrupt` against an idle session exits non-zero by design (there is
    // nothing in flight); that is a completed round trip, not a failure.
    if out.status.success() || call == Call::Interrupt {
        Some(elapsed)
    } else {
        None
    }
}
