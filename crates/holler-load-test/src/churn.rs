//! Scenario 4: churn (issue #373).
//!
//! Two measurements:
//!
//! 1. **Churn cycles.** `--churn-cycles` times: start `--bodies` real `holler
//!    body run` processes (each hosting `--sessions-per-body` spawn-mode
//!    `stub-acp` sessions), wait until the hub reports every one of them,
//!    drive one real `say` per session, then detach and kill them all. After
//!    every cycle, `hub status --json`'s `clients` and `sessions` must return
//!    to their pre-run baseline. That is #373's correctness invariant, so a
//!    cycle that doesn't get there within the budget aborts the run with a
//!    non-zero exit instead of being reported as a slow number.
//!
//! 2. **Dead-backend detection.** The 2026-09-21 incident #373 cites: an
//!    attach-mode backend (`opencode`) killed from outside Holler, while
//!    `holler roster` kept showing its session `connected`/`idle`. This
//!    reproduces it with a real process: the harness re-executes itself in
//!    `--fake-opencode` mode (the fake OpenCode HTTP server
//!    `holler-body`'s own attach tests use), attaches a body session to it,
//!    `SIGKILL`s that process, and times how long the roster takes to stop
//!    showing the session healthy. Not detecting it within the window is a
//!    valid, recorded result, not a harness failure: it's the bug.
//!
//! Timings come from polling `hub status --json` / the roster, so their
//! resolution is the poll interval plus one CLI round trip (tens of ms).

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::fleet::FleetMember;
use crate::hub::{own_process_group, Hub};
use crate::metrics::{ChurnReport, DeadBackendProbe, Report, ResourceSample, Samples, StepReport};
use crate::proc::CpuMeter;
use crate::{kill_tree, Config, Res};

const REGISTER_BUDGET: Duration = Duration::from_secs(60);
const CLEANUP_BUDGET: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(25);

pub async fn run(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    let started = Instant::now();
    let mut meter = hub.pid.map(CpuMeter::new);
    report.hub_baseline = sample(&mut meter);
    let rss_before = report.hub_baseline.rss_mib;
    let baseline = counts(hub).ok_or("could not read `hub status --json` before the first cycle")?;

    let mut join = Samples::new();
    let mut say = Samples::new();
    let mut say_failures = 0;
    let mut detach = Samples::new();
    let mut cleanup = Samples::new();
    for cycle in 0..cfg.churn_cycles {
        let t = run_cycle(cfg, hub, cycle, baseline).await?;
        join.push(t.join);
        say_failures += t.say_failures;
        for d in t.say {
            say.push(d);
        }
        detach.push(t.detach);
        cleanup.push(t.cleanup);
    }

    let steady = sample(&mut meter);
    let dead_backend = tokio::task::block_in_place(|| dead_backend_probe(cfg, hub))?;
    let final_counts = counts(hub);

    report.steps.push(step_report(
        cfg,
        started,
        steady,
        final_counts,
        baseline,
        ChurnReport {
            cycles_target: cfg.churn_cycles,
            cycles_completed: cfg.churn_cycles,
            bodies_per_cycle: cfg.bodies,
            sessions_per_body: cfg.sessions_per_body,
            join_to_registered: join.stats(),
            say: say.stats(),
            say_failures,
            detach: detach.stats(),
            cleanup: cleanup.stats(),
            hub_rss_before_mib: rss_before,
            hub_rss_after_mib: steady.rss_mib,
            dead_backend: Some(dead_backend),
        },
    ));
    Ok(())
}

struct CycleTimes {
    join: Duration,
    say: Vec<Duration>,
    say_failures: usize,
    detach: Duration,
    cleanup: Duration,
}

/// One join → run → detach cycle. Errors (aborting the run) if the hub
/// never registers the wave, or never returns to `baseline` afterwards.
async fn run_cycle(cfg: &Config, hub: &Hub, cycle: usize, baseline: (u64, u64)) -> Res<CycleTimes> {
    let joined_at = Instant::now();
    let mut members = Vec::with_capacity(cfg.bodies);
    for i in 0..cfg.bodies {
        match FleetMember::start(hub, &cfg.holler_bin, &cfg.stub_acp_bin, &format!("ch{cycle}-{i}"), cfg.sessions_per_body) {
            Ok(m) => members.push(m),
            Err(e) => {
                stop_all(members);
                return Err(format!("churn cycle {cycle}: body {i} failed to start: {e}").into());
            }
        }
    }

    let want = (baseline.0 + cfg.bodies as u64, baseline.1 + (cfg.bodies * cfg.sessions_per_body) as u64);
    let seen = wait_for_counts(hub, want, REGISTER_BUDGET).await;
    if seen != Some(want) {
        stop_all(members);
        return Err(format!("churn cycle {cycle}: hub never registered the wave; wanted {} , saw {}", fmt(Some(want)), fmt(seen)).into());
    }
    let join = joined_at.elapsed();

    let names: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();
    let (say, say_failures) = tokio::task::block_in_place(|| say_all(&cfg.holler_bin, hub, &names));

    let detach_started = Instant::now();
    stop_all(members);
    let detach = detach_started.elapsed();
    let killed_at = Instant::now();
    let seen = wait_for_counts(hub, baseline, CLEANUP_BUDGET).await;
    if seen != Some(baseline) {
        return Err(format!(
            "churn cycle {cycle}: hub did not return to baseline within {}s after every body \
             was detached and killed (leaked state); wanted {}, saw {}",
            CLEANUP_BUDGET.as_secs(),
            fmt(Some(baseline)),
            fmt(seen),
        )
        .into());
    }
    Ok(CycleTimes { join, say, say_failures, detach, cleanup: killed_at.elapsed() })
}

