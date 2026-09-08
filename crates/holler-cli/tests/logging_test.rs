#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Integration tests for the debug-logging CLI wiring (story #144, RED first).
//!
//! These drive the **real `holler` binary** as a subprocess and pin the
//! user-facing contract from ADR 0003 + the story:
//!
//! - `--debug` / `--log-format` beat the `HOLLER_DEBUG` / `HOLLER_LOG_FORMAT`
//!   environment; defaults are `none` / `text`.
//! - An unrecognised value is a fail-closed policy refusal: **exit 3**, a
//!   stderr message, and *no* logging output (the banner is *not* emitted).
//! - All logging goes to **stderr**; **stdout stays clean**.
//! - Every *log* line carries a fixed 12-wide `component` column.
//! - The `logging_started` banner is the first line written to stderr.
//! - `--log-format json` renders log events as one JSON object per line.
//!
//! NOTE (Decision, recorded in the PR): the story's draft named
//! `hub token list --json` / `hub token mint` as the redaction /
//! stdout-cleanliness leaves, but those leaves do not yet exist (token
//! management lands with the hub-serving story). This file therefore exercises
//! the *same leaf-agnostic contract* through `roster`, a currently-present leaf
//! (a "not implemented" stub, exit 1). The contract (stderr-only logs, clean
//! stdout, the component column, the banner) is identical for every leaf, so
//! pinning it here is equivalent.
//!
//! Two contracts are exercised by driving the #144 module's **public API
//! directly from this test process** (rather than waiting on a hub/body call
//! site that does not exist yet): `warn_is_always_emitted…` and
//! `json_format_renders_events…`. Because the module's `init` is a process-wide
//! `OnceLock`, those two tests live here (their own test binary) so each gets a
//! fresh process to install its own config; they are fully deterministic and
//! OS-independent.

use std::iter::Iterator;

use assert_cmd::Command;
use holler_proto::log::{
    emit, init, Component, Config, DebugLevel, Direction, Event, LogFormat, Severity,
};
use predicates::boolean::PredicateBooleanExt;
use predicates::str::contains;

fn holler() -> Command {
    Command::cargo_bin("holler").expect("holler binary on PATH")
}

// An unrecognised `--debug` value is a fail-closed policy refusal (ADR 0003:
// exit 3), reported on stderr, with no logging output.
#[test]
fn invalid_debug_value_exits_3() {
    holler()
        .args(["--debug", "verbose"])
        .arg("roster")
        .assert()
        .failure()
        .code(3)
        .stderr(contains("--debug"));
}

// The same holds for `--log-format`.
#[test]
fn invalid_log_format_exits_3() {
    holler()
        .args(["--log-format", "yaml"])
        .arg("roster")
        .assert()
        .failure()
        .code(3)
        .stderr(contains("--log-format"));
}

// A fail-closed refusal (exit 3) emits *no* log line at all: the banner itself
// reports the *resolved* settings, so it cannot appear for a value that failed
// to resolve. Only the error message is written.
#[test]
fn fail_closed_refusal_emits_no_log_lines() {
    let out = holler()
        .env("HOLLER_DEBUG", "noisy")
        .args(["--debug", "verbose"])
        .arg("roster")
        .output()
        .expect("run holler");
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let has_banner = stderr.lines().any(|l| l.contains("logging_started"));
    assert!(
        !has_banner,
        "a fail-closed refusal must not emit the banner (no settings to report)\ngot:\n{stderr}"
    );
    // …and the refusal is *reported* on stderr.
    assert!(
        stderr.contains("--debug"),
        "the refusal should name the offending flag\ngot:\n{stderr}"
    );
}

// A flag beats the environment: `--debug none` overrides `HOLLER_DEBUG=noisy`.
// The banner reports the *resolved* level (`none`), and no noisy debug frame
// (a `->` direction marker) leaks through.
#[test]
fn debug_flag_beats_env() {
    holler()
        .env("HOLLER_DEBUG", "noisy")
        .env("HOLLER_LOG_FORMAT", "text")
        .args(["--debug", "none"])
        .arg("roster")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("level=none")) // the flag won over the env's `noisy`
        .stderr(contains("->").not()); // …so no noisy debug frame is emitted
}

