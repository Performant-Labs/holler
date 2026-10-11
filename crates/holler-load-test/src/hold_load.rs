//! Scenario 5: session hold (issue #444, part of #437 and #366).
//!
//! The hold check sits on the hub's per-prompt hot path, so every `say` pays
//! for it, and holds add persisted state. The functional tests (issue #442)
//! prove correctness; this scenario measures that the feature is cheap:
//!
//! 1. **Say path with holds.** Two hold-free runs (their difference is the
//!    noise floor), then the same run with 0%, 10% and 50% of the driven
//!    sessions held. The unheld sessions' latency and throughput are compared
//!    with the baseline.
//! 2. **Refusal cost.** Refusing a held session's `say` must be cheaper than
//!    delivering it and must not queue anything: refusal latency at a
//!    sustained rate, and the hub's RSS before and after.
//! 3. **Roster scale.** `roster` latency with many sessions, with and without
//!    every one of them held.
//! 4. **Hold/release churn.** Rapid cycles on many sessions: throughput,
//!    latency, hub CPU, and an RSS leak check after a cooldown.
//! 5. **Restart with many holds.** Hub restart to `listening` with and without
//!    thousands of persisted holds, and how long until every session is back
//!    and shown held.
//!
//! Correctness is hard-failed, not measured: a `say` delivered to a held
//! session, a hold lost across the restart, or a refusal missing after it
//! aborts the run. The numbers are then judged against the thresholds in
//! [`judge`], which were set from the first measured baseline (recorded in
//! `docs/testing.md`) and fail the run when exceeded.
//!
//! Prompts are sent in-process over the control socket (`holler_hub::
//! control::say_at`), the call the CLI's say verb makes, so the numbers are the hub's
//! and not the cost of spawning a CLI process per call. Like the other
//! scenarios this is opt-in and not part of the per-PR suite; a number from a
//! contended shared runner is not a regression (see #345).

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use holler_hub::control::{self, ControlError};

use crate::fleet::FleetMember;
use crate::hold_report::{Churn, HoldReport, Phase, Restart, RosterRead, Threshold};
use crate::hub::Hub;
use crate::metrics::{Report, Samples};
use crate::proc::CpuMeter;
use crate::{Config, Res};

const HELD: i64 = -32011;
const BUSY: i64 = -32009;

/// What one `say` returned, as this scenario cares about it.
#[derive(Default)]
struct Driven {
    unheld: Samples,
    refusals: Samples,
    unheld_delivered: usize,
    busy: usize,
    delivered_to_held: usize,
    other_errors: usize,
}

impl Driven {
    fn merge(&mut self, o: Driven) {
        self.unheld.extend(&o.unheld);
        self.refusals.extend(&o.refusals);
        self.unheld_delivered += o.unheld_delivered;
        self.busy += o.busy;
        self.delivered_to_held += o.delivered_to_held;
        self.other_errors += o.other_errors;
    }
}

fn rss_of(pid: Option<u32>) -> Option<f64> {
    CpuMeter::new(pid?).sample().rss_mib
}

