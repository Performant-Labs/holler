//! Scenario 3: sustained throughput (issue #372).
//!
//! A real body fleet (`fleet.rs`, the same real `holler body run` processes
//! and real spawn-mode `stub-acp` sessions every other body-mode scenario
//! uses), driven with a **sustained** `say --queue` rate across every
//! session concurrently for a real, non-trivial wall-clock duration —
//! `--sustained-secs` (default 60s) — not a handful of calls. #372's own
//! framing is explicit that a burst is not what it asks for ("long enough to
//! observe steady-state behavior, not just a burst"); 60s at the default
//! rate/session count drives on the order of a thousand real turns through
//! three real body processes, which is long enough for a queue to fill,
//! drain, and refill more than once, and long enough for RSS to either show
//! a trend or not. It is still short enough to run by hand in well under two
//! minutes end to end (this is, like scenarios 1 and 2, deliberately **not**
//! wired into CI — see `docs/testing.md`'s "It is not in CI, deliberately").
//!
//! `--queue` is not incidental: a plain `say` against a session mid-turn is
//! refused outright with `-32009 session_busy`, which would just make a
//! rate that exceeds one session's own turn-completion rate look like a
//! wall of harness failures rather than the real, answerable thing #372
//! asks about — whether the FIFO queue that `--queue` feeds grows unbounded
//! under sustained load, or drains once load eases. `SessionManager`'s FIFO
//! is capped at 64 by construction (`QUEUE_CAP`,
//! `crates/holler-body/src/session_manager.rs`) — "unbounded" therefore has
//! a definite, code-level answer already; what this scenario adds is the
//! *empirical* half: does a real sustained rate actually build a real
//! backlog, does it stay well under that cap or hit it, and does the queue
//! actually drain to 0 once the drive stops, rather than merely being
//! guaranteed to in theory.
//!
//! # Where the queue-depth signal comes from
//!
//! `holler-body` already emits exactly the signal #372 needs, at
//! `HOLLER_DEBUG=quiet` (the level every scenario already runs bodies at):
//! `component=session` `queue_enqueue`/`queue_dequeue`/`queue_full` debug
//! events, each carrying the queue's own `depth` after the event (issue
//! #197's instrumentation, `session_manager/task.rs::log_session`). Rather
//! than adding a new control-socket query or a new `body status` field
//! (checked first — there is no existing queue-depth surface anywhere in
//! `holler_hub::control` or `body status`), this scenario pipes each
//! watched body's stderr and relays those three event kinds
//! (`fleet.rs::FleetMember::start_watched` / `QueueEvent`). This is the
//! smallest real instrumentation path: zero new code in `holler-body`
//! itself, reusing a debug line that already exists for exactly this
//! purpose.
//!
//! # RSS over time, not a before/after snapshot
//!
//! Every other scenario samples the hub's RSS once, at steady state.
//! "Flat after warmup" vs "monotonic growth" cannot be read off two points,
//! so this scenario's own background thread samples `proc::CpuMeter` every
//! `--rss-sample-ms` (default 2s) for the whole sustained-drive window and
//! reports the full series (`metrics::RssPoint`).

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::fleet::{FleetMember, QueueEvent, QueueEventKind};
use crate::hub::Hub;
use crate::metrics::{Report, RssPoint, Samples, StepReport};
use crate::proc::CpuMeter;
use crate::scenario::wait_for_client_count;
use crate::{Config, Res};

/// How long, after the sustained drive stops sending new calls, to keep
/// observing the queue before calling its final depth "final". Long enough
/// for a full session's own in-flight turn plus a handful of queued ones
/// behind it to finish draining (each turn here is ~100ms of stub work plus
/// real process/wire overhead — see the module doc), short enough not to
/// dominate the run's own wall clock.
const COOLDOWN: Duration = Duration::from_secs(8);