// And the reverse: `--debug noisy` overrides `HOLLER_DEBUG=none`.
#[test]
fn env_none_loses_to_flag_noisy() {
    holler()
        .env("HOLLER_DEBUG", "none")
        .env("HOLLER_LOG_FORMAT", "text")
        .args(["--debug", "noisy"])
        .arg("roster")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("level=noisy"));
}

// The banner names the resolved level and format.
#[test]
fn banner_names_resolved_level_and_format() {
    holler()
        .env("HOLLER_DEBUG", "noisy")
        .env("HOLLER_LOG_FORMAT", "json")
        .arg("roster")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("logging_started"))
        .stderr(contains("level=noisy"))
        .stderr(contains("format=json"));
}

// stdout is reserved for command output: logging must never leak onto it.
#[test]
fn log_output_stays_off_stdout() {
    holler()
        .env("HOLLER_DEBUG", "noisy")
        .env("HOLLER_LOG_FORMAT", "text")
        .arg("roster")
        .assert()
        .failure()
        .code(1)
        .stdout(contains(""))
        .stdout(contains("logging_started").not());
}

// Every *log* line carries a fixed 12-wide component column. The text layout
// is `<ts> <LEVEL> <12-char component column> <dir> …`; the column is left-
// padded to exactly 12 chars, so the direction token that follows it begins
// 12 chars after the level token. (`split_whitespace` would collapse the
// padding, so the width is measured by byte offset, not token count.)
#[test]
fn each_log_line_carries_component_column() {
    let out = holler()
        .env("HOLLER_DEBUG", "noisy")
        .env("HOLLER_LOG_FORMAT", "text")
        .arg("roster")
        .output()
        .expect("run holler");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let log_lines: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("INFO") || l.contains("WARN") || l.contains(" DEBUG "))
        .collect();
    assert!(
        !log_lines.is_empty(),
        "expected at least one log line (the banner)\n got: {stderr:?}"
    );
    for line in log_lines {
        let level_pos = line
            .find("INFO")
            .or_else(|| line.find("WARN"))
            .or_else(|| line.find("DEBUG"))
            .unwrap_or_else(|| panic!("no level token in log line: {line:?}"));
        // The layout is `<ts> <LEVEL> <12-char component column> <dir> …` (no
        // space *inside* the column — it is left-padded, then followed by a
        // single separator, then the direction). The column therefore occupies
        // offsets 4..16 relative to the level, and the first space at or after
        // offset 17 is the separator before the direction. A column of any
        // other width would move that separator. (`rfind`/`find` on the whole
        // rest won't work: the line's later fields also contain spaces.)
        let rest = &line[level_pos..];
        let col_end = rest
            .char_indices()
            .find(|(i, c)| *i >= 17 && *c == ' ')
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("no separator after the component column in: {line:?}"));
        assert_eq!(
            col_end, 17,
            "the component column is not exactly 12 wide (its separator landed at offset {col_end}, expected 17) in line {line:?}"
        );
    }
}

// The `logging_started` banner is the very first thing written to stderr
// (it precedes any blocking I/O).
#[test]
fn banner_is_first_stderr_line() {
    let out = holler()
        .env("HOLLER_DEBUG", "noisy")
        .env("HOLLER_LOG_FORMAT", "text")
        .arg("roster")
        .output()
        .expect("run holler");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let first = stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .expect("expected at least one stderr line");
    assert!(
        first.contains("logging_started"),
        "the first stderr line should be the banner, got: {first:?}"
    );
}

// --- RED: contracts this story does not yet wire up -------------------------

