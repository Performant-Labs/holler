//! Scenario 2: session scale (issue #371).
//!
//! Fixes N = `cfg.bodies` real `holler body run` processes and ramps M
//! (`cfg.session_ramp`) spawn-mode `stub-acp` sessions per body — 10 → 100 →
//! 500, the issue's own ramp, read literally as *per body* ("M (`stub-acp`
//! sessions per body)"). The real total session count is therefore
//! `bodies * M` at each rung (50 / 500 / 2500 at the default N=5). Each rung
//! restarts all N bodies fresh with that rung's own M, exactly like scenario
//! 1 tears every ramp step down completely before the next — a rung must not
//! inherit the previous rung's sessions.
//!
//! Starting M sessions per body is cheap even at M=500: `SessionManager::
//! start` spawns one lightweight tokio task per session, not a real
//! `stub-acp` process — a driver only spawns the real child on that
//! session's first prompt (`crates/holler-body/src/session_manager.rs`'s own
//! module doc, "restarts the driver on the next prompt … not eagerly"). So a
//! 2500-session rung is 5 real OS processes (the bodies) with 2500 idle
//! in-process tasks, plus a handful of real `stub-acp` children only for the
//! sessions this scenario actually drives a turn on (the presence probes
//! below) — not 2500 real child processes.
//!
//! ## What "fan-out" means on a pull-based hub
//!
//! Holler's wire protocol has no server-push channel from the hub to other
//! bodies or to a CLI reader (confirmed by reading `holler-hub`'s dispatch
//! code: `session/presence` is a body → hub notification, and
//! `roster`/`hub status` are one-shot control-socket reads against a single
//! shared, hub-side table, `holler_hub::roster::Roster`). There is nothing to
//! fan a presence change *out* to per-connection — every reader (this
//! harness, an operator's `holler roster`, a future dashboard) reads the same
//! table, so "every client observes it" is true by construction the instant
//! the table is updated. What is real and measurable is the propagation
//! latency *into* that table: how long after a session's own state genuinely
//! changes (a real `say` driving a real stub-acp turn from `idle` to
//! `working`) does the hub's roster actually reflect it. That is exactly the
//! mechanism issue #359 fixed a real race in (2026-09-21: a body publishing a
//! transient `idle` presence between queued turns), so this scenario's
//! `presence_fanout` measurement doubles as a standing regression guard for
//! that fix, per the issue's own framing.
//!
//! ## The `sessions` invariant is hard-failed, not just reported
//!
//! #371 calls the `hub status --json` `sessions` cross-check a correctness
//! invariant, not a metric with a threshold TBD. A mismatch that survives the
//! poll budget below aborts the whole run with a clear `Err` (mapped to a
//! non-zero process exit by `main`), before any of that rung's numbers are
//! recorded — a passing report must never hide a session count that silently
//! disagreed with reality.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::fleet::FleetMember;
use crate::hub::Hub;
use crate::metrics::{Report, Samples, StepReport};
use crate::proc::CpuMeter;
use crate::scenario::wait_for_client_count;
use crate::{Config, Res};