fn stop_all(members: Vec<FleetMember>) {
    for m in members {
        m.stop();
    }
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
async fn wait_for_counts(hub: &Hub, want: (u64, u64), budget: Duration) -> Option<(u64, u64)> {
    let deadline = Instant::now() + budget;
    let mut last = None;
    loop {
        last = counts(hub).or(last);
        if last == Some(want) || Instant::now() >= deadline {
            return last;
        }
        tokio::time::sleep(POLL).await;
    }
}

fn fmt(c: Option<(u64, u64)>) -> String {
    c.map_or_else(|| "unreadable".to_string(), |(cl, s)| format!("clients={cl} sessions={s}"))
}

fn sample(meter: &mut Option<CpuMeter>) -> ResourceSample {
    meter.as_mut().map(CpuMeter::sample).unwrap_or_default()
}

/// Start a body attached to a real fake-OpenCode process, kill that process,
/// and time how long the roster takes to stop showing the session healthy.
fn dead_backend_probe(cfg: &Config, hub: &Hub) -> Res<DeadBackendProbe> {
    let (mut backend, endpoint) = spawn_fake_opencode()?;
    let member = match FleetMember::start_attach(hub, &cfg.holler_bin, "churn-dead", "attached", &endpoint, "ses_churn") {
        Ok(m) => m,
        Err(e) => {
            kill_tree(&mut backend);
            return Err(e);
        }
    };
    let name = member.session_names[0].clone();

    let healthy_by = Instant::now() + REGISTER_BUDGET;
    while !is_healthy(&roster_row(hub, &name)) {
        if Instant::now() >= healthy_by {
            kill_tree(&mut backend);
            member.stop();
            return Err(format!("dead-backend probe: attach session never showed healthy; roster: {}", describe(&roster_row(hub, &name))).into());
        }
        std::thread::sleep(POLL);
    }

    kill_tree(&mut backend);
    let killed_at = Instant::now();
    let mut detection_ms = None;
    let mut row = roster_row(hub, &name);
    while killed_at.elapsed() < cfg.dead_backend_window {
        row = roster_row(hub, &name);
        if !is_healthy(&row) {
            detection_ms = Some(killed_at.elapsed().as_secs_f64() * 1000.0);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    member.stop();
    Ok(DeadBackendProbe {
        window_secs: cfg.dead_backend_window.as_secs(),
        detected: detection_ms.is_some(),
        detection_ms,
        roster_after: describe(&row),
    })
}

/// Re-execute this binary in `--fake-opencode` mode and read the endpoint
/// it prints.
fn spawn_fake_opencode() -> Res<(Child, String)> {
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.arg("--fake-opencode").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    own_process_group(&mut cmd);
    let mut child = cmd.spawn()?;
    let mut line = String::new();
    let read = child.stdout.take().map(|s| BufReader::new(s).read_line(&mut line));
    if !matches!(read, Some(Ok(n)) if n > 0) {
        kill_tree(&mut child);
        return Err("fake OpenCode process exited before printing its endpoint".into());
    }
    Ok((child, line.trim().to_string()))
}

/// The session's roster row, including `gone` rows, or `None` if absent.
fn roster_row(hub: &Hub, name: &str) -> Option<serde_json::Value> {
    let doc = holler_hub::control::roster_at(hub.state().path(), true, None).ok()?;
    doc.get("rows")?
        .as_array()?
        .iter()
        .find(|r| r.get("name").and_then(|n| n.as_str()) == Some(name))
        .cloned()
}

/// Healthy = present, connected, and idle or working: exactly what the
/// 2026-09-21 roster kept (wrongly) showing for a dead backend.
fn is_healthy(row: &Option<serde_json::Value>) -> bool {
    row.as_ref().is_some_and(|r| {
        r.get("conn_state").and_then(|c| c.as_str()) == Some("connected")
            && matches!(r.get("state").and_then(|s| s.as_str()), Some("idle" | "working"))
    })
}

fn describe(row: &Option<serde_json::Value>) -> String {
    row.as_ref().map_or_else(
        || "row absent".to_string(),
        |r| {
            format!(
                "state={} conn_state={}",
                r.get("state").and_then(|s| s.as_str()).unwrap_or("?"),
                r.get("conn_state").and_then(|s| s.as_str()).unwrap_or("?"),
            )
        },
    )
}

fn step_report(
    cfg: &Config,
    started: Instant,
    steady: ResourceSample,
    final_counts: Option<(u64, u64)>,
    baseline: (u64, u64),
    churn: ChurnReport,
) -> StepReport {
    let clients = final_counts.map(|c| c.0);
    StepReport {
        clients_target: cfg.bodies,
        clients_live: 0,
        hub_status_clients: clients,
        hub_status_matches: clients == Some(baseline.0),
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