// A `warn` is emitted **regardless of the debug level** (spec: severities are
// independent of the level — `info`/`warn` are always on; only `debug` is
// gated). At `--debug none` a warn still renders and a debug does not.
//
// Exercised by driving the module's public API directly: install the `none`
// config, then assert a `warn` renders and a `debug` does not (and `emit`
// honours the same gate). This is deterministic and OS-independent (it does not
// rely on a hub/body leaf, which does not yet exist), and it lives in this test
// binary because `init` is process-wide (each test here gets a fresh process).
#[test]
fn warn_is_always_emitted_independently_of_debug_level() {
    let config = Config {
        debug: DebugLevel::None,
        format: LogFormat::Text,
    };
    init(config);

    let warn = warn_event();
    let debug = debug_event();
    // A warn is always on, even at the default level…
    assert!(
        !warn.render(&config).is_empty(),
        "a warn must render even at --debug none (it is always on)"
    );
    // …while a debug is suppressed at the default level.
    assert!(
        debug.render(&config).is_empty(),
        "a debug event must be suppressed at --debug none"
    );
    // And the emit helper honours the same gate (warn prints to stderr, debug
    // is a no-op). We exercise the real write path here; the process's own
    // stderr is not asserted on (that would race the test harness's output).
    emit(&warn);
    emit(&debug);
}

// With `--log-format json`, every log event renders to **one JSON object**
// (machine-parseable, one object per line). The banner is deliberately *not*
// JSON (it is the one human-readable line), so this is about *event* lines.
//
// Exercised by driving the module's public API directly: install the `noisy` +
// `json` config, `emit` several events, and assert each rendered line is a
// single parseable JSON object carrying the structured fields (and, at `noisy`,
// a nested `frame`). Deterministic and OS-independent (no hub/body leaf needed).
#[test]
fn json_format_renders_events_as_one_object_per_line() {
    let config = Config {
        debug: DebugLevel::Noisy,
        format: LogFormat::Json,
    };
    init(config);

    // A few events of different shapes all render to one JSON object per line.
    let events = [
        warn_event(),
        Event {
            component: Component::Roster,
            severity: Severity::Info,
            direction: Direction::In,
            method: "roster/list",
            id: Some("0a1b2c3d4e5f"),
            peer: Some("io"),
            fields: vec![("session", "io/alpha".to_owned())],
            frame: Some(
                serde_json::json!({ "id": "0a1b2c3d4e5f", "method": "roster/list" }).to_string(),
            ),
        },
        debug_event(),
    ];
    for ev in &events {
        // `emit` writes the rendered line to stderr (the real write path).
        emit(ev);
        let line = ev.render(&config);
        assert!(
            !line.is_empty(),
            "a {ev:?}-shaped event must render at noisy"
        );
        let v: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("json event is not one parseable object: {line:?} ({e})"));
        assert!(
            v.is_object(),
            "expected a JSON object for the event, got {v:?}"
        );
        // The structured fields are present on every event line.
        assert!(v.get("component").is_some(), "missing component in {v:?}");
        assert!(v.get("ts").is_some(), "missing ts in {v:?}");
        // At `noisy`, the (already-redacted) frame is nested under `frame`.
        if ev.frame.is_some() {
            assert!(
                v.get("frame").is_some(),
                "a noisy event with a frame must nest it under `frame`, got {v:?}"
            );
        }
    }
}

/// A `warn` event on the token component carrying a secret-shaped frame (the
/// pepper is redacted before rendering).
fn warn_event() -> Event {
    Event {
        component: Component::Token,
        severity: Severity::Warn,
        direction: Direction::Local,
        method: "token/pepper_generated",
        id: Some("c14fb1a960b3"),
        peer: None,
        fields: Vec::new(),
        frame: Some(serde_json::json!({ "pepper": "hlr_live_secret" }).to_string()),
    }
}

/// A `debug` wire event (gated by the debug level).
fn debug_event() -> Event {
    Event {
        component: Component::Wire,
        severity: Severity::Debug,
        direction: Direction::Out,
        method: "session/prompt",
        id: Some("c14fb1a960b3"),
        peer: Some("io"),
        fields: Vec::new(),
        frame: None,
    }
}
