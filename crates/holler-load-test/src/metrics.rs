//! Latency samples, their summary statistics, and the report documents the
//! harness emits (issue #369 harness bullet 5: "JSON + a human table").
//!
//! Nothing here measures anything itself — [`Samples`] is a plain collector the
//! scenarios push into, and [`Report`] is the document `main` serializes. The
//! split keeps the measurement code (`wire.rs`, `fleet.rs`, `proc.rs`) free of
//! any opinion about how a number is later summarized or rendered.

use std::time::Duration;

use serde::Serialize;

/// A collected set of per-call durations, in the order they were observed.
///
/// Deliberately keeps every sample rather than a streaming digest: the
/// scenarios here run in the thousands of calls, not the millions, so exact
/// percentiles cost nothing and an approximate digest would be one more thing
/// a reader has to trust.
#[derive(Debug, Default, Clone)]
pub struct Samples {
    micros: Vec<u64>,
}

impl Samples {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one observation.
    pub fn push(&mut self, d: Duration) {
        self.micros.push(d.as_micros().min(u128::from(u64::MAX)) as u64);
    }

    /// Summarize. `None` for an empty collector — a statistic over nothing is
    /// a fabricated number, and this harness exists to not publish those.
    pub fn stats(&self) -> Option<LatencyStats> {
        if self.micros.is_empty() {
            return None;
        }
        let mut sorted = self.micros.clone();
        sorted.sort_unstable();
        let sum: u128 = sorted.iter().map(|v| u128::from(*v)).sum();
        let n = sorted.len();
        Some(LatencyStats {
            count: n,
            min_ms: ms(percentile_at(&sorted, 0.0)),
            p50_ms: ms(percentile_at(&sorted, 0.50)),
            p90_ms: ms(percentile_at(&sorted, 0.90)),
            p99_ms: ms(percentile_at(&sorted, 0.99)),
            max_ms: ms(percentile_at(&sorted, 1.0)),
            mean_ms: ms((sum / n as u128).min(u128::from(u64::MAX)) as u64),
        })
    }
}

/// The nearest-rank percentile of an already-sorted slice. `q` is clamped to
/// `[0,1]` and the index to the slice's bounds, so this cannot panic and
/// cannot index out of range — callers only ever reach it with a non-empty
/// slice (see [`Samples::stats`]).
fn percentile_at(sorted: &[u64], q: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let q = q.clamp(0.0, 1.0);
    let last = sorted.len() - 1;
    let idx = ((q * last as f64).round() as usize).min(last);
    sorted.get(idx).copied().unwrap_or(0)
}

/// Microseconds → milliseconds, to 3 decimal places (so a sub-millisecond
/// loopback handshake is still a real number, not `0`).
fn ms(micros: u64) -> f64 {
    (micros as f64) / 1000.0
}

/// The summary of one [`Samples`] collector.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct LatencyStats {
    pub count: usize,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
}

/// One observation of the hub process's own resource usage (see `proc.rs` for
/// how each field is obtained and which platforms can report it).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ResourceSample {
    /// Resident set size, MiB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_mib: Option<f64>,
    /// OS threads in the hub process.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
    /// Open file descriptors (Linux only — see `proc.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_fds: Option<usize>,
    /// CPU utilisation over the interval since the previous sample, as a
    /// percentage of one core (so >100 is real on a multi-core box).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f64>,
}

/// One RSS/thread observation taken at a known offset into a sustained run
/// (`sustained_throughput.rs`, issue #372) — a single before/after snapshot
/// cannot show "flat after warmup" vs "monotonic growth"; a series can.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RssPoint {
    /// Seconds since the sustained run's load-driving phase started.
    pub elapsed_s: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_mib: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<usize>,
}