pub async fn run(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    let hub_pid = hub
        .pid
        .ok_or("scenario `session-scale` samples the hub process, which needs a hub this harness started")?;
    let mut meter = CpuMeter::new(hub_pid);
    let _ = meter.sample();
    tokio::time::sleep(cfg.settle).await;
    report.hub_baseline = meter.sample();
    let baseline = report.hub_baseline;

    // Every body needs a hub-unique label — the token store refuses a
    // second mint under a label already on file (`label "X" already in
    // use`) — so this counts up across rungs rather than restarting at 0
    // each time; a fresh rung's bodies are new processes with new
    // identities, but the label namespace they mint into is the hub's own
    // for the whole run.
    let mut next_body_index: usize = 0;

    for &sessions_per_body in &cfg.session_ramp {
        let step_started = Instant::now();
        let expected_sessions = (cfg.bodies * sessions_per_body) as u64;

        let mut members = Vec::with_capacity(cfg.bodies);
        let mut failures = Vec::new();
        for _ in 0..cfg.bodies {
            let label = format!("ss{next_body_index}");
            next_body_index += 1;
            match FleetMember::start(hub, &cfg.holler_bin, &cfg.stub_acp_bin, &label, sessions_per_body) {
                Ok(m) => members.push(m),
                Err(e) => failures.push(e.to_string()),
            }
        }

        wait_for_client_count(hub, members.len() as u64, Duration::from_secs(60)).await;
        tokio::time::sleep(cfg.settle).await;

        // #371's hard correctness invariant: `hub status --json`'s
        // `sessions` must equal the real total this rung configured. A short
        // poll absorbs the last body's own presence beat still being in
        // flight; a mismatch that survives the budget is a real defect, not
        // a flaky read, and aborts the run rather than being smoothed over.
        let sessions_actual = wait_for_session_count(hub, expected_sessions, Duration::from_secs(60)).await;
        if sessions_actual != Some(expected_sessions) {
            let live = members.len();
            for member in members {
                member.stop();
            }
            return Err(format!(
                "session-scale invariant violated at M={sessions_per_body} sessions/body \
                 (N={} bodies, {live} live, expected total {expected_sessions}): `hub status --json` \
                 reported sessions={}{}",
                cfg.bodies,
                sessions_actual.map_or_else(|| "?".to_string(), |v| v.to_string()),
                if failures.is_empty() {
                    String::new()
                } else {
                    format!(" — body start failures: {}", failures.join("; "))
                },
            )
            .into());
        }

        let session_names: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();

        let (roster_read, fanout) = {
            let holler_bin = cfg.holler_bin.clone();
            let names = session_names;
            let roster_reads = cfg.roster_reads;
            let fanout_samples = cfg.fanout_samples;
            tokio::task::block_in_place(move || {
                let roster_read = measure_roster_read(&holler_bin, hub, roster_reads);
                let fanout = measure_fanout(&holler_bin, hub, &names, fanout_samples);
                (roster_read, fanout)
            })
        };

        let steady = meter.sample();
        report.steps.push(build_step_report(
            cfg,
            hub,
            &members,
            failures,
            baseline,
            steady,
            step_started,
            sessions_per_body,
            expected_sessions,
            sessions_actual,
            &fanout,
            &roster_read,
        ));

        for member in members {
            member.stop();
        }
        wait_for_client_count(hub, 0, Duration::from_secs(30)).await;
    }
    Ok(())
}

/// Build one rung's [`StepReport`]: the rss/threads-over-baseline deltas
/// plus the struct literal itself. Split out of [`run`] so that function
/// stays under this workspace's line-count lint as the shared `StepReport`
/// shape keeps growing new scenario-specific fields.
#[allow(clippy::too_many_arguments)] // #372: split out of `run` purely to satisfy the line-count lint
fn build_step_report(
    cfg: &Config,
    hub: &Hub,
    members: &[FleetMember],
    failures: Vec<String>,
    baseline: crate::metrics::ResourceSample,
    steady: crate::metrics::ResourceSample,
    step_started: Instant,
    sessions_per_body: usize,
    expected_sessions: u64,
    sessions_actual: Option<u64>,
    fanout: &Samples,
    roster_read: &Samples,
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
    StepReport {
        clients_target: cfg.bodies,
        clients_live: members.len(),
        hub_status_clients,
        hub_status_matches: hub_status_clients == Some(members.len() as u64),
        connect_failures: failures,
        ws_connect: None,
        handshake: None,
        connect_total: None,
        calls: None,
        hub_at_steady_state: steady,
        rss_delta_per_client_kib,
        threads_delta,
        step_seconds: step_started.elapsed().as_secs_f64(),
        sessions_per_body: Some(sessions_per_body),
        sessions_expected: Some(expected_sessions),
        sessions_actual,
        sessions_match: Some(sessions_actual == Some(expected_sessions)),
        presence_fanout: fanout.stats(),
        roster_read: roster_read.stats(),
        sustained_seconds: None,
        say_failures: None,
        queue_depth_max: None,
        queue_depth_final: None,
        queue_events_observed: None,
        queue_full_refusals: None,
        rss_series: Vec::new(),
        churn: None,
    }
}

