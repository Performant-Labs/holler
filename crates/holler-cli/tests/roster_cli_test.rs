#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #186
//! `holler roster` (issue #186, CLI surface): the live roster read over the
//! control socket, shown as a table (or `--json`). Real subprocesses, real
//! loopback sockets — no mocks of the circuit (the same discipline as
//! `body_join_test.rs` / `body_run_test.rs`).
//!
//! Readiness is always **observed** — `roster_json`'s row count — via
//! [`support::wait_for`], never a blind sleep (ADR 0002).
//!
//! The body this story drives advertises its configured sessions (the #182
//! connection loop's presence heartbeat), so a body configured with two
//! sessions shows two roster rows — the spec's `roster_cli_shows_two_sessions_
//! from_one_body`.

mod support;

use std::time::Duration;

use serde_json::Value;

use support::{holler_cmd, join, mint_token, roster_json, wait_for, Body, Hub, StateDir};

fn run(state: &StateDir, args: &[&str]) -> (i32, String, String) {
    let out = holler_cmd(state)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn roster_cli_shows_two_sessions_from_one_body() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &hub.ws_url(), &token_id, &secret);
    // Two spawn-mode sessions for the one body (pointed at the stub agent so
    // the body can actually bring them up if it spawns).
    let config = support::write_sessions_toml(&state, &[("alpha", &[]), ("beta", &[])]);
    let body = Body::start(&state, &config);

    // Observe (never sleep): wait until the hub's roster shows both sessions.
    wait_for(Duration::from_secs(15), || {
        let v = roster_json(&state);
        let rows = v.get("rows").and_then(|r| r.as_array())?;
        if rows.len() == 2 {
            Some(())
        } else {
            None
        }
    })
    .expect("the roster shows two sessions from the one body");

    let (code, stdout, stderr) = run(&state, &["roster"]);
    assert_eq!(code, 0, "roster exits 0: {stderr}");
    assert!(
        stdout.contains("alpha"),
        "the roster table shows the `alpha` session:\n{stdout}"
    );
    assert!(
        stdout.contains("beta"),
        "the roster table shows the `beta` session:\n{stdout}"
    );
    // The table has the spec's columns.
    for col in ["SESSION", "HARNESS", "MODE", "STATE", "CONN", "HOSTNAME", "PENDING"] {
        assert!(
            stdout.contains(col),
            "the roster table has a {col} column:\n{stdout}"
        );
    }
    body.stop(&state, Duration::from_secs(10));
    drop(hub);
}

#[test]
fn roster_json_shape() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &hub.ws_url(), &token_id, &secret);
    let config = support::write_sessions_toml(&state, &[("alpha", &[])]);
    let body = Body::start(&state, &config);

    wait_for(Duration::from_secs(15), || {
        let v = roster_json(&state);
        let rows = v.get("rows").and_then(|r| r.as_array())?;
        rows.iter().find(|r| r.get("name").and_then(Value::as_str) == Some("alpha")).map(|_| ())
    })
    .expect("the roster --json carries the body's session row");

    let v = roster_json(&state);
    let rows = v.get("rows").and_then(|r| r.as_array()).expect("roster --json carries a rows array");
    let row = rows.iter().find(|r| r.get("name").and_then(Value::as_str) == Some("alpha")).expect("the alpha row");
    // The JSON row carries the spec's fields.
    for key in ["name", "harness", "mode", "state", "conn_state", "client_id", "hostname", "last_seen"] {
        assert!(
            row.get(key).is_some(),
            "the roster --json row carries `{key}`: {row}"
        );
    }
    assert_eq!(row["state"].as_str(), Some("idle"), "a spawned-but-quiet session is `idle` in the JSON roster");
    assert_eq!(row["conn_state"].as_str(), Some("connected"));

    body.stop(&state, Duration::from_secs(10));
    drop(hub);
}