/// One rung of a ramp: everything measured at a given concurrent-client count.
#[derive(Debug, Clone, Serialize)]
pub struct StepReport {
    /// The concurrent-client count this step ramped to.
    pub clients_target: usize,
    /// How many clients actually completed the full handshake and stayed up.
    pub clients_live: usize,
    /// What `hub status --json` reported for `clients` at steady state.
    pub hub_status_clients: Option<u64>,
    /// `clients_live == hub_status_clients` — #370's "no silent drops" check.
    pub hub_status_matches: bool,
    /// Clients the hub refused during the connect burst, by refusal reason.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connect_failures: Vec<String>,
    /// TCP connect + WebSocket upgrade, per connection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_connect: Option<LatencyStats>,
    /// `circuit/authenticate` → `circuit/prove` → `circuit/hello`, per
    /// connection (the full handshake #370 names, excluding the WS dial above).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handshake: Option<LatencyStats>,
    /// WS dial + handshake together — what a body experiences as "time to
    /// live connection".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_total: Option<LatencyStats>,
    /// Per-call latency of the driven `say`/`interrupt`/`roster` traffic, if
    /// this step drove any (body mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calls: Option<LatencyStats>,
    /// The hub's resource usage once this step reached steady state.
    pub hub_at_steady_state: ResourceSample,
    /// Hub RSS attributable to this step's clients, over the idle baseline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_delta_per_client_kib: Option<f64>,
    /// Hub threads attributable to this step's clients, over the baseline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads_delta: Option<i64>,
    /// Wall-clock the whole step took.
    pub step_seconds: f64,
    /// `session-scale` (#371): stub-acp sessions per body this rung ramped
    /// to (10 / 100 / 500 — the issue's own ramp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions_per_body: Option<usize>,
    /// `session-scale`: the real total session count this rung configured
    /// (`bodies * sessions_per_body`) — the invariant `hub status --json`'s
    /// `sessions` must equal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions_expected: Option<u64>,
    /// `session-scale`: what `hub status --json` actually reported for
    /// `sessions` once the rung settled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions_actual: Option<u64>,
    /// `session-scale`: `sessions_expected == sessions_actual` — #371's hard
    /// correctness invariant, not just a reported metric. A run whose last
    /// poll still disagreed never reaches this struct: the scenario returns
    /// an error and the process exits non-zero instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions_match: Option<bool>,
    /// `session-scale`: `session/presence` propagation latency — real time
    /// from triggering a session's own state change to the hub's shared
    /// roster table reflecting it (see `session_scale.rs`'s module doc for
    /// why that is the honest reading of "fan-out" on a pull-based hub).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_fanout: Option<LatencyStats>,
    /// `session-scale`: `holler roster --json` read latency at this rung's
    /// session count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roster_read: Option<LatencyStats>,
    /// `sustained-throughput` (#372): how long, in seconds, the sustained
    /// `say --queue` drive itself ran (excludes warmup settle and the
    /// post-load cooldown — `step_seconds` above covers the whole step).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sustained_seconds: Option<f64>,
    /// `sustained-throughput`: real `say --queue` calls the driver could not
    /// complete (a `-32009`/`-32007`/transport failure) — excluded from
    /// `calls`'s latency distribution, never smoothed into it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub say_failures: Option<usize>,
    /// `sustained-throughput`: the largest total FIFO queue depth (summed
    /// across every session) observed at any point during the run, from the
    /// real `queue_enqueue`/`queue_dequeue` debug events — see
    /// `fleet.rs::QueueEvent`. `None` when no fleet member's stderr was
    /// watched (every other scenario).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_depth_max: Option<usize>,
    /// `sustained-throughput`: the total FIFO queue depth at the run's very
    /// last observed queue event (after the post-load cooldown) — 0 means
    /// every session's queue fully drained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_depth_final: Option<usize>,
    /// `sustained-throughput`: how many `queue_enqueue`/`queue_dequeue`
    /// events were observed in total (the series' own sample count).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_events_observed: Option<usize>,
    /// `sustained-throughput`: how many `--queue` calls were refused
    /// outright with `queue_full` (the FIFO's hard `QUEUE_CAP` was already
    /// reached for that session) — a real, bounded-by-design ceiling, not a
    /// harness failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_full_refusals: Option<usize>,
    /// `sustained-throughput`: the hub's own RSS/thread count sampled
    /// repeatedly across the sustained run (see `RssPoint`), so "flat after
    /// warmup" vs "monotonic growth" is a real series, not one snapshot.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub rss_series: Vec<RssPoint>,
    /// `churn` (#373): everything that scenario measures, in one place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub churn: Option<ChurnReport>,
}

