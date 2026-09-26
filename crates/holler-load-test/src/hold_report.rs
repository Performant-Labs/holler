//! The `session-hold` scenario's report (issue #444): what was measured, and
//! the thresholds it was judged against. Kept apart from `metrics.rs`, whose
//! `StepReport` is shaped for the ramp scenarios.

use serde::Serialize;

use crate::metrics::LatencyStats;

/// One driven load phase: every driven session runs a sequential `say` loop
/// against the hub for the phase's duration, some of them held.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Phase {
    pub label: String,
    /// Percent of the driven sessions that were held during the phase.
    pub held_percent: u32,
    pub driven_sessions: usize,
    pub held_sessions: usize,
    pub seconds: f64,
    /// Round-trip latency of `say` to the sessions that were **not** held.
    pub unheld: Option<LatencyStats>,
    pub unheld_delivered: usize,
    pub unheld_per_sec: f64,
    /// `session_busy` answers (nothing delivered; retried) to unheld sessions.
    pub unheld_busy_retries: usize,
    /// Latency of the `session_held` refusals of the held sessions.
    pub refusals: Option<LatencyStats>,
    pub refusals_per_sec: f64,
    /// A `say` to a held session that was **delivered** — a correctness
    /// failure, and hard-fails the run.
    pub delivered_to_held: usize,
    /// A refusal that was neither `session_held` nor `session_busy`.
    pub other_errors: usize,
    pub hub_rss_start_mib: Option<f64>,
    pub hub_rss_end_mib: Option<f64>,
}

/// `roster` read latency with a given number of sessions and holds.
#[derive(Debug, Clone, Serialize)]
pub struct RosterRead {
    pub sessions: usize,
    pub holds: usize,
    pub reads: usize,
    pub latency: Option<LatencyStats>,
}

/// Rapid hold/release cycles across many sessions.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Churn {
    pub workers: usize,
    pub seconds: f64,
    pub operations: usize,
    pub failed_operations: usize,
    pub ops_per_sec: f64,
    pub hold_latency: Option<LatencyStats>,
    pub release_latency: Option<LatencyStats>,
    pub hub_cpu_percent_mean: Option<f64>,
    pub hub_rss_start_mib: Option<f64>,
    /// RSS after the churn and a cooldown, so a leak shows as growth here.
    pub hub_rss_end_mib: Option<f64>,
    pub holds_file_bytes_end: u64,
}

/// A hub restart, with and without a large number of persisted holds.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Restart {
    pub holds: usize,
    pub holds_file_bytes: u64,
    /// Seconds to hold every session (the persistence write cost at scale).
    pub hold_all_secs: f64,
    /// Restart to `listening` with no persisted holds, ms.
    pub listening_ms_without_holds: f64,
    /// Restart to `listening` with `holds` persisted, ms.
    pub listening_ms_with_holds: f64,
    /// Seconds until every session is back on the roster and shown held;
    /// `None` if that did not happen within the budget.
    pub all_held_visible_secs: Option<f64>,
    /// How many holds the state file held after the restart.
    pub holds_after_restart: usize,
    pub sample_say_refused: bool,
}

/// One threshold the run is judged against.
#[derive(Debug, Clone, Serialize)]
pub struct Threshold {
    pub name: String,
    pub limit: f64,
    pub actual: Option<f64>,
    pub unit: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HoldReport {
    pub sessions_total: usize,
    pub sessions_driven: usize,
    pub phase_seconds: u64,
    /// The two hold-free runs: their difference is the noise floor.
    pub baseline_runs: Vec<Phase>,
    pub held_fractions: Vec<Phase>,
    pub refusal_only: Option<Phase>,
    pub roster_reads: Vec<RosterRead>,
    pub churn: Option<Churn>,
    pub restart: Option<Restart>,
    /// What else was running on the machine (#444: a number without its
    /// machine is not a result).
    pub other_load: String,
    pub thresholds: Vec<Threshold>,
}

impl HoldReport {
    /// The thresholds that were exceeded (or could not be evaluated).
    pub fn failed(&self) -> Vec<&Threshold> {
        self.thresholds.iter().filter(|t| !t.passed).collect()
    }
}

/// The plain-ASCII section `human_table` appends for this scenario.
pub fn lines(r: &HoldReport) -> String {
    let stat = |s: Option<LatencyStats>| s.map_or_else(|| "-".to_string(), |s| format!("p50={:.1} p90={:.1} p99={:.1} max={:.1} n={}", s.p50_ms, s.p90_ms, s.p99_ms, s.max_ms, s.count));
    let rss = |a: Option<f64>| a.map_or_else(|| "?".to_string(), |v| format!("{v:.1}"));
    let mut out = format!(
        "\nsession-hold (#444): {} sessions on the hub, {} driven, {}s per phase\nother load on the machine: {}\n\n",
        r.sessions_total, r.sessions_driven, r.phase_seconds, r.other_load
    );
    out.push_str("say path (ms): unheld sessions' say latency, held sessions' refusal latency\n");
    for p in r.baseline_runs.iter().chain(&r.held_fractions).chain(&r.refusal_only) {
        out.push_str(&format!(
            "  {:<26} held {:>3}%  unheld: {} ({:.1}/s, {} busy retries)\n{:<38}refusals: {} ({:.0}/s); rss {} -> {} MiB; delivered-to-held={} other-errors={}\n",
            p.label,
            p.held_percent,
            stat(p.unheld),
            p.unheld_per_sec,
            p.unheld_busy_retries,
            "",
            stat(p.refusals),
            p.refusals_per_sec,
            rss(p.hub_rss_start_mib),
            rss(p.hub_rss_end_mib),
            p.delivered_to_held,
            p.other_errors,
        ));
    }
    out.push_str("\nroster read (ms):\n");
    for rr in &r.roster_reads {
        out.push_str(&format!("  {} sessions, {} held: {}\n", rr.sessions, rr.holds, stat(rr.latency)));
    }
    if let Some(c) = &r.churn {
        out.push_str(&format!(
            "\nhold/release churn: {} workers, {:.0}s, {} ops ({:.0}/s, {} failed)\n  hold: {}\n  release: {}\n  hub cpu mean {}%; rss {} -> {} MiB after cooldown; holds file {} bytes\n",
            c.workers, c.seconds, c.operations, c.ops_per_sec, c.failed_operations, stat(c.hold_latency), stat(c.release_latency),
            rss(c.hub_cpu_percent_mean), rss(c.hub_rss_start_mib), rss(c.hub_rss_end_mib), c.holds_file_bytes_end,
        ));
    }
    if let Some(x) = &r.restart {
        out.push_str(&format!(
            "\nrestart with {} persisted holds ({} bytes; holding all took {:.1}s): listening after {:.0} ms (vs {:.0} ms with none); all held and visible after {}; holds in file after restart: {}; sample say refused: {}\n",
            x.holds, x.holds_file_bytes, x.hold_all_secs, x.listening_ms_with_holds, x.listening_ms_without_holds,
            x.all_held_visible_secs.map_or_else(|| "NOT within the budget".to_string(), |s| format!("{s:.1}s")),
            x.holds_after_restart, x.sample_say_refused,
        ));
    }
    out.push_str("\nthresholds (set from the first measured baseline; see docs/testing.md):\n");
    for t in &r.thresholds {
        out.push_str(&format!(
            "  {} {:<44} limit {:>9.1} {:<3} actual {}\n",
            if t.passed { "ok  " } else { "FAIL" },
            t.name,
            t.limit,
            t.unit,
            t.actual.map_or_else(|| "n/a".to_string(), |v| format!("{v:.1}")),
        ));
    }
    out
}
