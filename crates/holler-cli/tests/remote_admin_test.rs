#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #508
//! RED tests for #508/#509 (remote hub-admin client — epic #506), against
//! real hub + body subprocesses and a hand-rolled admin socket (`raw_ws`,
//! full handshake with a caller-chosen `circuit/hello` role) — no mocks of
//! the circuit, the same discipline as `hub_hygiene_test.rs`.
//!
//! **These tests are authored before any admin/`--server` production code
//! exists (Phase 4, T-red).** Per `docs/handoffs/506-brief.md`'s clarifying
//! evidence, current `main`'s `hello_exchange` ignores `circuit/hello.role`
//! entirely (a hello that fails to parse as `Hello` — including one whose
//! `role` is `"admin"`, which is not a `HelloRole` variant yet — "falls
//! through unrefused", `circuit.rs`). So a hand-rolled socket that
//! authenticates with a live body's own credential and sends `role:
//! "admin"` is, on today's hub, indistinguishable from a second `body run`
//! for the same token: it supersedes the real body and can write roster
//! rows. That is exactly the supersede/roster-hijack hazard AC 3–4 pin, and
//! it is what makes `remote_admin_hello_must_not_supersede_the_connected_
//! body` and `remote_admin_hello_must_not_create_a_roster_row_from_
//! presence` fail today for the *right* reason — not a compile error, not a
//! missing `[[test]]` entry (see this crate's `Cargo.toml`).
//!
//! `roster_remote_server_flag_is_not_yet_recognized` (AC 9) pins the CLI
//! surface: `--server` does not exist on `roster` yet, so `clap` refuses it.

mod support;

use std::time::Duration;

use support::raw_ws::live_socket;
use support::remote_admin_rig::{mutate_credential, run, spawn_cmd, two_dir_rig};
use support::{hub_status_json, join, mint_token, roster_json, wait_for, write_sessions_toml, Body, Hub, StateDir, STARTUP_WAIT};

// ---------------------------------------------------------------------------
// AC 9 — CLI surface: `--server` does not exist yet.
// ---------------------------------------------------------------------------