/// Poll `hub status --json`'s `sessions` until it reaches `want` or the
/// budget runs out, returning whatever the last real poll actually saw —
/// `None` only if every single poll itself failed to reach the hub at all
/// (never fabricated when a real read simply disagreed with `want`).
async fn wait_for_session_count(hub: &Hub, want: u64, budget: Duration) -> Option<u64> {
    let deadline = Instant::now() + budget;
    let mut last = None;
    loop {
        last = hub.sessions_count().ok().or(last);
        if last == Some(want) {
            return last;
        }
        if Instant::now() >= deadline {
            return last;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Time `n` real `holler roster --json` invocations against the live hub —
/// the exact CLI path #371 names, not a stand-in for it.
fn measure_roster_read(holler_bin: &Path, hub: &Hub, n: usize) -> Samples {
    let mut samples = Samples::new();
    for _ in 0..n {
        let started = Instant::now();
        let out = Command::new(holler_bin)
            .env("HOLLER_STATE_DIR", hub.state().path())
            .env("HOLLER_DEBUG", "quiet")
            .args(["roster", "--json"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
        if matches!(out, Ok(o) if o.status.success()) {
            samples.push(started.elapsed());
        }
    }
    samples
}

/// Probe `k` sessions, spread across the full session list (and so across
/// every body), for `session/presence` propagation latency.
fn measure_fanout(holler_bin: &Path, hub: &Hub, sessions: &[String], k: usize) -> Samples {
    let mut samples = Samples::new();
    if sessions.is_empty() || k == 0 {
        return samples;
    }
    let step = (sessions.len() / k).max(1);
    let mut idx = 0;
    let mut probed = 0;
    while idx < sessions.len() && probed < k {
        if let Some(d) = probe_one(holler_bin, hub, &sessions[idx]) {
            samples.push(d);
        }
        idx += step;
        probed += 1;
    }
    samples
}

/// Trigger one real state change — a real `say` driving a real `stub-acp`
/// turn from `idle` to `working` — and time how long the hub's own roster
/// table (the single shared source every reader, this harness included,
/// observes; see this module's doc for why that is "fan-out" on a pull-based
/// hub) takes to reflect it.
///
/// Polls the hub's control socket directly (`holler_hub::control::
/// roster_at`), not a `holler roster --json` subprocess per poll: a
/// subprocess spawn (5-20ms on this machine) would itself be a large
/// fraction of the propagation window it is trying to measure. `None` if the
/// turn's `working` window closed before any poll landed, or the `say`
/// could not even be launched — a miss is reported as missing, never
/// fabricated as zero.
fn probe_one(holler_bin: &Path, hub: &Hub, session: &str) -> Option<Duration> {
    let started = Instant::now();
    let mut say = Command::new(holler_bin)
        .env("HOLLER_STATE_DIR", hub.state().path())
        .env("HOLLER_DEBUG", "quiet")
        .args(["say", session, "load fanout probe"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = started + Duration::from_secs(2);
    let mut observed = None;
    while Instant::now() < deadline {
        if let Ok(doc) = holler_hub::control::roster_at(hub.state().path(), false, None) {
            if session_is_not_idle(&doc, session) {
                observed = Some(started.elapsed());
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    // Reap the `say` child regardless of whether the probe observed the
    // flip — the turn finishes on its own either way, and an unreaped child
    // would violate this harness's own "no orphaned processes" rule.
    let _ = say.wait();
    observed
}

/// Does the roster's `{rows:[...]}` document show `session` in any state
/// other than `idle`? The A2A state names are `idle`/`working`/
/// `input-required` (`holler_proto::docs::SessionState`).
fn session_is_not_idle(doc: &serde_json::Value, session: &str) -> bool {
    doc.get("rows")
        .and_then(|r| r.as_array())
        .into_iter()
        .flatten()
        .any(|row| {
            row.get("name").and_then(|n| n.as_str()) == Some(session)
                && row.get("state").and_then(|s| s.as_str()).is_some_and(|s| s != "idle")
        })
}
