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

/// `holler roster --server <url>` must be rejected by `clap` as an unknown
/// flag on current `main` — there is no `--server` on any of the seven verbs
/// yet (brief `Files`: `crates/holler-cli/src/cli.rs`, "`--server` on 7
/// structs", not yet applied). Once AC 9 lands this exact invocation must
/// instead fail differently (dial a real hub, or a clean "could not reach"),
/// never with a clap usage error — this test's failure mode is the pin.
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
        "AC 9 is not implemented yet — `roster --server` must not succeed on current main: {stderr}"
    );
    assert!(
        stderr.contains("--server") || stderr.to_lowercase().contains("unexpected argument"),
        "expected a clap usage error naming the unrecognized `--server` flag, got: {stderr}"
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