/// `holler roster --server <url>` against an unjoined state dir must fail
/// cleanly with a "not joined" error, never succeed and never fail with a
/// clap usage error naming `--server` as unrecognized. AC 9 landed `--server`
/// on `roster` (and the other six admin-eligible verbs); this test now pins
/// the post-AC-9 failure mode against a body identity that was never
/// joined — `admin_client::load_identity`'s `NotJoined` error, per
/// `BodyIdentity::path` (`crates/holler-body/src/identity.rs`).
#[test]
fn roster_remote_server_flag_is_not_yet_recognized() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let ws_url = hub.ws_url();

    let out = support::holler_cmd(&state)
        .args(["roster", "--server", &ws_url, "--json"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "`roster --server` against an unjoined state dir must not succeed: {stderr}"
    );
    assert!(
        !stderr.contains("--server") && !stderr.to_lowercase().contains("unexpected argument"),
        "`--server` is a recognized flag now (AC 9) — clap must not refuse it as unknown: {stderr}"
    );
    assert!(
        stderr.contains("credential.json") || stderr.to_lowercase().contains("not joined") || stderr.to_lowercase().contains("run `holler body join`"),
        "expected a clean 'not joined' error naming the missing body credential, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// AC 3/4 — the supersede / roster-hijack hazard.
// ---------------------------------------------------------------------------

/// **AC 3.** With a body connected and one session live, a hand-rolled
/// socket that authenticates with that same body's persisted credential and
/// sends `circuit/hello role: "admin"` must never supersede it: the body
/// stays connected, and a `say` to its session afterward still gets a reply.
///
/// This is the RED the brief's own Test plan calls for ("T writes it so that
/// it would fail against a naive implementation that just sends `admin/*`
/// over a normal body-path socket"). On current `main` it fails: the second
/// connection is treated as an ordinary body reconnect for the same token,
/// so it supersedes the first (`registry.find_by_token` + `old.supersede()`,
/// `circuit.rs`'s `handle_authenticated`) — the body's log gets
/// `conn_superseded` and the original session is left disconnected.
#[tokio::test]
async fn remote_admin_hello_must_not_supersede_the_connected_body() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "body-1");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(&state, &[("alpha", &[])]);
    let body = Body::start(&state, &config);

    wait_for(STARTUP_WAIT, || {
        let v = roster_json(&state);
        let rows = v.get("rows")?.as_array()?;
        (rows.len() == 1 && rows[0]["conn_state"] == "connected").then_some(())
    })
    .expect("the body's one session shows connected in the roster before the admin hello");

    // The credential an `admin_client` (brief MO 7) is specced to reuse:
    // this body's own persisted X25519 identity, read straight off disk —
    // never a second, separately-minted credential.
    // Held alive (never read from again) for the rest of the test: its mere
    // presence, past the hello, is the hazard AC 3 pins.
    let _admin = live_socket(&state, &hub, &token_id, "admin-client", "admin").await;

    // Give a real supersede (if the hub does one) time to land, then check
    // the body is still the one holding the roster row and can still serve
    // a `say` — the AC 3 pin. `wait_for` here observes readiness, it does
    // not assert; the assertions below are unconditional.
    let _ = wait_for(Duration::from_secs(2), || {
        let text = body.log_text();
        text.contains("conn_superseded").then_some(())
    });

    assert!(
        !body.log_text().contains("conn_superseded"),
        "AC 3: an admin-role hello must never supersede the connected body, \
         but the body's log shows `conn_superseded`:\n{}",
        body.log_text()
    );

    let after = roster_json(&state);
    let rows = after["rows"].as_array().expect("roster --json carries rows");
    assert_eq!(rows.len(), 1, "AC 3: still exactly one session row after the admin hello: {after}");
    assert_eq!(
        rows[0]["conn_state"], "connected",
        "AC 3: the body's row must stay `connected` (not `reconnecting`/`gone`) after the admin hello: {after}"
    );

    let say_out = support::say(&state, "alpha", "ping-after-admin-hello");
    assert!(
        say_out.status.success(),
        "AC 3: `say` to the still-connected body's session must still get a reply after the admin hello \
         (exit {:?}): stdout={} stderr={}",
        say_out.status.code(),
        String::from_utf8_lossy(&say_out.stdout),
        String::from_utf8_lossy(&say_out.stderr),
    );

    body.stop(&state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 4.** An admin-role connection must never create (or touch) a roster
/// row, even if it sends `session/presence` — the hub does not yet branch on
/// `hello.role` at all, so on current `main` a hand-rolled admin-role socket
/// that sends a `session/presence` notification is serviced exactly like a
/// body's, and a fresh row appears. This must not happen once AC 4 lands;
/// today it does, which is this RED's failure.
#[tokio::test]
async fn remote_admin_hello_must_not_create_a_roster_row_from_presence() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "admin-only");
    // `join` redeems the token and persists a body-shaped credential under
    // `state`, without ever running a live body connection — this test's
    // socket is the *only* connection this token ever makes.
    join(&state, &state, &hub.ws_url(), &token_id, &secret);

    let mut admin = live_socket(&state, &hub, &token_id, "admin-only", "admin").await;

    assert!(
        roster_json(&state)["rows"].as_array().expect("rows array").is_empty(),
        "no roster row should exist right after an admin hello with no presence sent yet"
    );

    let presence = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "session/presence",
        "params": {
            "hostname": "admin-only",
            "sessions": [{ "name": "phantom", "harness": "opencode", "state": "idle", "mode": "spawn" }],
        },
    });
    futures_util::SinkExt::send(&mut admin, tokio_tungstenite::tungstenite::Message::text(presence.to_string()))
        .await
        .expect("send session/presence on the admin socket");

    let saw_row = wait_for(Duration::from_secs(3), || {
        let v = roster_json(&state);
        let rows = v.get("rows")?.as_array()?;
        (!rows.is_empty()).then_some(())
    });
    assert!(
        saw_row.is_none(),
        "AC 4: an admin-role connection must never create a roster row from `session/presence`, \
         but one appeared: {}",
        roster_json(&state)
    );

    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 5 — the allowlist: `-32601` for anything else, including on a decode
// failure (handoff-S REWORK item 1 / Q-1's fix in `circuit/admin.rs`).
// ---------------------------------------------------------------------------