/// One driven session's loop: sequential `say`s until `deadline`.
fn drive_one(root: &Path, session: &str, held: bool, deadline: Instant, refusal_pause: Duration) -> Driven {
    let mut d = Driven::default();
    while Instant::now() < deadline {
        let started = Instant::now();
        let res = control::say_at(root, session, "load", false, Duration::from_secs(60));
        let took = started.elapsed();
        match res {
            Ok(_) if held => d.delivered_to_held += 1,
            Ok(_) => {
                d.unheld.push(took);
                d.unheld_delivered += 1;
            }
            Err(ControlError::Refused(e)) if e.code == HELD && held => {
                d.refusals.push(took);
                std::thread::sleep(refusal_pause);
            }
            Err(ControlError::Refused(e)) if e.code == BUSY => {
                d.busy += 1;
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => {
                d.other_errors += 1;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    d
}

/// Run one phase: every session in `driven` loops `say` for `secs`; the ones
/// in `held` are refused.
fn phase(cfg: &Config, hub: &Hub, label: &str, driven: &[String], held: &HashSet<String>, secs: u64) -> Phase {
    let root = hub.state().path().to_path_buf();
    let rss_start = rss_of(hub.pid);
    let started = Instant::now();
    let deadline = started + Duration::from_secs(secs);
    let pause = cfg.hold_refusal_pause;
    let total = Arc::new(Mutex::new(Driven::default()));
    std::thread::scope(|scope| {
        for s in driven {
            let (root, total) = (&root, Arc::clone(&total));
            let is_held = held.contains(s);
            scope.spawn(move || {
                let d = drive_one(root, s, is_held, deadline, pause);
                total.lock().unwrap_or_else(|e| e.into_inner()).merge(d);
            });
        }
    });
    let elapsed = started.elapsed().as_secs_f64();
    let d = std::mem::take(&mut *total.lock().unwrap_or_else(|e| e.into_inner()));
    Phase {
        label: label.to_string(),
        held_percent: if driven.is_empty() { 0 } else { (held.len() * 100 / driven.len()) as u32 },
        driven_sessions: driven.len(),
        held_sessions: held.len(),
        seconds: elapsed,
        unheld: d.unheld.stats(),
        unheld_delivered: d.unheld_delivered,
        unheld_per_sec: d.unheld_delivered as f64 / elapsed,
        unheld_busy_retries: d.busy,
        refusals: d.refusals.stats(),
        refusals_per_sec: d.refusals.count() as f64 / elapsed,
        delivered_to_held: d.delivered_to_held,
        other_errors: d.other_errors,
        hub_rss_start_mib: rss_start,
        hub_rss_end_mib: rss_of(hub.pid),
    }
}

/// Hold (or release) every session in `sessions`, one after another.
fn set_held(root: &Path, sessions: &[String], on: bool) -> Res<Duration> {
    let started = Instant::now();
    for s in sessions {
        if on {
            control::hold_at(root, s, Some("load test")).map_err(|e| format!("hold {s}: {e}"))?;
        } else {
            control::release_at(root, s).map_err(|e| format!("release {s}: {e}"))?;
        }
    }
    Ok(started.elapsed())
}

fn roster_reads(root: &Path, reads: usize) -> Res<(Samples, usize)> {
    let mut samples = Samples::new();
    let mut rows = 0;
    for _ in 0..reads {
        let started = Instant::now();
        let doc = control::roster_at(root, false, None).map_err(|e| format!("roster: {e}"))?;
        samples.push(started.elapsed());
        rows = doc["rows"].as_array().map_or(0, Vec::len);
    }
    Ok((samples, rows))
}

fn holds_file(root: &Path) -> (u64, usize) {
    let path = root.join("hub").join("holds.json");
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let count = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["holds"].as_object().map(|m| m.len()))
        .unwrap_or(0);
    (bytes, count)
}

/// Hold/release cycles from `workers` threads over `sessions`, for `secs`.
fn churn(cfg: &Config, hub: &Hub, sessions: &[String], secs: u64) -> Churn {
    let root = hub.state().path().to_path_buf();
    let rss_start = rss_of(hub.pid);
    let stop = Arc::new(AtomicBool::new(false));
    let cpu = {
        let (stop, pid) = (Arc::clone(&stop), hub.pid);
        std::thread::spawn(move || {
            let mut meter = pid.map(CpuMeter::new);
            let mut points = Vec::new();
            if let Some(m) = meter.as_mut() {
                let _ = m.sample();
            }
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(1000));
                if let Some(c) = meter.as_mut().and_then(|m| m.sample().cpu_percent) {
                    points.push(c);
                }
            }
            points
        })
    };
    let started = Instant::now();
    let deadline = started + Duration::from_secs(secs);
    let workers = cfg.hold_churn_workers.max(1);
    let results = Arc::new(Mutex::new((Samples::new(), Samples::new(), 0usize, 0usize)));
    std::thread::scope(|scope| {
        for w in 0..workers {
            let (root, results) = (&root, Arc::clone(&results));
            scope.spawn(move || {
                let (mut holds, mut releases, mut ops, mut failed) = (Samples::new(), Samples::new(), 0, 0);
                let mut i = w;
                while Instant::now() < deadline {
                    let s = &sessions[i % sessions.len()];
                    i += workers;
                    let t = Instant::now();
                    let h = control::hold_at(root, s, Some("churn"));
                    holds.push(t.elapsed());
                    let t = Instant::now();
                    let r = control::release_at(root, s);
                    releases.push(t.elapsed());
                    ops += 2;
                    failed += usize::from(h.is_err()) + usize::from(r.is_err());
                }
                let mut g = results.lock().unwrap_or_else(|e| e.into_inner());
                g.0.extend(&holds);
                g.1.extend(&releases);
                g.2 += ops;
                g.3 += failed;
            });
        }
    });
    let elapsed = started.elapsed().as_secs_f64();
    stop.store(true, Ordering::Relaxed);
    let cpu_points = cpu.join().unwrap_or_default();
    // A leak shows as growth that survives a cooldown, not as a peak.
    std::thread::sleep(Duration::from_secs(5));
    let g = std::mem::take(&mut *results.lock().unwrap_or_else(|e| e.into_inner()));
    Churn {
        workers,
        seconds: elapsed,
        operations: g.2,
        failed_operations: g.3,
        ops_per_sec: g.2 as f64 / elapsed,
        hold_latency: g.0.stats(),
        release_latency: g.1.stats(),
        hub_cpu_percent_mean: (!cpu_points.is_empty()).then(|| cpu_points.iter().sum::<f64>() / cpu_points.len() as f64),
        hub_rss_start_mib: rss_start,
        hub_rss_end_mib: rss_of(hub.pid),
        holds_file_bytes_end: holds_file(&root).0,
    }
}

/// Wait until `hub status` reports `total` sessions again (bodies re-joined).
fn wait_sessions(hub: &Hub, total: usize, budget: Duration) -> Res<Duration> {
    let started = Instant::now();
    while started.elapsed() < budget {
        if hub.sessions_count().ok() == Some(total as u64) {
            return Ok(started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(format!("the hub never reported {total} sessions again within {budget:?}").into())
}

/// Restart the hub with no holds, hold every session, restart again, and
/// check nothing was lost.
fn restart_scenario(hub: &mut Hub, all: &[String], budget: Duration) -> Res<Restart> {
    let root = hub.state().path().to_path_buf();
    let without = hub.restart()?;
    wait_sessions(hub, all.len(), budget)?;

    let hold_all = set_held(&root, all, true)?;
    let (bytes, in_file) = holds_file(&root);
    if in_file != all.len() {
        return Err(format!("{} holds set but the state file holds {in_file}", all.len()).into());
    }
    let with = hub.restart()?;
    let visible_from = Instant::now();
    let mut visible = None;
    while visible_from.elapsed() < budget {
        let held_rows = control::roster_at(&root, false, None)
            .ok()
            .and_then(|d| d["rows"].as_array().map(|r| r.iter().filter(|x| x["hold"] == true).count()))
            .unwrap_or(0);
        if held_rows == all.len() {
            visible = Some(visible_from.elapsed().as_secs_f64());
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let (_, after) = holds_file(&root);
    let sample = all.first().map(|s| control::say_at(&root, s, "x", false, Duration::from_secs(30)));
    let refused = matches!(&sample, Some(Err(ControlError::Refused(e))) if e.code == HELD);
    Ok(Restart {
        holds: all.len(),
        holds_file_bytes: bytes,
        hold_all_secs: hold_all.as_secs_f64(),
        listening_ms_without_holds: without.as_secs_f64() * 1000.0,
        listening_ms_with_holds: with.as_secs_f64() * 1000.0,
        all_held_visible_secs: visible,
        holds_after_restart: after,
        sample_say_refused: refused,
    })
}

pub async fn run(cfg: &Config, hub: &mut Hub, report: &mut Report) -> Res<()> {
    if hub.pid.is_none() {
        return Err("scenario `session-hold` restarts the hub and samples its RSS, which needs a hub this harness started".into());
    }
    let total = cfg.hold_total.max(cfg.hold_sessions);
    let bodies = cfg.bodies.max(1);
    let per_body = total.div_ceil(bodies);
    let total = per_body * bodies;
    let mut members = Vec::new();
    for i in 0..bodies {
        members.push(FleetMember::start(hub, &cfg.holler_bin, &cfg.stub_acp_bin, &format!("hd{i}"), per_body)?);
    }
    let all: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();
    crate::scenario::wait_for_client_count(hub, bodies as u64, Duration::from_secs(60)).await;
    let hub_ref: &Hub = hub;
    tokio::task::block_in_place(|| wait_sessions(hub_ref, total, Duration::from_secs(180)))?;

    let outcome = tokio::task::block_in_place(|| measure(cfg, hub, &all));
    for m in members {
        m.stop();
    }
    let hold = outcome?;
    report.hold = Some(hold);
    Ok(())
}

fn measure(cfg: &Config, hub: &mut Hub, all: &[String]) -> Res<HoldReport> {
    let root = hub.state().path().to_path_buf();
    let driven: Vec<String> = all.iter().take(cfg.hold_sessions).cloned().collect();
    let mut r = HoldReport {
        sessions_total: all.len(),
        sessions_driven: driven.len(),
        phase_seconds: cfg.hold_secs,
        other_load: std::env::var("HOLLER_LOAD_TEST_OTHER_LOAD").unwrap_or_else(|_| "not recorded (set HOLLER_LOAD_TEST_OTHER_LOAD)".into()),
        ..HoldReport::default()
    };
    let none = HashSet::new();

    // Warm every driven session (first prompt spawns its stub-acp).
    for s in &driven {
        let _ = control::say_at(&root, s, "warm", false, Duration::from_secs(60));
    }

    // 1. Baseline twice: the difference is the noise floor.
    r.baseline_runs.push(phase(cfg, hub, "baseline run 1", &driven, &none, cfg.hold_secs));
    r.baseline_runs.push(phase(cfg, hub, "baseline run 2", &driven, &none, cfg.hold_secs));
    for percent in [0usize, 10, 50] {
        let k = driven.len() * percent / 100;
        let held: HashSet<String> = driven.iter().take(k).cloned().collect();
        set_held(&root, &driven[..k], true)?;
        r.held_fractions.push(phase(cfg, hub, &format!("{percent}% held"), &driven, &held, cfg.hold_secs));
        set_held(&root, &driven[..k], false)?;
    }

    // 2. Refusal cost: everything held, a sustained refusal rate.
    set_held(&root, &driven, true)?;
    let held_all: HashSet<String> = driven.iter().cloned().collect();
    r.refusal_only = Some(phase(cfg, hub, "all driven held", &driven, &held_all, cfg.hold_secs));
    set_held(&root, &driven, false)?;

    // 3. Roster scale, with and without every session held.
    let (base_read, rows) = roster_reads(&root, cfg.roster_reads)?;
    r.roster_reads.push(RosterRead { sessions: rows, holds: 0, reads: cfg.roster_reads, latency: base_read.stats() });
    set_held(&root, all, true)?;
    let (held_read, rows) = roster_reads(&root, cfg.roster_reads)?;
    r.roster_reads.push(RosterRead { sessions: rows, holds: all.len(), reads: cfg.roster_reads, latency: held_read.stats() });
    set_held(&root, all, false)?;

    // 4. Churn.
    r.churn = Some(churn(cfg, hub, all, cfg.hold_secs));

    // 5. Restart with many holds.
    r.restart = Some(restart_scenario(hub, all, Duration::from_secs(180))?);

    // Correctness is not a threshold.
    for p in r.baseline_runs.iter().chain(&r.held_fractions).chain(&r.refusal_only) {
        if p.delivered_to_held > 0 {
            return Err(format!("{}: {} say(s) were DELIVERED to a held session", p.label, p.delivered_to_held).into());
        }
    }
    if let Some(x) = &r.restart {
        if x.holds_after_restart != x.holds || !x.sample_say_refused {
            return Err(format!(
                "restart lost hold state: {} held before, {} in the file after, sample say refused: {}",
                x.holds, x.holds_after_restart, x.sample_say_refused
            )
            .into());
        }
    }
    r.thresholds = judge(&r);
    Ok(r)
}

fn t(name: &str, limit: f64, actual: Option<f64>, unit: &str) -> Threshold {
    Threshold { name: name.to_string(), limit, actual, unit: unit.to_string(), passed: actual.is_some_and(|a| a <= limit) }
}

/// The thresholds (issue #444: set from the first measured baseline, not
/// guessed up front; the measured numbers are in `docs/testing.md`). Each is
/// generous by design, a multiple of what one quiet run measured, so that only
/// a real regression trips it and a noisy machine does not.
pub fn judge(r: &HoldReport) -> Vec<Threshold> {
    let p50 = |p: &Phase| p.unheld.map(|s| s.p50_ms);
    let base: Vec<f64> = r.baseline_runs.iter().filter_map(p50).collect();
    let base_mean = (!base.is_empty()).then(|| base.iter().sum::<f64>() / base.len() as f64);
    let noise = (base.len() == 2).then(|| (base[0] - base[1]).abs());
    let at50 = r.held_fractions.iter().find(|p| p.held_percent == 50).and_then(p50);
    let refusal_only = r.refusal_only.as_ref();
    let roster = |holds: bool| r.roster_reads.iter().find(|x| (x.holds > 0) == holds).and_then(|x| x.latency).map(|s| s.p50_ms);
    let churn = r.churn.as_ref();
    let restart = r.restart.as_ref();
    vec![
        t(
            "unheld say p50 with 50% held vs baseline",
            LIMIT_UNHELD_RATIO,
            at50.zip(base_mean).map(|(a, b)| (a - noise.unwrap_or(0.0)).max(0.0) / b.max(0.001)),
            "x",
        ),
        t("refusal p99", LIMIT_REFUSAL_P99_MS, refusal_only.and_then(|p| p.refusals).map(|s| s.p99_ms), "ms"),
        t(
            "refusal p50 as a share of a delivered say's p50",
            LIMIT_REFUSAL_VS_DELIVERY,
            refusal_only.and_then(|p| p.refusals).map(|s| s.p50_ms).zip(base_mean).map(|(a, b)| a / b.max(0.001)),
            "x",
        ),
        t(
            "hub RSS growth across the refusal phase",
            LIMIT_REFUSAL_RSS_MIB,
            refusal_only.and_then(|p| p.hub_rss_start_mib.zip(p.hub_rss_end_mib)).map(|(a, b)| (b - a).max(0.0)),
            "MiB",
        ),
        t("roster read p50, all held vs none held", LIMIT_ROSTER_RATIO, roster(true).zip(roster(false)).map(|(a, b)| a / b.max(0.001)), "x"),
        t("churn hold p99", LIMIT_CHURN_P99_MS, churn.and_then(|c| c.hold_latency).map(|s| s.p99_ms), "ms"),
        t(
            "churn operations that failed",
            LIMIT_CHURN_FAILED_PERCENT,
            churn.map(|c| c.failed_operations as f64 * 100.0 / (c.operations as f64).max(1.0)),
            "%",
        ),
        t(
            "persistence: time per hold when holding every session",
            LIMIT_HOLD_EACH_MS,
            restart.map(|x| x.hold_all_secs * 1000.0 / (x.holds as f64).max(1.0)),
            "ms",
        ),
        t(
            "hub RSS growth after churn and cooldown",
            LIMIT_CHURN_RSS_MIB,
            churn.and_then(|c| c.hub_rss_start_mib.zip(c.hub_rss_end_mib)).map(|(a, b)| (b - a).max(0.0)),
            "MiB",
        ),
        t("restart to listening with all holds persisted", LIMIT_RESTART_LISTENING_MS, restart.map(|x| x.listening_ms_with_holds), "ms"),
        t(
            "restart until every session is back and held",
            LIMIT_RESTART_VISIBLE_SECS,
            restart.map(|x| x.all_held_visible_secs.unwrap_or(f64::INFINITY)),
            "s",
        ),
    ]
}

// The limits. Written down after the first measured run (docs/testing.md,
// "Scenario 5 baseline: session hold"), each several times what that run
// measured on a busy machine (load average 16 to 56 on 10 cores), so that a
// quiet run passes with a wide margin and only a real regression trips one.
// The measured values are beside each.
/// Measured 0.6 (unheld sessions were not slower with half the sessions held).
const LIMIT_UNHELD_RATIO: f64 = 2.0;
/// Measured 2.1 ms.
const LIMIT_REFUSAL_P99_MS: f64 = 100.0;
/// Measured 0.001: a refusal is far cheaper than a delivery.
const LIMIT_REFUSAL_VS_DELIVERY: f64 = 0.5;
/// Measured 0 (RSS fell during the refusal phase).
const LIMIT_REFUSAL_RSS_MIB: f64 = 20.0;
/// Measured 2.8: the reply carries four extra fields per held row.
const LIMIT_ROSTER_RATIO: f64 = 8.0;
/// Measured 473 ms: every hold is a synced write of the state file (about 30 ms
/// each on macOS, where a sync is a full flush), serialised behind one lock.
const LIMIT_CHURN_P99_MS: f64 = 3000.0;
/// Measured 0.16% (5 of 3134 timed out at the 5 s control-socket limit).
const LIMIT_CHURN_FAILED_PERCENT: f64 = 2.0;
/// Measured 8.7 ms.
const LIMIT_HOLD_EACH_MS: f64 = 100.0;
/// Measured 1.6 MiB.
const LIMIT_CHURN_RSS_MIB: f64 = 30.0;
/// Measured 7 ms with 2,001 holds (16 ms with none: process start, not holds).
const LIMIT_RESTART_LISTENING_MS: f64 = 2000.0;
/// Measured 1.0 s.
const LIMIT_RESTART_VISIBLE_SECS: f64 = 60.0;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #444
mod tests {
    use super::*;
    use crate::metrics::LatencyStats;

    fn stats(p50: f64, p99: f64) -> Option<LatencyStats> {
        Some(LatencyStats { count: 10, min_ms: p50, p50_ms: p50, p90_ms: p99, p99_ms: p99, max_ms: p99, mean_ms: p50 })
    }

    fn healthy() -> HoldReport {
        let base = |p50| Phase { unheld: stats(p50, p50 * 2.0), ..Phase::default() };
        HoldReport {
            baseline_runs: vec![base(300.0), base(310.0)],
            held_fractions: vec![Phase { held_percent: 50, unheld: stats(250.0, 500.0), ..Phase::default() }],
            refusal_only: Some(Phase { refusals: stats(0.3, 2.0), hub_rss_start_mib: Some(20.0), hub_rss_end_mib: Some(20.5), ..Phase::default() }),
            roster_reads: vec![
                RosterRead { sessions: 100, holds: 0, reads: 5, latency: stats(10.0, 12.0) },
                RosterRead { sessions: 100, holds: 100, reads: 5, latency: stats(20.0, 30.0) },
            ],
            churn: Some(Churn { operations: 1000, failed_operations: 1, hold_latency: stats(30.0, 400.0), hub_rss_start_mib: Some(20.0), hub_rss_end_mib: Some(22.0), ..Churn::default() }),
            restart: Some(Restart { holds: 1000, hold_all_secs: 9.0, listening_ms_with_holds: 10.0, all_held_visible_secs: Some(1.0), ..Restart::default() }),
            ..HoldReport::default()
        }
    }

    #[test]
    fn a_healthy_run_passes_every_threshold() {
        let r = HoldReport { thresholds: judge(&healthy()), ..healthy() };
        assert!(r.failed().is_empty(), "{:?}", r.failed().iter().map(|t| &t.name).collect::<Vec<_>>());
        assert_eq!(r.thresholds.len(), 11);
    }

    /// The scenario fails when a threshold is exceeded: each one, in turn, made
    /// worse than its limit is reported as failed, and nothing else is.
    #[test]
    fn each_threshold_fails_when_exceeded() {
        type Break = fn(&mut HoldReport);
        let breaks: [(&str, Break); 10] = [
            ("unheld say p50", |r| r.held_fractions[0].unheld = stats(900.0, 900.0)),
            ("refusal p99", |r| r.refusal_only.as_mut().unwrap().refusals = stats(0.3, 500.0)),
            ("refusal p50 as a share", |r| r.refusal_only.as_mut().unwrap().refusals = stats(200.0, 2.0)),
            ("across the refusal phase", |r| r.refusal_only.as_mut().unwrap().hub_rss_end_mib = Some(90.0)),
            ("roster read p50", |r| r.roster_reads[1].latency = stats(200.0, 300.0)),
            ("churn hold p99", |r| r.churn.as_mut().unwrap().hold_latency = stats(30.0, 9000.0)),
            ("churn operations that failed", |r| r.churn.as_mut().unwrap().failed_operations = 500),
            ("time per hold", |r| r.restart.as_mut().unwrap().hold_all_secs = 500.0),
            ("after churn and cooldown", |r| r.churn.as_mut().unwrap().hub_rss_end_mib = Some(200.0)),
            ("restart to listening", |r| r.restart.as_mut().unwrap().listening_ms_with_holds = 9000.0),
        ];
        for (name, worsen) in breaks {
            let mut r = healthy();
            worsen(&mut r);
            let failed: Vec<String> = judge(&r).into_iter().filter(|t| !t.passed).map(|t| t.name).collect();
            assert_eq!(failed.len(), 1, "breaking `{name}` must fail exactly one threshold, got {failed:?}");
            assert!(failed[0].contains(name), "breaking `{name}` failed {failed:?}");
        }
        // A restart that never got every session back and held fails its threshold.
        let mut r = healthy();
        r.restart.as_mut().unwrap().all_held_visible_secs = None;
        assert!(judge(&r).iter().any(|t| !t.passed && t.name.contains("every session is back")));
        // A missing measurement is a failure, not a pass.
        assert!(judge(&HoldReport::default()).iter().all(|t| !t.passed));
    }
}
