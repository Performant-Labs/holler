//! Scenario 4: churn (issue #373).
//!
//! **Churn cycles.** Each cycle starts `--bodies` real `holler body run`
//! processes (each hosting `--sessions-per-body` spawn-mode `stub-acp`
//! sessions), waits until the hub reports every one of them, drives one real
//! `say` per session, then tears them down. Afterwards `hub status --json`'s
//! `clients` and `sessions` must return to their pre-run baseline: #373's "no
//! leaked state" invariant. A miss aborts the run with a non-zero exit rather
//! than being reported as a slow number.
//!
//! Run #1 used one teardown (`graceful`: `body detach`, then kill). Run #2
//! adds, via `--teardown-modes` (rotated per cycle, see `teardown.rs`):
//! `crash` (SIGKILL, no detach), `hang` (SIGSTOP, so connected but silent;
//! whether the hub cleans up on its own is recorded, not failed), and
//! `restart` (crash, then `body run` again on the same saved credential).
//! Also `--parallel-teardown`, a `--resident` body that stays up for the
//! whole run and gets `say` traffic while each wave is torn down, a
//! `--label-reuse-probe`, `--churn-secs` for a time-bounded run, and a
//! per-cycle series of hub RSS/threads/roster size.
//!
//! **Dead-backend detection.** Reproduces the 2026-09-21 incident #373 cites
//! (an attach-mode backend killed outside Holler, while the roster kept
//! showing its session `connected`/`idle`) with a real process; see
//! `probes::dead_backend`. Not detecting it is a recorded result, not a
//! harness failure: it's the bug (#397).
//!
//! Timings come from polling `hub status --json` / the roster, so their
//! resolution is the poll interval plus one CLI round trip (tens of ms).

mod probes;
mod teardown;

pub use teardown::TeardownMode;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::fleet::FleetMember;
use crate::hub::Hub;
use crate::metrics::{
    ChurnReport, CycleSample, HangOutcome, Report, ResidentOutcome, ResourceSample, RestartOutcome, Samples, StepReport,
};
use crate::proc::CpuMeter;
use crate::{Config, Res};

const REGISTER_BUDGET: Duration = Duration::from_secs(60);
const CLEANUP_BUDGET: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(25);

pub async fn run(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    tokio::task::block_in_place(|| run_blocking(cfg, hub, report))
}

/// Everything one cycle's teardown needs to know.
pub struct Cycle<'a> {
    cfg: &'a Config,
    hub: &'a Hub,
    cycle: usize,
    /// `(clients, sessions)` before any cycle (the resident body included).
    baseline: (u64, u64),
    /// `baseline` plus this cycle's wave.
    want: (u64, u64),
    resident_sessions: Option<&'a [String]>,
}

/// Everything measured across the run.
#[derive(Default)]
pub struct Accum {
    join: Samples,
    say: Samples,
    say_failures: usize,
    detach: Samples,
    cleanup: Samples,
    cleanup_by_mode: BTreeMap<String, Samples>,
    hang_cycles: usize,
    hang_cleaned: usize,
    hang_latency: Samples,
    hang_roster: Option<String>,
    restart_cycles: usize,
    rejoin: Samples,
    identity_changed: usize,
    ghost_or_missing: usize,
    not_connected: usize,
    resident_say: Samples,
    resident_failures: usize,
    resident_checks: usize,
    resident_disturbances: usize,
    first_graceful_label: Option<String>,
    series: Vec<CycleSample>,
}

impl Accum {
    fn record_cleanup(&mut self, mode: TeardownMode, d: Duration) {
        self.cleanup.push(d);
        self.cleanup_by_mode.entry(mode.name().to_string()).or_default().push(d);
    }
}