/// **AC 5.** An admin socket sending any of three non-admin methods (two of
/// which are not in the wire catalog at all, and so previously failed to
/// decode and got silently dropped — handoff-S's Q-1, fixed by
/// `circuit/admin.rs::reply_decode_error`) must get an explicit `-32601`
/// reply carrying the request's own id, never a hang and never a bare
/// disconnect. Un-catalogued names (`control/revoke`, `control/test_drop`)
/// exercise the decode-failure reply path directly; a catalogued-but-wrong-
/// direction name (`session/prompt`, hub-to-body) exercises the
/// decodes-fine-but-unrecognised path (`handle_request`'s own `-32601` at
/// admin.rs:241-244).
#[tokio::test]
async fn admin_socket_refuses_every_non_admin_method_with_method_not_found() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "admin-refusals");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);

    let mut admin = live_socket(&state, &hub, &token_id, "admin-refusals", "admin").await;

    for (idx, method) in ["control/revoke", "control/test_drop", "session/prompt", "admin/revoke", "admin/hold", "admin/release"].iter().enumerate() {
        let id = format!("b-refuse{idx:03}");
        let req = serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": {} });
        futures_util::SinkExt::send(&mut admin, tokio_tungstenite::tungstenite::Message::text(req.to_string()))
            .await
            .unwrap_or_else(|e| panic!("send {method} on the admin socket: {e}"));

        // Advisory (handoff-S): wrapped in a timeout so a regression that
        // reverts the fix fails fast instead of hanging this test forever.
        let reply = tokio::time::timeout(Duration::from_secs(10), support::raw_ws::decode_next(&mut admin))
            .await
            .unwrap_or_else(|_| panic!("admin socket must reply to {method} within 10s, not hang"))
            .unwrap_or_else(|| panic!("admin socket must reply to {method}, not close"));
        let holler_proto::Envelope::Error { id: reply_id, error } = reply else {
            panic!("AC 5: {method} on an admin socket must get an error frame, got {reply:?}");
        };
        assert_eq!(error.code, -32601, "AC 5: {method} on an admin socket must be refused with -32601, got {error:?}");
        assert_eq!(reply_id.as_deref(), Some(id.as_str()), "AC 5: the refusal for {method} must carry back the request's own id");
    }

    drop(hub);
}

/// **AC 5 (second bullet).** `admin/roster` is legal only on an admin-role
/// connection (catalog `Direction::BodyToHub`, meaning "an admin client to
/// the hub" per `methods.rs`'s own doc comment — never a plain body). A
/// body-role socket sending it must get `-32601`, the same catch-all
/// `circuit.rs::handle_inbound`'s trailing `Envelope::Request { id, .. }` arm
/// already gives any other unrecognised-for-this-role request.
#[tokio::test]
async fn body_socket_sending_admin_roster_gets_method_not_found() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "body-not-admin");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);

    let mut body_socket = live_socket(&state, &hub, &token_id, "body-not-admin", "body").await;

    let req = serde_json::json!({ "jsonrpc": "2.0", "id": "b-notadmin1", "method": "admin/roster", "params": {} });
    futures_util::SinkExt::send(&mut body_socket, tokio_tungstenite::tungstenite::Message::text(req.to_string()))
        .await
        .expect("send admin/roster on a body-role socket");

    let reply = support::raw_ws::decode_next(&mut body_socket)
        .await
        .expect("the body socket must reply to admin/roster, not hang");
    let holler_proto::Envelope::Error { error, .. } = reply else {
        panic!("AC 5: admin/roster on a body-role socket must get an error frame, got {reply:?}");
    };
    assert_eq!(error.code, -32601, "AC 5: admin/roster on a body-role socket must be refused with -32601, got {error:?}");

    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 17 — `admin_connected`/`admin_dropped` log lines.
// ---------------------------------------------------------------------------

