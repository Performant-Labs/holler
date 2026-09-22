//! `holler-load-test` — Holler's dedicated load/performance harness (issue
//! [#369](https://github.com/Performant-Labs/holler/issues/369)).
//!
//! # Why a purpose-built binary
//!
//! Holler's wire protocol is JSON-RPC 2.0 over a WebSocket (ADR 0004), not
//! HTTP REST, so the generic load tools (k6, wrk, vegeta, Locust) cannot speak
//! to it at all. #369's answer is the same one this repo already reached for
//! its deterministic agent double: a purpose-built binary that lives in the
//! workspace, is built by `cargo build --workspace`, and drives the *real*
//! shipping binaries — `stub-acp`'s convention, applied to load.
//!
//! # What it drives
//!
//! * A real `holler hub serve` on a free loopback port (or an already-running
//!   one, via `--hub-url` + `--hub-state`).
//! * N concurrent connections, in one of two modes:
//!   * **wire** (`scenario connection-scale`) — in-process clients running the
//!     real circuit handshake, indistinguishable from a body to the hub. This
//!     is what makes per-connection handshake latency measurable, and what
//!     scales to #370's N=200.
//!   * **body fleet** (`scenario body-fleet`) — real `holler body run`
//!     processes, each hosting M spawn-mode `stub-acp` sessions, with a
//!     rate-driven `say`/`interrupt`/`roster` call mix against them.
//! * Per-call latency and the hub process's own RSS / thread count / CPU.
//!
//! # What it deliberately does not do
//!
//! Assert thresholds. #370 says they are "TBD from the first real baseline
//! run"; a harness that invents one produces a green tick with nothing behind
//! it. Every number here is measured or absent.
//!
//! # Running it
//!
//! ```text
//! cargo build --workspace --release
//! ./target/release/holler-load-test --scenario connection-scale --ramp 1,50,200
//! ```
//!
//! See `docs/testing.md` § "Load testing" for the full flag surface and the
//! recorded baselines.

mod fleet;
mod hub;
mod metrics;
mod proc;
mod scenario;
mod session_scale;
mod wire;

use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

use clap::{Parser, ValueEnum};

/// The harness's error type. A load harness that aborts on the first
/// unexpected shape is useless, so every fallible step returns this and the
/// caller decides whether it ends a client, a step, or the run.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Res<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Scenario {
    /// Issue #370: ramp concurrent connections, measure handshake latency and
    /// hub resource usage at each rung.
    ConnectionScale,
    /// #369's process-fleet half: real body processes with real `stub-acp`
    /// sessions, driven at a controlled request rate.
    BodyFleet,
    /// Issue #371: fix N real body processes, ramp M spawn-mode `stub-acp`
    /// sessions per body — 10 → 100 → 500 — and measure `session/presence`
    /// propagation latency, the `hub status --json` `sessions` invariant
    /// (hard-failed on mismatch), and `holler roster --json` read latency.
    SessionScale,
}

#[derive(Parser, Debug)]
#[command(
    name = "holler-load-test",
    about = "Holler's load/performance harness (issue #369)",
    long_about = None,
)]
struct Cli {
    /// Which scenario to run.
    #[arg(long, value_enum, default_value = "connection-scale")]
    scenario: Scenario,

    /// Concurrent-connection counts to ramp through, comma-separated.
    /// #370's own ramp is the default.
    #[arg(long, default_value = "1,50,200")]
    ramp: String,

    /// How many handshakes may be in flight at once. Kept under the hub's
    /// default unauthenticated-connection cap (64, `holler_hub::hygiene`) so a
    /// ramp measures connection scale rather than that cap; raise it
    /// deliberately to observe the cap instead.
    #[arg(long, default_value_t = 32)]
    connect_concurrency: usize,

    /// How long to let the hub settle before sampling its resource usage.
    #[arg(long, default_value_t = 2000)]
    settle_ms: u64,

    /// Presence-heartbeat interval each held connection uses. The production
    /// default is 15s; a shorter beat keeps a short ramp step realistic.
    #[arg(long, default_value_t = 5000)]
    heartbeat_ms: u64,