/// Scenario 4 (#373): repeated join → run → detach cycles, plus the
/// dead-backend detection probe.
#[derive(Debug, Clone, Serialize)]
pub struct ChurnReport {
    pub cycles_target: usize,
    /// Every completed cycle returned the hub to baseline; a cycle that did
    /// not aborts the run, so this equals `cycles_target` in any report.
    pub cycles_completed: usize,
    pub bodies_per_cycle: usize,
    pub sessions_per_body: usize,
    /// Starting a wave → the hub reporting every body and session, per cycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_to_registered: Option<LatencyStats>,
    /// One real `say` per session per cycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub say: Option<LatencyStats>,
    pub say_failures: usize,
    /// The harness detaching and killing every body in a cycle, run
    /// sequentially (`holler body detach`, then SIGKILL of its process group).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detach: Option<LatencyStats>,
    /// Last body killed → `hub status --json` `clients` and `sessions` both
    /// back to baseline, per cycle: the hub's own cleanup latency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup: Option<LatencyStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_rss_before_mib: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_rss_after_mib: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dead_backend: Option<DeadBackendProbe>,
}

/// An attach-mode session whose backend process is killed from outside
/// Holler: how long until the hub's roster stops showing it healthy.
#[derive(Debug, Clone, Serialize)]
pub struct DeadBackendProbe {
    pub window_secs: u64,
    pub detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detection_ms: Option<f64>,
    /// The session's roster row (state/conn) when detection fired, or when
    /// the window ran out.
    pub roster_after: String,
}

/// The whole run.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub scenario: String,
    pub issue: String,
    /// RFC-3339-ish UTC timestamp of the run.
    pub started_at: String,
    pub host: HostInfo,
    pub config: serde_json::Value,
    /// The hub's resource usage with zero clients connected — every
    /// `*_delta_*` field is relative to this.
    pub hub_baseline: ResourceSample,
    pub steps: Vec<StepReport>,
}

/// Where the numbers were measured. #369 is explicit that a load number
/// without its machine is not a result ("record what else was running").
#[derive(Debug, Clone, Serialize)]
pub struct HostInfo {
    pub os: String,
    pub arch: String,
    pub logical_cpus: Option<usize>,
}

impl HostInfo {
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            logical_cpus: std::thread::available_parallelism().ok().map(|n| n.get()),
        }
    }
}

/// Render the report as the human table `docs/testing.md`'s conventions call
/// for — plain ASCII, one row per ramp step, no colour and no box-drawing
/// characters (this output is pasted into issues and CI logs).
pub fn human_table(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "scenario: {} ({})\nhost: {} {} ({} logical cpus)\nstarted: {}\n",
        report.scenario,
        report.issue,
        report.host.os,
        report.host.arch,
        report
            .host
            .logical_cpus
            .map_or_else(|| "?".to_string(), |n| n.to_string()),
        report.started_at,
    ));
    out.push_str(&format!(
        "hub baseline (0 clients): rss={} threads={} fds={}\n\n",
        opt_f(report.hub_baseline.rss_mib, "MiB"),
        opt_u(report.hub_baseline.threads),
        opt_u(report.hub_baseline.open_fds),
    ));

    out.push_str(&format!(
        "{:>7} {:>7} {:>8} {:>9} {:>9} {:>9} {:>9} {:>8} {:>8} {:>7} {:>8}\n",
        "target", "live", "status", "hs p50", "hs p90", "hs p99", "hs max", "rss", "thr", "cpu%", "step s",
    ));
    out.push_str(&format!("{}\n", "-".repeat(101)));
    for step in &report.steps {
        let hs = step.handshake;
        out.push_str(&format!(
            "{:>7} {:>7} {:>8} {:>9} {:>9} {:>9} {:>9} {:>8} {:>8} {:>7} {:>8}\n",
            step.clients_target,
            step.clients_live,
            step.hub_status_clients
                .map_or_else(|| "?".to_string(), |c| format!("{c}{}", if step.hub_status_matches { "" } else { "!" })),
            hs.map_or_else(|| "-".to_string(), |s| format!("{:.1}", s.p50_ms)),
            hs.map_or_else(|| "-".to_string(), |s| format!("{:.1}", s.p90_ms)),
            hs.map_or_else(|| "-".to_string(), |s| format!("{:.1}", s.p99_ms)),
            hs.map_or_else(|| "-".to_string(), |s| format!("{:.1}", s.max_ms)),
            opt_f(step.hub_at_steady_state.rss_mib, ""),
            opt_u(step.hub_at_steady_state.threads),
            opt_f(step.hub_at_steady_state.cpu_percent, ""),
            format!("{:.1}", step.step_seconds),
        ));
    }
    out.push_str("\n(hs = full circuit handshake, ms: authenticate -> prove -> hello.\n");
    out.push_str(" status: hub status --json's `clients`; a trailing `!` means it did NOT match `live`.)\n");

    for step in &report.steps {
        out.push_str(&step_extra_lines(step));
    }
    out
}