/// **AC 17.** Both the `admin_connected` and `admin_dropped` hub log lines
/// must each carry all four fields (`token_id`, `label`, `peer`, `sas`), and
/// an admin socket must never produce a `conn_connected` line — that event
/// name is the body-path connection log (`circuit.rs:271`), and an admin
/// socket never reaches that branch (MO 3-4).
#[tokio::test]
async fn admin_connected_and_dropped_log_lines_carry_all_four_fields() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = mint_token(&state, "admin-log-fields");
    join(&state, &state, &hub.ws_url(), &token_id, &secret);

    let admin = live_socket(&state, &hub, &token_id, "admin-log-fields", "admin").await;

    wait_for(Duration::from_secs(5), || hub.log_text().contains("admin_connected").then_some(())).expect("admin_connected must be logged once the admin hello completes");

    // Close the socket cleanly so `run`'s loop ends and `admin_dropped` logs.
    drop(admin);

    wait_for(Duration::from_secs(5), || hub.log_text().contains("admin_dropped").then_some(())).expect("admin_dropped must be logged once the admin socket closes");

    let log = hub.log_text();
    let connected_lines: Vec<&str> = log.lines().filter(|l| l.contains("admin_connected")).collect();
    let dropped_lines: Vec<&str> = log.lines().filter(|l| l.contains("admin_dropped")).collect();
    assert_eq!(connected_lines.len(), 1, "AC 17: exactly one admin_connected line: {log}");
    assert_eq!(dropped_lines.len(), 1, "AC 17: exactly one admin_dropped line: {log}");
    for field in ["token_id", "label", "peer", "sas"] {
        assert!(connected_lines[0].contains(field), "AC 17: admin_connected must carry `{field}`: {}", connected_lines[0]);
        assert!(dropped_lines[0].contains(field), "AC 17: admin_dropped must carry `{field}`: {}", dropped_lines[0]);
    }
    assert!(
        !log.contains("conn_connected"),
        "AC 17: an admin socket must never produce a body-path `conn_connected` line: {log}"
    );

    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 3 (end-to-end) / AC 4 (last_seen) / AC 11 — handoff-S round-2, required
// item 1-2 and its own dedicated AC 11 check.
// ---------------------------------------------------------------------------