    /// `body-fleet`: how many real `holler body run` processes to start.
    #[arg(long, default_value_t = 3)]
    bodies: usize,

    /// `body-fleet`: spawn-mode `stub-acp` sessions per body.
    #[arg(long, default_value_t = 1)]
    sessions_per_body: usize,

    /// `body-fleet`: how many `say`/`interrupt`/`roster` calls to drive.
    #[arg(long, default_value_t = 24)]
    calls: usize,

    /// `body-fleet`: target call rate, calls per second across all sessions.
    #[arg(long, default_value_t = 4.0)]
    rate: f64,

    /// `session-scale`: spawn-mode `stub-acp` sessions *per body* to ramp
    /// through — #371's own ramp is the default. The real total session
    /// count at each rung is `--bodies * <this rung's count>`.
    #[arg(long, default_value = "10,100,500")]
    session_ramp: String,

    /// `session-scale`: how many sample sessions (spread across all bodies)
    /// to probe for `session/presence` propagation latency, per rung.
    #[arg(long, default_value_t = 5)]
    fanout_samples: usize,

    /// `session-scale`: how many real `holler roster --json` invocations to
    /// time, per rung.
    #[arg(long, default_value_t = 20)]
    roster_reads: usize,

    /// Path to the `holler` binary (default: next to this one).
    #[arg(long)]
    holler_bin: Option<PathBuf>,

    /// Path to the `stub-acp` binary (default: next to this one).
    #[arg(long)]
    stub_acp_bin: Option<PathBuf>,

    /// Drive an already-running hub at this URL instead of starting one.
    /// Requires `--hub-state`.
    #[arg(long)]
    hub_url: Option<String>,

    /// The `HOLLER_STATE_DIR` of the hub named by `--hub-url` — minting a join
    /// token means writing that hub's own token store.
    #[arg(long)]
    hub_state: Option<PathBuf>,

    /// Extra `KEY=VALUE` environment variables for the hub this harness
    /// starts (e.g. `HOLLER_MAX_PREAUTH_CONNECTIONS=256`). Repeatable.
    #[arg(long = "hub-env", value_name = "KEY=VALUE")]
    hub_env: Vec<String>,

    /// Write the JSON report here (the human table always goes to stdout).
    #[arg(long)]
    json_out: Option<PathBuf>,
}

/// The resolved run configuration the scenarios read.
pub struct Config {
    pub ramp: Vec<usize>,
    pub connect_concurrency: usize,
    pub settle: Duration,
    pub heartbeat: Duration,
    pub bodies: usize,
    pub sessions_per_body: usize,
    pub calls: usize,
    pub rate: f64,
    pub holler_bin: PathBuf,
    pub stub_acp_bin: PathBuf,
    pub session_ramp: Vec<usize>,
    pub fanout_samples: usize,
    pub roster_reads: usize,
}

