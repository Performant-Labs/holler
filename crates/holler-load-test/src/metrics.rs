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
        if let Some(per) = step.rss_delta_per_client_kib {
            out.push_str(&format!(
                "  N={:<4} rss over baseline: {:.0} KiB/client",
                step.clients_target, per
            ));
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