/// **AC 3 end-to-end.** With a body connected and one session live, a real
/// `holler roster --server <ws>` run **from the body's own state dir**
/// exits 0 and lists that session; afterward the body is still connected,
/// and a later `say` to it still gets its reply — the brief's own named
/// test (`remote_admin_does_not_supersede_the_body`).
#[test]
fn remote_admin_does_not_supersede_the_body() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    let out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert!(out.status.success(), "AC 3: `roster --server` must exit 0: {}", String::from_utf8_lossy(&out.stderr));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).expect("roster --json is valid JSON");
    let rows = doc["rows"].as_array().expect("rows array");
    assert!(rows.iter().any(|r| r["name"] == "b/alpha"), "AC 3: the remote roster must list the body's session: {doc}");

    let after = roster_json(&hub_state);
    assert_eq!(after["rows"][0]["conn_state"], "connected", "AC 3: the body stays connected after the remote roster: {after}");

    let say_out = run(&hub_state, &["say", "b/alpha", "ping-after-remote-roster"]);
    assert!(say_out.status.success(), "AC 3: `say` after the remote roster must still get a reply: {}", String::from_utf8_lossy(&say_out.stderr));

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 4 (second half).** Admin traffic (`admin/roster`, `admin/status`, run
/// twice each with a pause between) must never move the body's own roster
/// row's `last_seen` (roster.rs:162) — the hazard-free half AC 4 pins beyond
/// "no row is created" (already covered by
/// `remote_admin_hello_must_not_create_a_roster_row_from_presence`).
#[test]
fn remote_admin_traffic_never_moves_the_bodys_last_seen() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    let before = roster_json(&hub_state);
    let last_seen_before = before["rows"][0]["last_seen"].as_u64().expect("last_seen is a number");

    std::thread::sleep(Duration::from_secs(2)); // last_seen is whole seconds; make a moved value observable.

    let roster_out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert!(roster_out.status.success(), "{}", String::from_utf8_lossy(&roster_out.stderr));
    let status_out = run(&body_state, &["hub", "status", "--server", &ws_url, "--json"]);
    assert!(status_out.status.success(), "{}", String::from_utf8_lossy(&status_out.stderr));

    let after = roster_json(&hub_state);
    let last_seen_after = after["rows"][0]["last_seen"].as_u64().expect("last_seen is a number");
    assert_eq!(
        last_seen_before, last_seen_after,
        "AC 4: admin/roster and admin/status traffic must never touch the body's last_seen: before={before} after={after}"
    );

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 11.** The remote form needs no hub state at all: the body dir this
/// test runs the remote CLI against has no `hub/` subtree (never created —
/// only `join` ever wrote to it, and `join` writes only `body/`), and the
/// remote call still succeeds.
#[test]
fn remote_form_needs_no_hub_state() {
    let (_hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    assert!(!body_state.hub().exists(), "AC 11: the body dir must have no hub/ subtree: {}", body_state.hub().display());

    let out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert!(out.status.success(), "AC 11: the remote form must succeed with only body state present: {}", String::from_utf8_lossy(&out.stderr));

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 6 — hold parity.
// ---------------------------------------------------------------------------

/// **AC 6.** A remote `say --server` to a locally `hold`-ed session gets the
/// same `session_held` refusal text and exit code as the local form (hold
/// itself stays local-only — out of scope per the brief — only the say path
/// is remote here).
#[test]
fn remote_say_to_a_held_session_gets_the_same_session_held_refusal_as_local() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    let held = run(&hub_state, &["hold", "b/alpha", "--reason", "deploy freeze"]);
    assert!(held.status.success(), "{}", String::from_utf8_lossy(&held.stderr));

    let local = run(&hub_state, &["say", "b/alpha", "hi"]);
    let remote = run(&body_state, &["say", "b/alpha", "hi", "--server", &ws_url]);

    assert_eq!(local.status.code(), remote.status.code(), "AC 6: local vs remote exit code must match for a held session");
    let local_err = String::from_utf8_lossy(&local.stderr);
    let remote_err = String::from_utf8_lossy(&remote.stderr);
    assert!(local_err.contains("session_held") && remote_err.contains("session_held"), "local={local_err} remote={remote_err}");
    assert!(remote_err.contains("deploy freeze"), "AC 6: the remote refusal must carry the hold reason: {remote_err}");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 7 — an unknown token, a wrong X25519 key, and a revoked token on the
// admin path each get -32002 exactly as the body path does, and count
// toward the existing lockout — no new code path around it.
// ---------------------------------------------------------------------------

/// **AC 7 (unknown token).** Corrupting the joined credential's `token_id`
/// to one the hub never minted reproduces the body path's own
/// `token_unknown` refusal on the admin path: exit 1, the same plain words,
/// and a lockout entry naming it.
#[test]
fn remote_admin_unknown_token_is_refused_and_counted_in_lockout() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);
    mutate_credential(&body_state, |v| v["token_id"] = serde_json::json!("tok_does_not_exist"));

    let out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert_eq!(out.status.code(), Some(1), "AC 7/13: exit 1 for an unknown token");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("this hub has no such token"), "{stderr}");

    let doc = hub_status_json(&hub_state);
    let peers = doc["lockout"]["peers"].as_array().expect("lockout.peers");
    assert!(
        peers.iter().any(|p| p["reasons"].get("token_unknown").is_some()),
        "AC 7: the refusal must count toward the existing lockout table: {doc}"
    );

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 7 (wrong X25519 key) / AC 13.** Overwriting the joined credential's
/// x25519 identity key reproduces a `static key mismatch` refusal: exit 1,
/// the body path's own words, and a lockout entry.
#[test]
fn remote_admin_wrong_key_is_refused_and_counted_in_lockout() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);
    let key_path = holler_body::x25519_identity::identity_path(body_state.path());
    std::fs::write(&key_path, [9u8; 32]).expect("overwrite the x25519 identity key");

    let out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert_eq!(out.status.code(), Some(1), "AC 7/13: exit 1 for a wrong X25519 key");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("does not match the key the hub registered at join") || stderr.contains("hub public key mismatch"),
        "{stderr}"
    );

    let doc = hub_status_json(&hub_state);
    let peers = doc["lockout"]["peers"].as_array().expect("lockout.peers");
    assert!(!peers.is_empty(), "AC 7: the refusal must count toward the existing lockout table: {doc}");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 7 (revoked token).** `hub token revoke` on a token an admin client is
