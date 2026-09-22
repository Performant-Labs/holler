//! Scenario 1: connection scale (issue #370).
//!
//! Ramps N concurrent connections against one real hub and, at each rung,
//! measures the four things #370 names: WS connect + full handshake latency
//! per connection, hub RSS, hub thread count, and whether `hub status
//! --json`'s `clients` still equals N.
//!
//! **No thresholds are asserted.** #370 is explicit that they are "TBD from
//! the first real baseline run", so this scenario's job is to produce that
//! baseline honestly — it fails only on a harness error (a client that could
//! not connect at all, a hub that died), never on a latency being "too high",
//! because there is not yet a measured basis for such a number.

use std::time::{Duration, Instant};

use crate::hub::Hub;
use crate::metrics::{Report, Samples, StepReport};
use crate::proc::CpuMeter;
use crate::wire::{self, ClientIdentity};
use crate::{Config, Res};

/// One connected client's handle, held so the connection stays open for the
/// whole step.
struct LiveClient {
    task: tokio::task::JoinHandle<Res<()>>,
}

/// The outcome of one client's connect attempt.
enum Attempt {
    Live(wire::ConnectTimings, LiveClient),
    Failed(String),
}

pub async fn run(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    let hub_pid = hub.pid.ok_or(
        "scenario `connection-scale` samples the hub process's own RSS/threads, which needs a hub \
         this harness started — rerun without `--hub-url`, or accept that resource numbers are \
         unavailable for an attached hub",
    )?;
    let mut meter = CpuMeter::new(hub_pid);
    // Establish the CPU baseline and the zero-client resource floor every
    // per-client delta below is measured against.
    let _ = meter.sample();
    tokio::time::sleep(cfg.settle).await;
    let baseline = meter.sample();
    report.hub_baseline = baseline;

    let hub_pubkey = hub.x25519_pubkey()?;
    let mut next_client_index: usize = 0;

    for &target in &cfg.ramp {
        let step_started = Instant::now();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // Pre-provision every token for this rung *before* the connect burst:
        // the token store is a flock-guarded file, so minting inside the burst
        // would measure the file lock, not the hub's connection path.
        let mut provisioned = Vec::with_capacity(target);
        for _ in 0..target {
            let index = next_client_index;
            next_client_index += 1;
            let identity = ClientIdentity::derive(index as u64);
            let label = format!("ld{index}");
            let token_id = hub.provision(&label, &identity)?;
            provisioned.push((label, token_id, identity));
        }

        let attempts = connect_burst(cfg, hub, &hub_pubkey, provisioned, &shutdown_rx).await;

        let mut ws_connect = Samples::new();
        let mut handshake = Samples::new();
        let mut connect_total = Samples::new();
        let mut failures = Vec::new();
        let mut live = Vec::new();
        for attempt in attempts {
            match attempt {
                Attempt::Live(t, client) => {
                    ws_connect.push(t.ws_connect);
                    handshake.push(t.handshake);
                    connect_total.push(t.total());
                    live.push(client);
                }
                Attempt::Failed(reason) => failures.push(reason),
            }
        }

        // Let the hub reach steady state before sampling it: a sample taken
        // during the connect burst measures the burst, not the steady state
        // #370 asks for.
        tokio::time::sleep(cfg.settle).await;
        let steady = meter.sample();
        let hub_status_clients = hub.client_count().ok();

        let rss_delta_per_client_kib = match (baseline.rss_mib, steady.rss_mib) {
            (Some(b), Some(s)) if !live.is_empty() => Some(((s - b) * 1024.0) / live.len() as f64),
            _ => None,
        };
        let threads_delta = match (baseline.threads, steady.threads) {
            (Some(b), Some(s)) => Some(s as i64 - b as i64),
            _ => None,
        };

        report.steps.push(StepReport {
            clients_target: target,
            clients_live: live.len(),
            hub_status_clients,
            hub_status_matches: hub_status_clients == Some(live.len() as u64),
            connect_failures: failures,
            ws_connect: ws_connect.stats(),
            handshake: handshake.stats(),
            connect_total: connect_total.stats(),
            calls: None,
            hub_at_steady_state: steady,
            rss_delta_per_client_kib,
            threads_delta,
            step_seconds: step_started.elapsed().as_secs_f64(),
        });

        // Tear this rung down completely before the next one: #370's ramp is
        // 1 → 50 → 200 as independent measurements, so a rung must not inherit
        // the previous rung's connections (or its memory).
        let _ = shutdown_tx.send(true);
        for client in live {
            if let Ok(Err(e)) = tokio::time::timeout(Duration::from_secs(10), client.task).await {
                eprintln!("note: a client reported {e} during teardown");
            }
        }
        wait_for_client_count(hub, 0, Duration::from_secs(30)).await;
    }
    Ok(())
}

