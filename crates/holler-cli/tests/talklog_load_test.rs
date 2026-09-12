#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #300
//! Load test (issue #300): TalkLog behavior at a large accumulated log size.
//!
//! **Bounded/automated**, per the issue's own scope: drive one session
//! through several hundred real prompt/reply exchanges (stub agent,
//! `--chunks 1` for fast turns) via repeated real `say` calls against a real
//! hub + body + `stub-acp` (no mocks of the circuit — the same discipline
//! every other file under this dir uses), measure/log per-append latency and
//! file size as entry count grows, and assert no corruption/data loss at any
//! point. This does **not** redesign TalkLog's persistence model — that is
//! explicitly out of scope for #300 — it only characterizes actual behavior.
//!
//! # What the current code actually does (read before writing this test)
//!
//! `crates/holler-hub/src/talk.rs::append_talklog` opens
//! `<state>/hub/talklog/<label>__<session>.jsonl`
//! (`crates/holler-hub/src/state.rs::talklog_path`) with `OpenOptions::new()
//! .create(true).append(true)` and writes exactly one JSON line per call. This
//! is **not** the "load the whole array, append, rewrite the whole file"
//! shape the issue's own historical framing describes (that was the
//! `holler-server` predecessor design, mirrored by issue holler-server#296) —
//! the module layout has shifted since: this hub already persists TalkLog as
//! an append-only JSONL file, so each append is an O(1) `write(2)` onto the
//! open fd, not an O(entries) read-modify-write. The load test below still
//! measures the real thing rather than assuming that from the source alone.

mod support;

use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use support::{join, mint_token, wait_for, write_sessions_toml, Body, Hub, StateDir};

/// Real prompt/reply exchanges driven through `say`, on top of one warm-up
/// turn. Within the issue's "several hundred to a couple thousand" bound;
/// picked to keep this standalone test's wall-clock in the tens-of-seconds
/// range (each iteration spawns a real `holler say` subprocess — full binary
/// startup, a control-socket round trip, and a real hub turn — measured at
/// ~120ms/iteration; 500 iterations already crossed a minute) rather than
/// minutes.
const ENTRIES: usize = 250;

/// `say SESSION TEXT` against `state_path` (mirrors `talk_test.rs`'s own
/// `say_full`, duplicated here rather than shared since each integration-test
/// binary only pulls in `support` as a module, not a shared crate).
fn say(state_path: &Path, session: &str, text: &str) -> Output {
    Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", state_path)
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["say", session, text])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say`")
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn talklog_survives_large_accumulated_log_without_corruption() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let (token_id, secret) = mint_token(&hub_state, "b");
    join(&body_state, &hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(&body_state, &[("alpha", &["--chunks", "1"])]);
    let _body = Body::start(&body_state, &config);

    // Warm up: retry until the hub has cached this body's first presence
    // (the same "unknown session" retry `talk_test.rs::say_ready` uses).
    let warm = wait_for(Duration::from_secs(10), || {
        let out = say(hub_state.path(), "alpha", "warm up");
        (out.status.success() || !stderr_of(&out).contains("unknown session")).then_some(out)
    })
    .unwrap_or_else(|| panic!("warm-up say never got past unknown_session within 10s"));
    assert!(warm.status.success(), "warm-up say must succeed: {}", stderr_of(&warm));

    let path = hub_state.hub().join("talklog").join("default__alpha.jsonl");
    wait_for(Duration::from_secs(5), || std::fs::metadata(&path).ok())
        .unwrap_or_else(|| panic!("talklog never appeared at {}", path.display()));

    let mut latencies_ms: Vec<f64> = Vec::with_capacity(ENTRIES);
    let mut sizes_bytes: Vec<u64> = Vec::with_capacity(ENTRIES);

    for i in 0..ENTRIES {
        let started = Instant::now();
        let out = say(hub_state.path(), "alpha", &format!("exchange number {i}"));
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        assert!(out.status.success(), "say #{i} must succeed; stderr: {}", stderr_of(&out));
        latencies_ms.push(elapsed_ms);
        let size = std::fs::metadata(&path).expect("talklog file exists after every append").len();
        sizes_bytes.push(size);
    }

    // No corruption / no data loss: every line in the whole file parses as
    // JSON, and every one of the (warm-up + ENTRIES) turns left its
    // prompt/done lines behind with a unique prompt_id (an "update" line only
    // exists per chunk the stub streamed — `--chunks 1` still streams exactly
    // one, so this file's prompt/done counts are the load-bearing invariant).
    let content = std::fs::read_to_string(&path).expect("read final talklog");
    let lines: Vec<&str> = content.lines().collect();
    let mut prompt_ids = HashSet::new();
    let mut prompts = 0usize;
    let mut dones = 0usize;
    for (n, line) in lines.iter().enumerate() {
        let v: Value = serde_json::from_str(line).unwrap_or_else(|e| panic!("corrupt talklog line {n}: {e}: {line:?}"));
        if let Some(pid) = v.get("prompt_id").and_then(|p| p.as_str()) {
            prompt_ids.insert(pid.to_string());
        }
        if v.get("text").is_some() {
            prompts += 1;
        }
        if v.get("stopReason").is_some() {
            dones += 1;
        }
    }
    let total_turns = ENTRIES + 1; // + the warm-up turn
    assert_eq!(prompts, total_turns, "every turn's prompt line must be present, no loss: {} lines total", lines.len());
    assert_eq!(dones, total_turns, "every turn's done line must be present, no loss: {} lines total", lines.len());
    assert_eq!(prompt_ids.len(), total_turns, "every turn has a unique prompt_id, no collision/corruption");

    // Latency-growth characterization (the issue's own actionable output,
    // logged as evidence even if inconclusive): compare the mean per-append
    // latency of the first vs. last quartile of turns.
    let q = ENTRIES / 4;
    let first_mean = latencies_ms[..q].iter().sum::<f64>() / q as f64;
    let last_mean = latencies_ms[ENTRIES - q..].iter().sum::<f64>() / q as f64;
    let growth_ratio = last_mean / first_mean.max(0.001);
    let first_size = sizes_bytes[0];
    let last_size = *sizes_bytes.last().expect("at least one entry");
    eprintln!(
        "[talklog-load #300] entries={ENTRIES} first_quartile_mean_ms={first_mean:.3} \
         last_quartile_mean_ms={last_mean:.3} growth_ratio={growth_ratio:.3} \
         file_size_first={first_size}B file_size_last={last_size}B"
    );
    // This does NOT fail merely because latency grew somewhat (turn-to-turn
    // noise on a shared CI runner is expected, and the issue's own scope is
    // "characterize, don't redesign") — but a large, clearly non-linear
    // blowup (5x+ from first to last quartile) at only ENTRIES-scale would be
    // real evidence TalkLog's append path degrades, which the issue asks be
    // reported loudly rather than silently passed over.
    assert!(
        growth_ratio < 5.0,
        "per-append latency grew {growth_ratio:.2}x from the first to the last quartile over {ENTRIES} \
         entries (first={first_mean:.3}ms, last={last_mean:.3}ms) — this is evidence of real non-linear \
         growth in TalkLog's append path (crates/holler-hub/src/talk.rs::append_talklog). Not a redesign \
         target for issue #300, but a finding that must be reported, not silently passed."
    );
}