fn run_blocking(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    let started = Instant::now();
    let mut meter = hub.pid.map(CpuMeter::new);
    report.hub_baseline = sample(&mut meter);
    let rss_before = report.hub_baseline.rss_mib;
    let initial_clients = counts(hub).map(|c| c.0);

    let resident = if cfg.resident { Some(start_resident(cfg, hub)?) } else { None };
    let outcome = churn_loop(cfg, hub, started, &mut meter, resident.as_ref());
    let resident_sessions = resident.as_ref().map_or(0, |r| r.session_names.len());
    if let Some(r) = resident {
        r.stop();
    }
    let (acc, cycles) = outcome?;

    let label_reuse = match (&acc.first_graceful_label, cfg.label_reuse_probe) {
        (Some(label), true) => Some(probes::label_reuse(hub, label)),
        (None, true) => Some("not run: no graceful cycle happened".to_string()),
        _ => None,
    };
    let steady = sample(&mut meter);
    let dead_backend = probes::dead_backend(cfg, hub)?;
    let final_clients = counts(hub).map(|c| c.0);
    let churn = build_churn_report(cfg, acc, cycles, resident_sessions, rss_before, steady.rss_mib, label_reuse, dead_backend);
    let back_to_initial = final_clients.is_some() && final_clients == initial_clients;
    report.steps.push(step_report(cfg, started, steady, final_clients, back_to_initial, churn));
    Ok(())
}

fn churn_loop(
    cfg: &Config,
    hub: &Hub,
    started: Instant,
    meter: &mut Option<CpuMeter>,
    resident: Option<&FleetMember>,
) -> Res<(Accum, usize)> {
    let baseline = counts(hub).ok_or("could not read `hub status --json` before the first cycle")?;
    let wave = (cfg.bodies as u64, (cfg.bodies * cfg.sessions_per_body) as u64);
    let mut acc = Accum::default();
    let mut cycle = 0;
    while cfg.churn_secs.map_or(cycle < cfg.churn_cycles, |secs| started.elapsed() < secs) {
        let mode = cfg.teardown_modes[cycle % cfg.teardown_modes.len()];
        let c = Cycle {
            cfg,
            hub,
            cycle,
            baseline,
            want: (baseline.0 + wave.0, baseline.1 + wave.1),
            resident_sessions: resident.map(|r| r.session_names.as_slice()),
        };
        run_cycle(&c, mode, &mut acc)?;
        check_resident(&c, &mut acc);
        let s = sample(meter);
        acc.series.push(CycleSample {
            cycle,
            mode: mode.name().to_string(),
            elapsed_s: started.elapsed().as_secs_f64(),
            rss_mib: s.rss_mib,
            threads: s.threads,
            roster_rows: Some(probes::roster_rows(hub).len()),
            minted_records: probes::token_count(hub),
        });
        cycle += 1;
    }
    Ok((acc, cycle))
}

/// One cycle: start the wave, check the hub registers all of it, drive one
/// `say` per session, then tear it down in `mode`.
fn run_cycle(c: &Cycle, mode: TeardownMode, acc: &mut Accum) -> Res<()> {
    let joined_at = Instant::now();
    let mut members = Vec::with_capacity(c.cfg.bodies);
    for i in 0..c.cfg.bodies {
        let label = format!("ch{}-{i}", c.cycle);
        match FleetMember::start(c.hub, &c.cfg.holler_bin, &c.cfg.stub_acp_bin, &label, c.cfg.sessions_per_body) {
            Ok(m) => members.push(m),
            Err(e) => {
                teardown_all(members, false, FleetMember::stop);
                return Err(format!("churn cycle {}: body {i} failed to start: {e}", c.cycle).into());
            }
        }
    }
    let seen = wait_for_counts(c.hub, c.want, REGISTER_BUDGET);
    if seen != Some(c.want) {
        teardown_all(members, false, FleetMember::stop);
        return Err(format!(
            "churn cycle {}: hub never registered the wave; wanted {}, saw {}",
            c.cycle,
            fmt(Some(c.want)),
            fmt(seen)
        )
        .into());
    }
    acc.join.push(joined_at.elapsed());
    if mode == TeardownMode::Graceful && acc.first_graceful_label.is_none() {
        acc.first_graceful_label = Some(format!("ch{}-0", c.cycle));
    }

    let names: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();
    let (say, failures) = say_all(&c.cfg.holler_bin, c.hub, &names);
    for d in say {
        acc.say.push(d);
    }
    acc.say_failures += failures;

    teardown::run(mode, c, members, acc)
}