fn main() -> Res<()> {
    let cli = Cli::parse();

    let ramp = parse_ramp(&cli.ramp)?;
    let session_ramp = parse_ramp(&cli.session_ramp)?;
    let holler_bin = match cli.holler_bin {
        Some(p) => p,
        None => hub::sibling_bin(holler_exe_name())?,
    };
    let stub_acp_bin = match cli.stub_acp_bin {
        Some(p) => p,
        None => hub::sibling_bin(stub_acp_exe_name())?,
    };
    let cfg = Config {
        ramp,
        connect_concurrency: cli.connect_concurrency,
        settle: Duration::from_millis(cli.settle_ms),
        heartbeat: Duration::from_millis(cli.heartbeat_ms),
        bodies: cli.bodies,
        sessions_per_body: cli.sessions_per_body,
        calls: cli.calls,
        rate: cli.rate,
        holler_bin: holler_bin.clone(),
        stub_acp_bin,
        session_ramp,
        fanout_samples: cli.fanout_samples,
        roster_reads: cli.roster_reads,
    };

    let hub_env = parse_env(&cli.hub_env)?;
    let mut hub = match (cli.hub_url, cli.hub_state) {
        (Some(url), Some(state)) => hub::Hub::attach(holler_bin, hub::StateDir::adopt(state), url),
        (Some(_), None) => return Err("--hub-url requires --hub-state (the hub's own HOLLER_STATE_DIR)".into()),
        (None, _) => hub::Hub::start(holler_bin, hub::StateDir::fresh()?, &hub_env)?,
    };

    let mut report = metrics::Report {
        scenario: format!("{:?}", cli.scenario),
        issue: match cli.scenario {
            Scenario::ConnectionScale => "#370".to_string(),
            Scenario::BodyFleet => "#369".to_string(),
            Scenario::SessionScale => "#371".to_string(),
        },
        started_at: rfc3339_utc_now(),
        host: metrics::HostInfo::detect(),
        config: config_json(&cfg, &hub_env),
        hub_baseline: metrics::ResourceSample::default(),
        steps: Vec::new(),
    };

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let outcome = runtime.block_on(async {
        match cli.scenario {
            Scenario::ConnectionScale => scenario::run(&cfg, &hub, &mut report).await,
            Scenario::BodyFleet => scenario::run_body_fleet(&cfg, &hub, &mut report).await,
            Scenario::SessionScale => session_scale::run(&cfg, &hub, &mut report).await,
        }
    });
    hub.stop();
    outcome?;

    print!("{}", metrics::human_table(&report));
    if let Some(path) = cli.json_out {
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("json report: {}", path.display());
    }
    Ok(())
}

fn parse_ramp(s: &str) -> Res<Vec<usize>> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        out.push(part.parse::<usize>().map_err(|e| format!("bad --ramp element {part:?}: {e}"))?);
    }
    if out.is_empty() {
        return Err("--ramp named no counts".into());
    }
    Ok(out)
}

fn parse_env(pairs: &[String]) -> Res<Vec<(String, String)>> {
    pairs
        .iter()
        .map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| format!("--hub-env {p:?} is not KEY=VALUE").into())
        })
        .collect()
}

fn config_json(cfg: &Config, hub_env: &[(String, String)]) -> serde_json::Value {
    serde_json::json!({
        "ramp": cfg.ramp,
        "connect_concurrency": cfg.connect_concurrency,
        "settle_ms": cfg.settle.as_millis() as u64,
        "heartbeat_ms": cfg.heartbeat.as_millis() as u64,
        "bodies": cfg.bodies,
        "sessions_per_body": cfg.sessions_per_body,
        "calls": cfg.calls,
        "rate_per_sec": cfg.rate,
        "session_ramp": cfg.session_ramp,
        "fanout_samples": cfg.fanout_samples,
        "roster_reads": cfg.roster_reads,
        "hub_env": hub_env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(),
    })
}

/// A minimal RFC-3339 UTC stamp. The workspace's `time` crate is
/// `holler-proto`'s dependency for the wire's own timestamps; a report header
/// does not justify pulling it in here, and the arithmetic is exact.
fn rfc3339_utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Howard Hinnant's `civil_from_days`: days since the Unix epoch → (y, m, d).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn holler_exe_name() -> &'static str {
    if cfg!(windows) {
        "holler.exe"
    } else {
        "holler"
    }
}

fn stub_acp_exe_name() -> &'static str {
    if cfg!(windows) {
        "stub-acp.exe"
    } else {
        "stub-acp"
    }
}

/// Kill a child's whole process tree and reap it.
///
/// Unix: the child was put in its own process group at spawn (see
/// [`hub::own_process_group`]), so one `kill(-pgid, SIGKILL)` takes the child
/// and every descendant — which matters for a `holler body run` that has
/// spawned `stub-acp` children. Windows: `taskkill /F /T`. Idempotent.
pub fn kill_tree(child: &mut Child) {
    let pid = child.id() as i32;
    #[cfg(unix)]
    {
        // SAFETY: `kill` with a negative pid signals the process group the
        // child leads; an already-exited group is simply `ESRCH`, which is
        // ignored here (the `wait` below reaps either way).
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = child.wait();
}