/// Open `provisioned.len()` connections, at most `cfg.connect_concurrency`
/// handshakes in flight at once, and keep every successful one live.
///
/// The concurrency bound is not politeness: the hub caps *unauthenticated*
/// connections at `HOLLER_MAX_PREAUTH_CONNECTIONS` (default 64,
/// `holler_hub::hygiene`) and refuses anything over it with WebSocket code
/// 1013. Firing 200 dials at once would therefore measure that cap rather than
/// connection scale. The harness surfaces the bound as a flag so a run can
/// deliberately exceed it and observe the refusal, which is itself a real
/// result.
async fn connect_burst(
    cfg: &Config,
    hub: &Hub,
    hub_pubkey: &[u8; 32],
    provisioned: Vec<(String, String, ClientIdentity)>,
    shutdown_rx: &tokio::sync::watch::Receiver<bool>,
) -> Vec<Attempt> {
    let permits = std::sync::Arc::new(tokio::sync::Semaphore::new(cfg.connect_concurrency.max(1)));
    let mut joins = Vec::with_capacity(provisioned.len());

    for (label, token_id, identity) in provisioned {
        let permits = std::sync::Arc::clone(&permits);
        let ws_url = hub.ws_url.clone();
        let hub_pubkey = *hub_pubkey;
        let heartbeat = cfg.heartbeat;
        let mut shutdown = shutdown_rx.clone();
        joins.push(tokio::spawn(async move {
            let Ok(permit) = permits.acquire().await else {
                return Attempt::Failed("harness semaphore closed".to_string());
            };
            let connected = wire::connect_and_handshake(&ws_url, &token_id, &identity, &hub_pubkey, &label).await;
            drop(permit);
            match connected {
                Err(e) => Attempt::Failed(e.to_string()),
                Ok((ws, timings)) => {
                    let task = tokio::spawn(async move {
                        wire::hold_live(ws, &label, heartbeat, &mut shutdown).await
                    });
                    Attempt::Live(timings, LiveClient { task })
                }
            }
        }));
    }

    let mut attempts = Vec::with_capacity(joins.len());
    for join in joins {
        match join.await {
            Ok(a) => attempts.push(a),
            Err(e) => attempts.push(Attempt::Failed(format!("harness task failed: {e}"))),
        }
    }
    attempts
}

/// #369's process-fleet half: real `holler body run` processes, each hosting
/// M spawn-mode `stub-acp` sessions, driven at a controlled request rate.
///
/// Reported as a single ramp step (the fleet size), so the same report
/// document and human table serve both scenarios. This is the mode scenarios
/// #371-#373 are expected to build on; it is implemented and verified here
/// rather than sketched, but this issue's own baseline is scenario 1's.
pub async fn run_body_fleet(cfg: &Config, hub: &Hub, report: &mut Report) -> Res<()> {
    let hub_pid = hub
        .pid
        .ok_or("scenario `body-fleet` samples the hub process, which needs a hub this harness started")?;
    let mut meter = CpuMeter::new(hub_pid);
    let _ = meter.sample();
    tokio::time::sleep(cfg.settle).await;
    report.hub_baseline = meter.sample();
    let baseline = report.hub_baseline;

    let step_started = Instant::now();
    let mut members = Vec::with_capacity(cfg.bodies);
    let mut failures = Vec::new();
    for i in 0..cfg.bodies {
        match crate::fleet::FleetMember::start(hub, &cfg.holler_bin, &cfg.stub_acp_bin, &format!("lb{i}"), cfg.sessions_per_body) {
            Ok(m) => members.push(m),
            Err(e) => failures.push(e.to_string()),
        }
    }

    // Wait for every body to actually register before driving traffic: a
    // `say` to a session the hub has not seen yet is `unknown session`, which
    // would be recorded as a failed call rather than as the startup race it
    // really is (the same first-presence race `load_roster_scale_test.rs`
    // documents at length).
    wait_for_client_count(hub, members.len() as u64, Duration::from_secs(60)).await;
    tokio::time::sleep(cfg.settle).await;

    let sessions: Vec<String> = members.iter().flat_map(|m| m.session_names.clone()).collect();
    let calls = {
        let holler_bin = cfg.holler_bin.clone();
        let rate = cfg.rate;
        let total = cfg.calls;
        tokio::task::block_in_place(|| crate::fleet::drive_calls(&holler_bin, hub, &sessions, rate, total))
    };

    let steady = meter.sample();
    let hub_status_clients = hub.client_count().ok();
    let rss_delta_per_client_kib = match (baseline.rss_mib, steady.rss_mib) {
        (Some(b), Some(s)) if !members.is_empty() => Some(((s - b) * 1024.0) / members.len() as f64),
        _ => None,
    };
    let threads_delta = match (baseline.threads, steady.threads) {
        (Some(b), Some(s)) => Some(s as i64 - b as i64),
        _ => None,
    };

    report.steps.push(StepReport {
        clients_target: cfg.bodies,
        clients_live: members.len(),
        hub_status_clients,
        hub_status_matches: hub_status_clients == Some(members.len() as u64),
        connect_failures: failures,
        ws_connect: None,
        handshake: None,
        connect_total: None,
        calls: calls.stats(),
        hub_at_steady_state: steady,
        rss_delta_per_client_kib,
        threads_delta,
        step_seconds: step_started.elapsed().as_secs_f64(),
    });

    for member in members {
        member.stop();
    }
    Ok(())
}

/// Poll `hub status --json` until `clients` reaches `want` (or the budget
/// runs out). Used between rungs so a rung starts from a genuinely empty hub
/// rather than from whatever the previous teardown had not finished reaping.
async fn wait_for_client_count(hub: &Hub, want: u64, budget: Duration) {
    let deadline = Instant::now() + budget;
    loop {
        if hub.client_count().ok() == Some(want) {
            return;
        }
        if Instant::now() >= deadline {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