/// still using reproduces `token_not_bound`: exit 1, the body path's own
/// words, and a lockout entry naming this token.
#[test]
fn remote_admin_revoked_token_is_refused_and_counted_in_lockout() {
    let (hub_state, body_state, hub, _body, token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);
    let revoke = run(&hub_state, &["hub", "token", "revoke", &token_id]);
    assert!(revoke.status.success(), "{}", String::from_utf8_lossy(&revoke.stderr));

    let out = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert_eq!(out.status.code(), Some(1), "AC 7/13: exit 1 for a revoked token");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no longer accepts this body's token"), "{stderr}");

    let doc = hub_status_json(&hub_state);
    let peers = doc["lockout"]["peers"].as_array().expect("lockout.peers");
    assert!(
        peers.iter().any(|p| p["token_ids"].as_array().is_some_and(|ids| ids.iter().any(|t| t["id"] == token_id.as_str()))),
        "AC 7: the revoked token's refusal must count toward its own lockout entry: {doc}"
    );

    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 8 — concurrency: 2 remote + 1 local `say`, with and without `--queue`.
// ---------------------------------------------------------------------------

/// **AC 8 (no `--queue`).** Two remote `say`s plus one local `say`, all
/// racing the same session: each contender exits either 0 with a reply or 1
/// with `session_busy`, at least one exits 0, none panics or hangs past its
/// own timeout, and afterward the body is still connected and a fourth
/// `say` succeeds.
#[test]
fn remote_and_local_say_concurrency_matches_local_only_story() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &["--slow", "--chunks", "5"])]);

    let a = spawn_cmd(&body_state, &["say", "b/alpha", "hi-A", "--server", &ws_url]);
    let b = spawn_cmd(&body_state, &["say", "b/alpha", "hi-B", "--server", &ws_url]);
    let c = spawn_cmd(&hub_state, &["say", "b/alpha", "hi-C"]);

    let outs: Vec<_> = [a, b, c].into_iter().map(|ch| ch.wait_with_output().expect("wait on holler")).collect();
    let mut ok = 0;
    for (label, o) in ["remote A", "remote B", "local"].iter().zip(&outs) {
        let stderr = String::from_utf8_lossy(&o.stderr);
        match o.status.code() {
            Some(0) => ok += 1,
            Some(1) => assert!(stderr.contains("session_busy"), "AC 8: {label} refused for a reason other than session_busy: {stderr}"),
            other => panic!("AC 8: {label} exited unexpectedly ({other:?}): {stderr}"),
        }
    }
    assert!(ok >= 1, "AC 8: at least one of the three contenders must get the reply");

    wait_for(STARTUP_WAIT, || {
        roster_json(&hub_state)["rows"].as_array()?.iter().any(|r| r["name"] == "b/alpha" && r["state"] == "idle").then_some(())
    })
    .expect("AC 8: the session settles back to idle after the contention");
    let fourth = run(&hub_state, &["say", "b/alpha", "hi-D"]);
    assert!(fourth.status.success(), "AC 8: a fourth say after the contention must succeed: {}", String::from_utf8_lossy(&fourth.stderr));

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 8 (`--queue` on all three).** All three contenders exit 0 with a
/// reply.
#[test]
fn remote_and_local_say_with_queue_all_three_succeed() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &["--slow", "--chunks", "3"])]);

    let a = spawn_cmd(&body_state, &["say", "b/alpha", "hi-A", "--server", &ws_url, "--queue"]);
    let b = spawn_cmd(&body_state, &["say", "b/alpha", "hi-B", "--server", &ws_url, "--queue"]);
    let c = spawn_cmd(&hub_state, &["say", "b/alpha", "hi-C", "--queue"]);

    let outs: Vec<_> = [a, b, c].into_iter().map(|ch| ch.wait_with_output().expect("wait on holler")).collect();
    for (label, o) in ["remote A", "remote B", "local"].iter().zip(&outs) {
        assert!(o.status.success(), "AC 8: with --queue, {label} must exit 0: {}", String::from_utf8_lossy(&o.stderr));
    }

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 10 — `--json` byte parity between the remote and local forms, masking
// exactly the brief's named volatile fields.
// ---------------------------------------------------------------------------

/// The last non-empty line of `stderr` — the `error: …` line every `*_cmd.rs`
/// refusal prints, after this process's own `logging_started` banner and (on
/// the remote/WS path only) the transport's own per-frame `component: wire`
/// debug lines (`HOLLER_DEBUG=quiet`'s "shape only" debug logging, `log.rs`'s
/// module doc) — a real, expected difference between the local Unix-socket
/// client (no per-frame wire logging at all) and the remote WS client, not
/// part of AC 10's own parity claim (the *rendered refusal*, not this
/// process's own diagnostic noise).
fn error_line(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .last()
        .unwrap_or_default()
        .to_string()
}

