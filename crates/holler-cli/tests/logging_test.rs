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
//! management lands with the hub-serving story). This test therefore exercises
//! the *same leaf-agnostic contract* through `roster`, a currently-present leaf
//! (a "not implemented" stub, exit 1). The contract (stderr-only logs, clean
//! stdout, the component column, the banner) is identical for every leaf, so
//! pinning it here is equivalent.
//!
//! Two contracts are **documented but not yet GREEN** because they depend on
//! code this story does not wire up:
//! - `warn_is_always_emitted_independently_of_debug_level` — needs a leaf to
//!   actually call `holler_proto::log::emit` with a `warn` (the #144 module
//!   provides `emit`; call sites land with the hub/body stories). RED.
//! - `json_format_renders_events_as_one_object_per_line` — needs at least one
//!   JSON log event on stderr. The banner is *always* text by design (the one
//!   human-readable line), so with no events yet the JSON contract is vacuous.
//!   RED.

use std::iter::Iterator;

use assert_cmd::Command;
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
// gated). At `--debug none` a warn still shows and a debug does not.
//
// RED until some leaf calls `holler_proto::log::emit` with a `warn` severity.
#[test]
#[allow(unused_variables)]
fn warn_is_always_emitted_independently_of_debug_level() {
    let out = holler()
        .env("HOLLER_DEBUG", "none")
        .env("HOLLER_LOG_FORMAT", "text")
        .arg("roster")
        .output()
        .expect("run holler");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // A warn is always on…
    assert!(
        stderr.contains("WARN"),
        "RED: a `warn` event is always emitted regardless of --debug; none is wired yet\n got: {stderr}"
    );
    // …and a debug is off at the default level.
    assert!(
        !stderr.contains(" DEBUG "),
        "a `debug` event must be suppressed at --debug none\ngot:\n{stderr}"
    );
}

// With `--log-format json`, every log event on stderr is one JSON object
// (machine-parseable, one object per line). The banner is deliberately *not*
// JSON (it is the one human-readable line), so it is excluded here.
//
// RED until a JSON log event exists on stderr (needs an `emit` call site).
#[test]
#[allow(unused_variables)]
fn json_format_renders_events_as_one_object_per_line() {
    let out = holler()
        .env("HOLLER_DEBUG", "noisy")
        .env("HOLLER_LOG_FORMAT", "json")
        .arg("roster")
        .output()
        .expect("run holler");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Exclude the banner (always text by design) and any non-log lines.
    let event_lines: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !l.contains("logging_started"))
        .collect();
    assert!(
        !event_lines.is_empty(),
        "RED: expected at least one JSON log event on stderr (none wired yet)\n got: {stderr}"
    );
    for line in event_lines {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("stderr line is not a single JSON object: {line:?} ({e})"));
        assert!(v.is_object(), "expected a JSON object, got {v:?}");
    }
}