/// The per-step detail lines under the main table: everything a step
/// measured that the fixed-width table above has no column for (rss/client,
/// connect failures, driven-call latency, session-scale's own rows, and
/// scenario 3's sustained-throughput rows — sustained drive time, `say`
/// latency, queue depth, and the RSS-over-time series). Split out of
/// [`human_table`] itself so that function stays under this workspace's
/// line-count/cognitive-complexity lints as scenarios keep adding their own
/// rows here.
fn step_extra_lines(step: &StepReport) -> String {
    let mut out = String::new();
    if let Some(per) = step.rss_delta_per_client_kib {
        out.push_str(&format!("  N={:<4} rss over baseline: {:.0} KiB/client", step.clients_target, per));
        if let Some(td) = step.threads_delta {
            out.push_str(&format!("; threads over baseline: {td:+}"));
        }
        out.push('\n');
    }
    if !step.connect_failures.is_empty() {
        out.push_str(&format!(
            "  N={:<4} connect failures ({}): {}\n",
            step.clients_target,
            step.connect_failures.len(),
            summarize_failures(&step.connect_failures),
        ));
    }
    if let Some(calls) = step.calls {
        out.push_str(&format!(
            "  N={:<4} driven calls: n={} p50={:.1}ms p90={:.1}ms max={:.1}ms\n",
            step.clients_target, calls.count, calls.p50_ms, calls.p90_ms, calls.max_ms,
        ));
    }
    if let Some(spb) = step.sessions_per_body {
        out.push_str(&format!(
            "  M={:<4} sessions: expected={} actual={}{} \n",
            spb,
            step.sessions_expected.map_or_else(|| "?".to_string(), |v| v.to_string()),
            step.sessions_actual.map_or_else(|| "?".to_string(), |v| v.to_string()),
            if step.sessions_match == Some(true) { " (match)" } else { " (MISMATCH)" },
        ));
    }
    if let Some(fanout) = step.presence_fanout {
        out.push_str(&format!(
            "  M={:<4} presence fan-out: n={} p50={:.1}ms p90={:.1}ms max={:.1}ms\n",
            step.sessions_per_body.unwrap_or(0),
            fanout.count,
            fanout.p50_ms,
            fanout.p90_ms,
            fanout.max_ms,
        ));
    }
    if let Some(roster_read) = step.roster_read {
        out.push_str(&format!(
            "  M={:<4} roster read: n={} p50={:.1}ms p90={:.1}ms max={:.1}ms\n",
            step.sessions_per_body.unwrap_or(0),
            roster_read.count,
            roster_read.p50_ms,
            roster_read.p90_ms,
            roster_read.max_ms,
        ));
    }
    out.push_str(&sustained_throughput_lines(step));
    if let Some(churn) = &step.churn {
        out.push_str(&churn_lines(churn));
    }
    out
}