/// Replace the brief's named volatile field(s) with a fixed placeholder so
/// two otherwise-identical `--json` documents compare equal.
fn masked(verb: &str, mut v: serde_json::Value) -> serde_json::Value {
    let mask_field = |row: &mut serde_json::Value, field: &str| {
        if let Some(o) = row.as_object_mut() {
            if o.contains_key(field) {
                o.insert(field.to_string(), serde_json::json!("<masked>"));
            }
        }
    };
    match verb {
        "roster" => {
            if let Some(rows) = v.get_mut("rows").and_then(|r| r.as_array_mut()) {
                for row in rows.iter_mut() {
                    mask_field(row, "last_seen");
                    mask_field(row, "last_update_at");
                }
            }
        }
        "wait" => {
            if let Some(rows) = v.get_mut("rows").and_then(|r| r.as_array_mut()) {
                for row in rows.iter_mut() {
                    mask_field(row, "age_secs");
                }
            }
        }
        "hub_status" => {
            if let Some(peers) = v.pointer_mut("/lockout/peers").and_then(|p| p.as_array_mut()) {
                for p in peers.iter_mut() {
                    mask_field(p, "retry_after_secs");
                }
            }
        }
        _ => {}
    }
    v
}

fn json_body(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("invalid --json stdout ({e}): {}", String::from_utf8_lossy(&out.stdout)))
}

/// **AC 10.** `roster --json` and `hub status --json` from the remote form
/// are byte-identical to the local form against the same hub state, once
/// each verb's own named volatile field is masked.
#[test]
fn remote_roster_and_hub_status_json_match_local_after_masking() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    let local = run(&hub_state, &["roster", "--json"]);
    let remote = run(&body_state, &["roster", "--server", &ws_url, "--json"]);
    assert!(local.status.success() && remote.status.success());
    assert_eq!(masked("roster", json_body(&local)), masked("roster", json_body(&remote)), "AC 10: roster --json parity");

    let local = run(&hub_state, &["hub", "status", "--json"]);
    let remote = run(&body_state, &["hub", "status", "--server", &ws_url, "--json"]);
    assert!(local.status.success() && remote.status.success());
    assert_eq!(masked("hub_status", json_body(&local)), masked("hub_status", json_body(&remote)), "AC 10: hub status --json parity");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 10.** `say --json` from the remote form matches the local form