fn start_resident(cfg: &Config, hub: &Hub) -> Res<FleetMember> {
    let before = counts(hub).ok_or("could not read `hub status --json` before starting the resident body")?;
    let resident = FleetMember::start(hub, &cfg.holler_bin, &cfg.stub_acp_bin, "resident", cfg.sessions_per_body)?;
    let want = (before.0 + 1, before.1 + cfg.sessions_per_body as u64);
    let seen = wait_for_counts(hub, want, REGISTER_BUDGET);
    if seen != Some(want) {
        resident.stop();
        return Err(format!("resident body never registered; wanted {}, saw {}", fmt(Some(want)), fmt(seen)).into());
    }
    Ok(resident)
}

/// After a cycle's cleanup, every resident session must still be connected
/// and idle/working; anything else counts as a disturbance.
fn check_resident(c: &Cycle, acc: &mut Accum) {
    let Some(names) = c.resident_sessions else { return };
    let rows = probes::roster_rows(c.hub);
    for name in names {
        acc.resident_checks += 1;
        if !probes::is_healthy(probes::rows_named(&rows, name).next()) {
            acc.resident_disturbances += 1;
        }
    }
}

/// Run `f` (a teardown) while the resident body, if any, takes one `say` per
/// session: churn happening next to live traffic.
fn with_resident_traffic<R>(c: &Cycle, acc: &mut Accum, f: impl FnOnce() -> R) -> R {
    let Some(names) = c.resident_sessions else { return f() };
    let (result, (samples, failures)) = std::thread::scope(|scope| {
        let traffic = scope.spawn(|| say_all(&c.cfg.holler_bin, c.hub, names));
        let result = f();
        (result, traffic.join().unwrap_or_default())
    });
    for d in samples {
        acc.resident_say.push(d);
    }
    acc.resident_failures += failures;
    result
}

/// Tear every member down with `kill`, one at a time or all at once.
fn teardown_all(members: Vec<FleetMember>, parallel: bool, kill: fn(FleetMember)) {
    if !parallel {
        members.into_iter().for_each(kill);
        return;
    }
    std::thread::scope(|scope| {
        for m in members {
            scope.spawn(move || kill(m));
        }
    });
}

/// Wait for the hub to return to `c.baseline`; error (aborting the run) if it
/// doesn't within the cleanup budget.
fn expect_baseline(c: &Cycle, after: &str) -> Res<()> {
    let seen = wait_for_counts(c.hub, c.baseline, CLEANUP_BUDGET);
    if seen == Some(c.baseline) {
        return Ok(());
    }
    Err(format!(
        "churn cycle {} ({after}): hub did not return to baseline within {}s (leaked state); wanted {}, saw {}",
        c.cycle,
        CLEANUP_BUDGET.as_secs(),
        fmt(Some(c.baseline)),
        fmt(seen),
    )
    .into())
}