pub async fn run(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    let hub_pid = hub.pid.ok_or(
        "scenario `sustained-throughput` samples the hub process's own RSS over time, which needs a \
         hub this harness started — rerun without `--hub-url`",
    )?;
    let mut baseline_meter = CpuMeter::new(hub_pid);
    let _ = baseline_meter.sample();
    tokio::time::sleep(cfg.settle).await;
    report.hub_baseline = baseline_meter.sample();
    let baseline = report.hub_baseline;

    let (queue_tx, queue_rx) = std_mpsc::channel::<QueueEvent>();
    let (members, failures) = start_watched_fleet(cfg, hub, queue_tx);
    wait_for_client_count(hub, members.len() as u64, Duration::from_secs(60)).await;
    tokio::time::sleep(cfg.settle).await;

    let sessions: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();
    if sessions.is_empty() {
        for member in members {
            member.stop();
        }
        return Err(format!(
            "sustained-throughput could not start any session (fleet start failures: {})",
            failures.join("; ")
        )
        .into());
    }

    let queue = spawn_queue_aggregator(queue_rx);
    let rss = spawn_rss_sampler(hub_pid, cfg.rss_sample_interval);

    let step_started = Instant::now();
    let drive_started = Instant::now();
    let (say_samples, say_failures) = {
        let holler_bin = cfg.holler_bin.clone();
        let sessions = sessions.clone();
        let rate = cfg.rate;
        let duration = cfg.sustained;
        let concurrency = cfg.connect_concurrency;
        tokio::task::block_in_place(|| drive_sustained_say(&holler_bin, hub, &sessions, rate, duration, concurrency))
    };
    let drive_elapsed = drive_started.elapsed();

    // Let load ease before reading the "final" queue depth and RSS — #372
    // asks whether the queue "drains correctly once load eases", which this
    // cooldown window is what actually lets happen.
    tokio::time::sleep(COOLDOWN).await;

    let steady = baseline_meter.sample();
    let outcome = queue.finish();
    let rss_points = rss.finish();

    report.steps.push(build_step_report(
        cfg,
        hub,
        &members,
        failures,
        baseline,
        steady,
        step_started,
        say_samples,
        say_failures,
        drive_elapsed,
        outcome,
        rss_points,
    ));

    for member in members {
        member.stop();
    }
    Ok(())
}

/// Start `cfg.bodies` real, stderr-watched bodies (`FleetMember::start_watched`),
/// each cloning `events` so its own reader thread can relay queue-depth
/// events. Split out of [`run`] to keep that function under this
/// workspace's line-count lint.
fn start_watched_fleet(cfg: &Config, hub: &Hub, events: std_mpsc::Sender<QueueEvent>) -> (Vec<FleetMember>, Vec<String>) {
    let mut members = Vec::with_capacity(cfg.bodies);
    let mut failures = Vec::new();
    for i in 0..cfg.bodies {
        match FleetMember::start_watched(
            hub,
            &cfg.holler_bin,
            &cfg.stub_acp_bin,
            &format!("th{i}"),
            cfg.sessions_per_body,
            events.clone(),
        ) {
            Ok(m) => members.push(m),
            Err(e) => failures.push(e.to_string()),
        }
    }
    // Every `FleetMember`'s own reader thread holds a clone; dropping this
    // one means the channel (and the aggregator thread reading it) ends once
    // every body's stderr pipe closes, not before.
    drop(events);
    (members, failures)
}

/// The queue-depth aggregator's final numbers: the largest total depth
/// (summed across every session) observed at any point, the depth at the
/// very last event (after the cooldown — 0 means every session's FIFO fully
/// drained), how many enqueue/dequeue events were observed in total, and how
/// many `--queue` calls were refused outright because a session's FIFO was
/// already at its hard cap.
struct QueueOutcome {
    max_depth: Option<usize>,
    final_depth: Option<usize>,
    events_observed: usize,
    full_refusals: usize,
}

/// A running queue-depth aggregator: consumes [`QueueEvent`]s off a channel
/// on a background thread, maintaining each session's own current depth so
/// the reported total is the real *sum* across every session this run
/// started. Reads with a bounded timeout (not a blocking `for ev in rx`)
/// specifically so [`Self::finish`] can return a live snapshot — "the depth
/// right after the cooldown, while the fleet is still running" — without
/// first having to kill every fleet member to close the channel.
struct QueueAggregator {
    stop: Arc<AtomicBool>,
    outcome: Arc<Mutex<QueueOutcome>>,
    handle: std::thread::JoinHandle<()>,
}

