//! Portable resource sampling of the *hub* process (issue #369 harness bullet
//! 4: "aggregate resource usage … via `ps`/`/proc`").
//!
//! Holler targets macOS and Linux, so this mirrors the split
//! `crates/holler-cli/tests/load_roster_scale_test.rs` already uses (a
//! `#[cfg(target_os = "linux")]` `/proc` reader with a portable fallback) —
//! except that a fallback returning `None` everywhere is useless for a harness
//! whose whole job is producing numbers, so the non-Linux path shells out to
//! `ps`, which both platforms ship.
//!
//! **CPU is sampled as a delta, never as `ps %cpu`.** `ps`'s own `%cpu` column
//! is an average over the process's entire lifetime on both platforms, which
//! for a hub that has been up since before the ramp started is exactly the
//! wrong number. [`CpuMeter`] instead reads cumulative CPU *time* and divides
//! the increment by the wall-clock increment, so each ramp step reports the
//! CPU it actually used during that step.

use std::process::Command;
use std::time::Instant;

use crate::metrics::ResourceSample;

/// Read cumulative CPU time (seconds) and RSS (KiB) for `pid`.
///
/// `ps -o time=,rss= -p PID` is the one invocation that means the same thing
/// on macOS and Linux: `time` is cumulative CPU time (`[[dd-]hh:]mm:ss[.ff]`)
/// and `rss` is resident set size in KiB. `None` if the process is gone or the
/// output does not parse — a missing measurement is reported as missing, never
/// defaulted to zero.
fn ps_time_and_rss(pid: u32) -> Option<(f64, f64)> {
    let out = Command::new("ps")
        .args(["-o", "time=,rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| !l.trim().is_empty())?;
    let mut fields = line.split_whitespace();
    let cpu = parse_ps_time(fields.next()?)?;
    let rss_kib: f64 = fields.next()?.parse().ok()?;
    Some((cpu, rss_kib))
}

/// Parse `ps`'s `time` column into seconds. Accepts every shape either
/// platform emits: `mm:ss`, `mm:ss.ff` (macOS), `hh:mm:ss` and `dd-hh:mm:ss`
/// (Linux, long-lived processes).
fn parse_ps_time(s: &str) -> Option<f64> {
    let (days, rest) = match s.split_once('-') {
        Some((d, rest)) => (d.parse::<f64>().ok()?, rest),
        None => (0.0, s),
    };
    let mut secs = 0.0;
    for part in rest.split(':') {
        secs = secs * 60.0 + part.parse::<f64>().ok()?;
    }
    Some(days * 86_400.0 + secs)
}

/// OS thread count for `pid`.
///
/// Linux reads `/proc/<pid>/status`'s `Threads:` line — exact. macOS has no
/// such counter, so this counts `ps -M`'s per-thread rows: that command prints
/// one header line, one process-summary line, then one line per thread *when
/// the process has more than one*, so the count is `lines - 2` with a floor of
/// 1 (a single-threaded process prints only the header and its summary).
#[cfg(target_os = "linux")]
fn thread_count(pid: u32) -> Option<usize> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("Threads:"))
        .and_then(|n| n.trim().parse::<usize>().ok())
}

#[cfg(not(target_os = "linux"))]
fn thread_count(pid: u32) -> Option<usize> {
    let out = Command::new("ps").args(["-M", "-p", &pid.to_string()]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let lines = String::from_utf8_lossy(&out.stdout).lines().filter(|l| !l.trim().is_empty()).count();
    Some(lines.saturating_sub(2).max(1))
}

/// Open file descriptors for `pid`. Linux only: the macOS equivalent is
/// `lsof`, which is slow enough (hundreds of ms) to perturb the very
/// steady-state window this harness samples during, so it is deliberately not
/// measured there rather than measured badly.
#[cfg(target_os = "linux")]
fn open_fds(pid: u32) -> Option<usize> {
    std::fs::read_dir(format!("/proc/{pid}/fd")).ok().map(Iterator::count)
}

#[cfg(not(target_os = "linux"))]
fn open_fds(_pid: u32) -> Option<usize> {
    None
}

/// Stateful CPU sampler: each [`CpuMeter::sample`] reports the CPU used since
/// the previous call, so a per-ramp-step number is a per-ramp-step number.
pub struct CpuMeter {
    pid: u32,
    last: Option<(f64, Instant)>,
}

impl CpuMeter {
    pub fn new(pid: u32) -> Self {
        Self { pid, last: None }
    }

    /// Take a full resource sample. The first call establishes the CPU
    /// baseline and reports `cpu_percent: None` (there is no interval yet to
    /// average over — reporting 0 would be a fabricated number).
    pub fn sample(&mut self) -> ResourceSample {
        let now = Instant::now();
        let (cpu_secs, rss_kib) = match ps_time_and_rss(self.pid) {
            Some(v) => (Some(v.0), Some(v.1)),
            None => (None, None),
        };

        let cpu_percent = match (cpu_secs, self.last) {
            (Some(cpu), Some((prev_cpu, prev_at))) => {
                let wall = now.duration_since(prev_at).as_secs_f64();
                if wall > 0.0 {
                    Some(((cpu - prev_cpu) / wall) * 100.0)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(cpu) = cpu_secs {
            self.last = Some((cpu, now));
        }

        ResourceSample {
            rss_mib: rss_kib.map(|k| k / 1024.0),
            threads: thread_count(self.pid),
            open_fds: open_fds(self.pid),
            cpu_percent,
        }
    }
}
