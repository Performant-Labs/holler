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

use support::raw_ws::{connect_ws, go_live_as, hub_x25519_pubkey};
use support::{join, mint_token, roster_json, wait_for, write_sessions_toml, Body, Hub, StateDir, STARTUP_WAIT};

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
    let secret_bytes: [u8; 32] = std::fs::read(holler_body::x25519_identity::identity_path(state.path()))
        .expect("read the body's persisted x25519 identity")
        .try_into()
        .expect("the identity key file is exactly 32 raw bytes");
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();

    let mut admin = connect_ws(&ws_url).await;
    go_live_as(&mut admin, &token_id, &secret_bytes, &hub_pubkey, "admin-client", &ws_url, "admin").await;

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

    let secret_bytes: [u8; 32] = std::fs::read(holler_body::x25519_identity::identity_path(state.path()))
        .expect("read the joined credential's x25519 identity")
        .try_into()
        .expect("the identity key file is exactly 32 raw bytes");
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();

    let mut admin = connect_ws(&ws_url).await;
    go_live_as(&mut admin, &token_id, &secret_bytes, &hub_pubkey, "admin-only", &ws_url, "admin").await;

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

    let secret_bytes: [u8; 32] = std::fs::read(holler_body::x25519_identity::identity_path(state.path()))
        .expect("read the joined credential's x25519 identity")
        .try_into()
        .expect("the identity key file is exactly 32 raw bytes");
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();

    let mut admin = connect_ws(&ws_url).await;
    go_live_as(&mut admin, &token_id, &secret_bytes, &hub_pubkey, "admin-refusals", &ws_url, "admin").await;

    for (idx, method) in ["control/revoke", "control/test_drop", "session/prompt"].iter().enumerate() {
        let id = format!("b-refuse{idx:03}");
        let req = serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": {} });
        futures_util::SinkExt::send(&mut admin, tokio_tungstenite::tungstenite::Message::text(req.to_string()))
            .await
            .unwrap_or_else(|e| panic!("send {method} on the admin socket: {e}"));

        let reply = support::raw_ws::decode_next(&mut admin)
            .await
            .unwrap_or_else(|| panic!("admin socket must reply to {method}, not hang or close"));
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

    let secret_bytes: [u8; 32] = std::fs::read(holler_body::x25519_identity::identity_path(state.path()))
        .expect("read the joined credential's x25519 identity")
        .try_into()
        .expect("the identity key file is exactly 32 raw bytes");
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();

    let mut body_socket = connect_ws(&ws_url).await;
    go_live_as(&mut body_socket, &token_id, &secret_bytes, &hub_pubkey, "body-not-admin", &ws_url, "body").await;

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

    let secret_bytes: [u8; 32] = std::fs::read(holler_body::x25519_identity::identity_path(state.path()))
        .expect("read the joined credential's x25519 identity")
        .try_into()
        .expect("the identity key file is exactly 32 raw bytes");
    let hub_pubkey = hub_x25519_pubkey(&state);
    let ws_url = hub.ws_url();

    let mut admin = connect_ws(&ws_url).await;
    go_live_as(&mut admin, &token_id, &secret_bytes, &hub_pubkey, "admin-log-fields", &ws_url, "admin").await;

    wait_for(Duration::from_secs(5), || hub.log_text().contains("admin_connected").then_some(())).expect("admin_connected must be logged once the admin hello completes");

    // Close the socket cleanly so `run`'s loop ends and `admin_dropped` logs.
    drop(admin);

    wait_for(Duration::from_secs(5), || hub.log_text().contains("admin_dropped").then_some(())).expect("admin_dropped must be logged once the admin socket closes");

    let log = hub.log_text();
    let connected_line = log.lines().find(|l| l.contains("admin_connected")).expect("an admin_connected line exists");
    let dropped_line = log.lines().find(|l| l.contains("admin_dropped")).expect("an admin_dropped line exists");
    for field in ["token_id", "label", "peer", "sas"] {
        assert!(connected_line.contains(field), "AC 17: admin_connected must carry `{field}`: {connected_line}");
        assert!(dropped_line.contains(field), "AC 17: admin_dropped must carry `{field}`: {dropped_line}");
    }
    assert!(
        !log.contains("conn_connected"),
        "AC 17: an admin socket must never produce a body-path `conn_connected` line: {log}"
    );

    drop(hub);
}