/// (stub-acp's reply text is deterministic per chunk count, so only
/// `elapsed_ms` needs masking), and `wait --json`/`hub query status --json`
/// match too.
#[test]
fn remote_say_wait_and_hub_query_json_match_local_after_masking() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &["--chunks", "2"])]);

    let local = run(&hub_state, &["say", "b/alpha", "parity-ping", "--json"]);
    assert!(local.status.success(), "{}", String::from_utf8_lossy(&local.stderr));
    wait_for(STARTUP_WAIT, || {
        roster_json(&hub_state)["rows"].as_array()?.iter().any(|r| r["name"] == "b/alpha" && r["state"] == "idle").then_some(())
    })
    .expect("idle before the remote say");
    let remote = run(&body_state, &["say", "b/alpha", "parity-ping", "--server", &ws_url, "--json"]);
    assert!(remote.status.success(), "{}", String::from_utf8_lossy(&remote.stderr));
    // `message.messageId` is a fresh random id per turn even for two
    // identical **local** runs (confirmed: it is not stable), so per the
    // brief's own AC 10 rule ("if T finds another field that differs
    // between two identical local runs... the masked set is extended by
    // that one field only") it joins `elapsed_ms` in the masked set here.
    let mut l = json_body(&local);
    let mut r = json_body(&remote);
    for doc in [&mut l, &mut r] {
        doc["elapsed_ms"] = serde_json::json!("<masked>");
        doc["message"]["messageId"] = serde_json::json!("<masked>");
    }
    assert_eq!(l, r, "AC 10: say --json parity (elapsed_ms/message.messageId masked)");

    let local = run(&hub_state, &["--json", "wait", "b/alpha"]);
    let remote = run(&body_state, &["--json", "wait", "b/alpha", "--server", &ws_url]);
    assert!(local.status.success() && remote.status.success());
    assert_eq!(masked("wait", json_body(&local)), masked("wait", json_body(&remote)), "AC 10: wait --json parity");

    let local = run(&hub_state, &["--json", "hub", "query", "status"]);
    let remote = run(&body_state, &["--json", "hub", "query", "--server", &ws_url, "status"]);
    assert!(local.status.success() && remote.status.success());
    assert_eq!(json_body(&local), json_body(&remote), "AC 10: hub query status --json parity (no field is masked)");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 10.** `interrupt --json` and `answer --json` refusals (an unknown
/// session, in both cases) match the local form byte for byte on stderr and
/// exit code.
#[test]
fn remote_interrupt_and_answer_refusals_match_local() {
    let (hub_state, body_state, hub, body, _token_id, ws_url) = two_dir_rig(&[("alpha", &[])]);

    let local = run(&hub_state, &["--json", "interrupt", "b/nonexistent"]);
    let remote = run(&body_state, &["--json", "interrupt", "b/nonexistent", "--server", &ws_url]);
    assert_eq!(local.status.code(), remote.status.code(), "AC 10: interrupt refusal exit code parity");
    assert_eq!(error_line(&local), error_line(&remote), "AC 10: interrupt refusal message parity");

    let local = run(&hub_state, &["--json", "answer", "b/nonexistent", "yes"]);
    let remote = run(&body_state, &["--json", "answer", "b/nonexistent", "yes", "--server", &ws_url]);
    assert_eq!(local.status.code(), remote.status.code(), "AC 10: answer refusal exit code parity");
    assert_eq!(error_line(&local), error_line(&remote), "AC 10: answer refusal message parity");

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

// ---------------------------------------------------------------------------
// AC 13 — the remaining failure words and exit codes (the not-joined and
// session_held cases are already pinned above/elsewhere).
// ---------------------------------------------------------------------------

/// **AC 13.** `--server ws://10.0.0.5:1` (plaintext, non-loopback) is refused
/// before any dial, exit 3, even against a state dir that was never joined.
/// A loopback address nothing is listening on gets exit 1, "could not reach
/// the hub at …".
#[test]
fn remote_server_policy_and_unreachable_failures_get_the_documented_exit_codes() {
    let unjoined = StateDir::new();
    let out = run(&unjoined, &["roster", "--server", "ws://10.0.0.5:1", "--json"]);
    assert_eq!(out.status.code(), Some(3), "AC 13: a non-loopback plaintext --server is exit 3: {}", String::from_utf8_lossy(&out.stderr));

    let (_hub_state, body_state, hub, body, _token_id, _ws_url) = two_dir_rig(&[("alpha", &[])]);
    let dead_addr = support::hold_rig::free_addr();
    let out = run(&body_state, &["roster", "--server", &format!("ws://{dead_addr}"), "--json"]);
    assert_eq!(out.status.code(), Some(1), "AC 13: an unreachable server is exit 1: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("could not reach the hub at"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    body.stop(&body_state, Duration::from_secs(10));
    drop(hub);
}

/// **AC 13.** An ambiguous session name on the remote path exits 2, matching
/// the local form's exact wording (two bodies, each hosting a session named
/// `alpha`).
#[test]
fn remote_ambiguous_session_exits_2_matching_local() {
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);

    let body1_state = StateDir::new();
    let (token1, secret1) = mint_token(&hub_state, "b1");
    join(&body1_state, &hub_state, &hub.ws_url(), &token1, &secret1);
    let config1 = write_sessions_toml(&body1_state, &[("alpha", &[])]);
    let body1 = Body::start(&body1_state, &config1);

    let body2_state = StateDir::new();
    let (token2, secret2) = mint_token(&hub_state, "b2");
    join(&body2_state, &hub_state, &hub.ws_url(), &token2, &secret2);
    let config2 = write_sessions_toml(&body2_state, &[("alpha", &[])]);
    let body2 = Body::start(&body2_state, &config2);

    for want in ["b1/alpha", "b2/alpha"] {
        wait_for(STARTUP_WAIT, || {
            roster_json(&hub_state)["rows"].as_array()?.iter().any(|r| r["name"] == want && r["state"] == "idle").then_some(())
        })
        .unwrap_or_else(|| panic!("{want} never came up idle"));
    }

    let ws_url = hub.ws_url();
    let local = run(&hub_state, &["say", "alpha", "hi"]);
    let remote = run(&body1_state, &["say", "alpha", "hi", "--server", &ws_url]);
    assert_eq!(local.status.code(), Some(2), "local ambiguous form must exit 2: {}", String::from_utf8_lossy(&local.stderr));
    assert_eq!(remote.status.code(), Some(2), "AC 13: remote ambiguous form must exit 2 matching local: {}", String::from_utf8_lossy(&remote.stderr));
    assert_eq!(error_line(&local), error_line(&remote), "AC 13: ambiguous stderr text must match local");

    body1.stop(&body1_state, Duration::from_secs(10));
    body2.stop(&body2_state, Duration::from_secs(10));
    drop(hub);
}