/// One real `say` per session, all in flight at once.
fn say_all(holler_bin: &Path, hub: &Hub, sessions: &[String]) -> (Vec<Duration>, usize) {
    let outcomes: Vec<Option<Duration>> = std::thread::scope(|scope| {
        let handles: Vec<_> = sessions
            .iter()
            .map(|s| {
                scope.spawn(move || {
                    let t = Instant::now();
                    let out = Command::new(holler_bin)
                        .env("HOLLER_STATE_DIR", hub.state().path())
                        .env("HOLLER_DEBUG", "quiet")
                        .args(["say", s, "churn"])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();
                    matches!(out, Ok(st) if st.success()).then(|| t.elapsed())
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().ok().flatten()).collect()
    });
    let failures = outcomes.iter().filter(|o| o.is_none()).count();
    (outcomes.into_iter().flatten().collect(), failures)
}

/// `(clients, sessions)` from one `hub status --json` read.
fn counts(hub: &Hub) -> Option<(u64, u64)> {
    let doc = hub.status_json().ok()?;
    Some((doc.get("clients")?.as_u64()?, doc.get("sessions")?.as_u64()?))
}

/// Poll until the hub reports exactly `want`, or the budget runs out;
/// returns the last real reading either way.
fn wait_for_counts(hub: &Hub, want: (u64, u64), budget: Duration) -> Option<(u64, u64)> {
    let deadline = Instant::now() + budget;
    let mut last = None;
    loop {
        last = counts(hub).or(last);
        if last == Some(want) || Instant::now() >= deadline {
            return last;
        }
        std::thread::sleep(POLL);
    }
}

fn fmt(c: Option<(u64, u64)>) -> String {
    c.map_or_else(|| "unreadable".to_string(), |(cl, s)| format!("clients={cl} sessions={s}"))
}

fn sample(meter: &mut Option<CpuMeter>) -> ResourceSample {
    meter.as_mut().map(CpuMeter::sample).unwrap_or_default()
}

#[allow(clippy::too_many_arguments)] // #373: one call site, assembling the report from run_blocking's locals
fn build_churn_report(
    cfg: &Config,
    acc: Accum,
    cycles: usize,
    resident_sessions: usize,
    rss_before: Option<f64>,
    rss_after: Option<f64>,
    label_reuse: Option<String>,
    dead_backend: crate::metrics::DeadBackendProbe,
) -> ChurnReport {
    ChurnReport {
        cycles_target: cfg.churn_secs.map_or(cfg.churn_cycles, |_| cycles),
        cycles_completed: cycles,
        bodies_per_cycle: cfg.bodies,
        sessions_per_body: cfg.sessions_per_body,
        join_to_registered: acc.join.stats(),
        say: acc.say.stats(),
        say_failures: acc.say_failures,
        detach: acc.detach.stats(),
        cleanup: acc.cleanup.stats(),
        hub_rss_before_mib: rss_before,
        hub_rss_after_mib: rss_after,
        dead_backend: Some(dead_backend),
        teardown_modes: cfg.teardown_modes.iter().map(|m| m.name().to_string()).collect(),
        parallel_teardown: cfg.parallel_teardown,
        cleanup_by_mode: acc.cleanup_by_mode.iter().filter_map(|(k, v)| v.stats().map(|s| (k.clone(), s))).collect(),
        hang: (acc.hang_cycles > 0).then(|| HangOutcome {
            cycles: acc.hang_cycles,
            budget_secs: cfg.hang_budget.as_secs(),
            cleaned_within_budget: acc.hang_cleaned,
            latency: acc.hang_latency.stats(),
            roster_when_not_cleaned: acc.hang_roster.clone(),
        }),
        restart: (acc.restart_cycles > 0).then(|| RestartOutcome {
            cycles: acc.restart_cycles,
            rejoin: acc.rejoin.stats(),
            identity_changed: acc.identity_changed,
            ghost_or_missing_rows: acc.ghost_or_missing,
            not_connected: acc.not_connected,
        }),
        resident: (resident_sessions > 0).then(|| ResidentOutcome {
            sessions: resident_sessions,
            say: acc.resident_say.stats(),
            say_failures: acc.resident_failures,
            checks: acc.resident_checks,
            disturbances: acc.resident_disturbances,
        }),
        mint_lock_retries: crate::fleet::mint_lock_retries(),
        label_reuse,
        series: acc.series,
    }
}

fn step_report(
    cfg: &Config,
    started: Instant,
    steady: ResourceSample,
    clients: Option<u64>,
    back_to_initial: bool,
    churn: ChurnReport,
) -> StepReport {
    StepReport {
        clients_target: cfg.bodies,
        clients_live: 0,
        hub_status_clients: clients,
        hub_status_matches: back_to_initial,
        connect_failures: Vec::new(),
        ws_connect: None,
        handshake: None,
        connect_total: None,
        calls: None,
        hub_at_steady_state: steady,
        rss_delta_per_client_kib: None,
        threads_delta: None,
        step_seconds: started.elapsed().as_secs_f64(),
        sessions_per_body: None,
        sessions_expected: None,
        sessions_actual: None,
        sessions_match: None,
        presence_fanout: None,
        roster_read: None,
        sustained_seconds: None,
        say_failures: None,
        queue_depth_max: None,
        queue_depth_final: None,
        queue_events_observed: None,
        queue_full_refusals: None,
        rss_series: Vec::new(),
        churn: Some(churn),
    }
}