impl QueueAggregator {
    /// Signal the background thread to stop, join it, and return the
    /// snapshot as of that moment.
    fn finish(self) -> QueueOutcome {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.handle.join();
        let locked = self.outcome.lock().unwrap_or_else(|e| e.into_inner());
        QueueOutcome {
            max_depth: locked.max_depth,
            final_depth: locked.final_depth,
            events_observed: locked.events_observed,
            full_refusals: locked.full_refusals,
        }
    }
}

/// Spawn the queue-depth aggregator thread. Split out of [`run`] to keep
/// that function under this workspace's line-count lint.
fn spawn_queue_aggregator(rx: std_mpsc::Receiver<QueueEvent>) -> QueueAggregator {
    let stop = Arc::new(AtomicBool::new(false));
    let outcome = Arc::new(Mutex::new(QueueOutcome { max_depth: None, final_depth: None, events_observed: 0, full_refusals: 0 }));
    let handle = {
        let stop = Arc::clone(&stop);
        let outcome = Arc::clone(&outcome);
        std::thread::spawn(move || {
            // Keyed by (body label, session name): each session has its own
            // independent FIFO, so the reported total is the *sum* across
            // every session this run started, not any single session's own
            // depth.
            let mut per_session: HashMap<(String, String), usize> = HashMap::new();
            while !stop.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(ev) => {
                        let mut o = outcome.lock().unwrap_or_else(|e| e.into_inner());
                        o.events_observed += 1;
                        match ev.kind {
                            QueueEventKind::Full => {
                                o.full_refusals += 1;
                                continue;
                            }
                            QueueEventKind::Enqueue | QueueEventKind::Dequeue => {
                                per_session.insert((ev.label, ev.session), ev.depth);
                            }
                        }
                        let total: usize = per_session.values().sum();
                        o.max_depth = Some(o.max_depth.unwrap_or(0).max(total));
                        o.final_depth = Some(total);
                    }
                    Err(std_mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        })
    };
    QueueAggregator { stop, outcome, handle }
}

/// A running hub-RSS-over-time sampler: samples `proc::CpuMeter` on a
/// background thread every `interval` until [`RssSampler::finish`] signals
/// it to stop, so a sustained run's RSS is a real series, not one
/// before/after snapshot.
struct RssSampler {
    stop: Arc<AtomicBool>,
    series: Arc<Mutex<Vec<RssPoint>>>,
    handle: std::thread::JoinHandle<()>,
}

impl RssSampler {
    fn finish(self) -> Vec<RssPoint> {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.handle.join();
        std::mem::take(&mut *self.series.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// Spawn the RSS-over-time sampler thread. Split out of [`run`] to keep that
/// function under this workspace's line-count lint.
fn spawn_rss_sampler(hub_pid: u32, interval: Duration) -> RssSampler {
    let series: Arc<Mutex<Vec<RssPoint>>> = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let handle = {
        let series = Arc::clone(&series);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut meter = CpuMeter::new(hub_pid);
            let started = Instant::now();
            while !stop.load(Ordering::Relaxed) {
                let sample = meter.sample();
                series.lock().unwrap_or_else(|e| e.into_inner()).push(RssPoint {
                    elapsed_s: started.elapsed().as_secs_f64(),
                    rss_mib: sample.rss_mib,
                    threads: sample.threads,
                });
                std::thread::sleep(interval);
            }
        })
    };
    RssSampler { stop, series, handle }
}

/// Build the run's single [`StepReport`]. Split out of [`run`] for the same
/// line-count-lint reason `session_scale.rs`'s own `build_step_report` was.
#[allow(clippy::too_many_arguments)] // #372: split out of `run` purely to satisfy the line-count lint
fn build_step_report(
    cfg: &Config,
    hub: &Hub,
    members: &[FleetMember],
    failures: Vec<String>,
    baseline: crate::metrics::ResourceSample,
    steady: crate::metrics::ResourceSample,
    step_started: Instant,
    say_samples: Samples,
    say_failures: usize,
    drive_elapsed: Duration,
    queue: QueueOutcome,
    rss_series: Vec<RssPoint>,
) -> StepReport {
    let hub_status_clients = hub.client_count().ok();
    let rss_delta_per_client_kib = match (baseline.rss_mib, steady.rss_mib) {
        (Some(b), Some(s)) if !members.is_empty() => Some(((s - b) * 1024.0) / members.len() as f64),
        _ => None,
    };
    let threads_delta = match (baseline.threads, steady.threads) {
        (Some(b), Some(s)) => Some(s as i64 - b as i64),
        _ => None,
    };
    let sessions_expected = (cfg.bodies * cfg.sessions_per_body) as u64;
    let sessions_actual = hub.sessions_count().ok();
    StepReport {
        clients_target: cfg.bodies,
        clients_live: members.len(),
        hub_status_clients,
        hub_status_matches: hub_status_clients == Some(members.len() as u64),
        connect_failures: failures,
        ws_connect: None,
        handshake: None,
        connect_total: None,
        calls: say_samples.stats(),
        hub_at_steady_state: steady,
        rss_delta_per_client_kib,
        threads_delta,
        step_seconds: step_started.elapsed().as_secs_f64(),
        sessions_per_body: Some(cfg.sessions_per_body),
        sessions_expected: Some(sessions_expected),
        sessions_actual,
        sessions_match: Some(sessions_actual == Some(sessions_expected)),
        presence_fanout: None,
        roster_read: None,
        sustained_seconds: Some(drive_elapsed.as_secs_f64()),
        say_failures: Some(say_failures),
        queue_depth_max: queue.max_depth,
        queue_depth_final: queue.final_depth,
        queue_events_observed: Some(queue.events_observed),
        queue_full_refusals: Some(queue.full_refusals),
        rss_series,
    }
}

/// Drive real `say <session> <text> --queue` calls across `sessions` at
/// `rate_per_sec` (total, round-robin across sessions) for `duration`,
/// recording each call's real wall-clock round-trip latency.
///
/// Bounded to `concurrency` OS worker threads regardless of how many total
/// calls that works out to (at the default 60s / a few calls/sec that is
/// already a few hundred `holler` subprocess invocations) — each worker
/// claims the next scheduled call index and sleeps until that call's own
/// due instant, the same open-loop-scheduling idea `fleet::drive_calls`
/// uses, just spread over a bounded pool instead of one thread per call so
/// a long sustained run does not need one OS thread per call.
fn drive_sustained_say(
    holler_bin: &Path,
    hub: &Hub,
    sessions: &[String],
    rate_per_sec: f64,
    duration: Duration,
    concurrency: usize,
) -> (Samples, usize) {
    if sessions.is_empty() || rate_per_sec <= 0.0 {
        return (Samples::new(), 0);
    }
    let interval = Duration::from_secs_f64(1.0 / rate_per_sec);
    let total_calls = ((duration.as_secs_f64()) * rate_per_sec).round().max(1.0) as usize;
    let started = Instant::now();
    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    let failures = std::sync::atomic::AtomicUsize::new(0);
    let samples = Mutex::new(Samples::new());

    std::thread::scope(|scope| {
        for _ in 0..concurrency.max(1) {
            scope.spawn(|| loop {
                let n = next_idx.fetch_add(1, Ordering::Relaxed);
                if n >= total_calls {
                    return;
                }
                let session = &sessions[n % sessions.len()];
                let due = started + interval.mul_f64(n as f64);
                let now = Instant::now();
                if due > now {
                    std::thread::sleep(due - now);
                }
                match run_say_queued(holler_bin, hub, session, n) {
                    Some(d) => samples.lock().unwrap_or_else(|e| e.into_inner()).push(d),
                    None => {
                        failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    });

    (samples.into_inner().unwrap_or_else(|e| e.into_inner()), failures.load(Ordering::Relaxed))
}

/// One real `holler say <session> "..." --queue` invocation, timed end to
/// end (the whole CLI round trip, including the turn itself — `say` blocks
/// until the turn resolves, unlike `session_scale.rs`'s fire-and-poll
/// fan-out probe). `None` on any failure (non-zero exit, including
/// `-32009`/`-32007` since `--queue` should make those rare, or the process
/// could not even be spawned) — a failed call is not a latency sample.
fn run_say_queued(holler_bin: &Path, hub: &Hub, session: &str, n: usize) -> Option<Duration> {
    let text = format!("sustained load call {n}");
    let started = Instant::now();
    let out = Command::new(holler_bin)
        .env("HOLLER_STATE_DIR", hub.state().path())
        .env("HOLLER_DEBUG", "quiet")
        .args(["say", session, &text, "--queue"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let elapsed = started.elapsed();
    if out.status.success() {
        Some(elapsed)
    } else {
        None
    }
}