/// Scenario 4's (#373) own detail lines.
fn churn_lines(c: &ChurnReport) -> String {
    let lat = |s: Option<LatencyStats>| {
        s.map_or_else(
            || "n=0".to_string(),
            |s| format!("n={} p50={:.1}ms p90={:.1}ms p99={:.1}ms max={:.1}ms", s.count, s.p50_ms, s.p90_ms, s.p99_ms, s.max_ms),
        )
    };
    let mut out = format!(
        "  churn: {}/{} cycles returned the hub to baseline ({} bodies x {} sessions each)\n",
        c.cycles_completed, c.cycles_target, c.bodies_per_cycle, c.sessions_per_body,
    );
    out.push_str(&format!("  join -> registered: {}\n", lat(c.join_to_registered)));
    out.push_str(&format!("  say:                {} (failures={})\n", lat(c.say), c.say_failures));
    out.push_str(&format!("  detach all bodies:  {}\n", lat(c.detach)));
    out.push_str(&format!("  killed -> baseline: {}\n", lat(c.cleanup)));
    out.push_str(&format!(
        "  hub rss: before={} after={}\n",
        opt_f(c.hub_rss_before_mib, "MiB"),
        opt_f(c.hub_rss_after_mib, "MiB"),
    ));
    if let Some(d) = &c.dead_backend {
        let verdict = match d.detection_ms {
            Some(ms) if d.detected => format!("detected after {ms:.0}ms"),
            _ => format!("NOT detected within {}s", d.window_secs),
        };
        out.push_str(&format!("  dead attach backend: {verdict}; roster then showed: {}\n", d.roster_after));
    }
    out
}

/// Scenario 3's (#372) own detail lines: sustained drive time, `say`
/// latency, queue depth, and the RSS-over-time series. Split out of
/// [`step_extra_lines`] for the same line-count/complexity reason that
/// function was split out of [`human_table`].
fn sustained_throughput_lines(step: &StepReport) -> String {
    let mut out = String::new();
    if let Some(secs) = step.sustained_seconds {
        out.push_str(&format!("  sustained drive: {secs:.1}s\n"));
    }
    if let Some(calls) = step.calls {
        out.push_str(&format!(
            "  say latency: n={} p50={:.1}ms p90={:.1}ms p99={:.1}ms max={:.1}ms (failures={})\n",
            calls.count,
            calls.p50_ms,
            calls.p90_ms,
            calls.p99_ms,
            calls.max_ms,
            step.say_failures.unwrap_or(0),
        ));
    }
    if step.queue_depth_max.is_some() || step.queue_depth_final.is_some() {
        out.push_str(&format!(
            "  queue depth: max={} final={} events={} queue_full_refusals={}\n",
            step.queue_depth_max.map_or_else(|| "?".to_string(), |v| v.to_string()),
            step.queue_depth_final.map_or_else(|| "?".to_string(), |v| v.to_string()),
            step.queue_events_observed.map_or_else(|| "?".to_string(), |v| v.to_string()),
            step.queue_full_refusals.map_or_else(|| "?".to_string(), |v| v.to_string()),
        ));
    }
    if !step.rss_series.is_empty() {
        out.push_str(&format!("  hub RSS over time ({} samples):\n", step.rss_series.len()));
        for point in &step.rss_series {
            out.push_str(&format!(
                "    t={:>6.1}s rss={} threads={}\n",
                point.elapsed_s,
                opt_f(point.rss_mib, "MiB"),
                opt_u(point.threads),
            ));
        }
    }
    out
}

/// Collapse a failure list into `reason xN` pairs — 150 identical refusals
/// should read as one line, not 150.
fn summarize_failures(failures: &[String]) -> String {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for f in failures {
        match counts.iter_mut().find(|(k, _)| k == f) {
            Some((_, n)) => *n += 1,
            None => counts.push((f.clone(), 1)),
        }
    }
    counts
        .into_iter()
        .map(|(k, n)| format!("{k} x{n}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn opt_f(v: Option<f64>, unit: &str) -> String {
    match v {
        Some(v) if unit.is_empty() => format!("{v:.1}"),
        Some(v) => format!("{v:.1}{unit}"),
        None => "?".to_string(),
    }
}

fn opt_u(v: Option<usize>) -> String {
    v.map_or_else(|| "?".to_string(), |v| v.to_string())
}
