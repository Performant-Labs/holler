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

    // The row's name is qualified `<label>/<session>` (ADR 0005 §2, issue
    // #236): `mint_token` above minted the `body-1` label, so the row is
    // `body-1/alpha`, not the bare `alpha` the body itself advertised.
    wait_for(Duration::from_secs(15), || {
        let v = roster_json(&state);
        let rows = v.get("rows").and_then(|r| r.as_array())?;
        rows.iter().find(|r| r.get("name").and_then(Value::as_str) == Some("body-1/alpha")).map(|_| ())
    })
    .expect("the roster --json carries the body's session row");

    let v = roster_json(&state);
    let rows = v.get("rows").and_then(|r| r.as_array()).expect("roster --json carries a rows array");
    let row = rows.iter().find(|r| r.get("name").and_then(Value::as_str) == Some("body-1/alpha")).expect("the alpha row");
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

/// `holler roster --prefix PREFIX` (issue #236, ADR 0005 §4): a row is
/// listed when its name is exactly `PREFIX` or nests under it (`PREFIX/…`).
/// Two bodies mint distinct labels (`io`, `media`) against the same hub, so
/// their rows land under different qualified prefixes (`io/alpha`,
/// `media/gamma`) — `--prefix io` must return `io/alpha` alone, never
/// `media/gamma`, proving the filter is a real transitive-prefix match on
/// the label boundary and not a plain substring search.
#[test]
fn roster_cli_prefix_filters_by_label() {
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);

    let io_body_state = StateDir::new();
    let (io_token, io_secret) = mint_token(&hub_state, "io");
    join(&io_body_state, &hub.ws_url(), &io_token, &io_secret);
    let io_config = support::write_sessions_toml(&io_body_state, &[("alpha", &[])]);
    let io_body = Body::start(&io_body_state, &io_config);

    let media_body_state = StateDir::new();
    let (media_token, media_secret) = mint_token(&hub_state, "media");
    join(&media_body_state, &hub.ws_url(), &media_token, &media_secret);
    let media_config = support::write_sessions_toml(&media_body_state, &[("gamma", &[])]);
    let media_body = Body::start(&media_body_state, &media_config);

    // Observe (never sleep): wait until both bodies' rows have landed.
    wait_for(Duration::from_secs(15), || {
        let v = roster_json(&hub_state);
        let rows = v.get("rows").and_then(|r| r.as_array())?;
        let names: Vec<&str> = rows.iter().filter_map(|r| r.get("name").and_then(Value::as_str)).collect();
        if names.contains(&"io/alpha") && names.contains(&"media/gamma") {
            Some(())
        } else {
            None
        }
    })
    .expect("the roster shows both bodies' sessions");

    // `--prefix io` (no trailing slash): only `io/alpha`.
    let (code, stdout, stderr) = run(&hub_state, &["--json", "roster", "--prefix", "io"]);
    assert_eq!(code, 0, "roster --prefix io exits 0: {stderr}");
    let v: Value = serde_json::from_str(&stdout).expect("roster --prefix io --json is a JSON object");
    let rows = v.get("rows").and_then(|r| r.as_array()).expect("rows array");
    let names: Vec<&str> = rows.iter().filter_map(|r| r.get("name").and_then(Value::as_str)).collect();
    assert_eq!(names, vec!["io/alpha"], "--prefix io lists only the io/ row, never media/gamma");

    // `--prefix io/` (trailing slash, the ADR's own example spelling):
    // identical result.
    let (code, stdout, stderr) = run(&hub_state, &["--json", "roster", "--prefix", "io/"]);
    assert_eq!(code, 0, "roster --prefix io/ exits 0: {stderr}");
    let v: Value = serde_json::from_str(&stdout).expect("roster --prefix io/ --json is a JSON object");
    let rows = v.get("rows").and_then(|r| r.as_array()).expect("rows array");
    let names: Vec<&str> = rows.iter().filter_map(|r| r.get("name").and_then(Value::as_str)).collect();
    assert_eq!(names, vec!["io/alpha"], "a trailing slash on --prefix behaves the same as none");

    // `--prefix media`: only `media/gamma`.
    let (code, stdout, stderr) = run(&hub_state, &["--json", "roster", "--prefix", "media"]);
    assert_eq!(code, 0, "roster --prefix media exits 0: {stderr}");
    let v: Value = serde_json::from_str(&stdout).expect("roster --prefix media --json is a JSON object");
    let rows = v.get("rows").and_then(|r| r.as_array()).expect("rows array");
    let names: Vec<&str> = rows.iter().filter_map(|r| r.get("name").and_then(Value::as_str)).collect();
    assert_eq!(names, vec!["media/gamma"], "--prefix media lists only the media/ row, never io/alpha");

    // No `--prefix` at all: both rows still come back (the flag narrows, it
    // never becomes the only way to list).
    let (code, stdout, stderr) = run(&hub_state, &["--json", "roster"]);
    assert_eq!(code, 0, "plain roster exits 0: {stderr}");
    let v: Value = serde_json::from_str(&stdout).expect("roster --json is a JSON object");
    let rows = v.get("rows").and_then(|r| r.as_array()).expect("rows array");
    assert_eq!(rows.len(), 2, "without --prefix both bodies' rows are listed");

    io_body.stop(&io_body_state, Duration::from_secs(10));
    media_body.stop(&media_body_state, Duration::from_secs(10));
    drop(hub);
}
