#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #315
//! Platform & machine parity (issue #315, catalog group `platform`,
//! `hlr-1400`–`1404`).
//!
//! Windows is deferred (see `docs/testing.md`'s "Windows is deferred"
//! section and issue #308), so this file pins what parity `holler` *does*
//! guarantee across Linux and macOS — the five **automated** (`Type: auto`)
//! cases in the catalog's small platform group:
//!
//! - [`localhost_name_refused`] (hlr-1400) — `--listen localhost:0` is
//!   refused; only a numeric loopback literal is ever accepted.
//! - [`ipv6_loopback_or_skip`] (hlr-1401) — `[::1]` works end to end when
//!   the environment can bind it; **skips**, never fails, when it cannot.
//! - [`self_signed_wss_fails_closed`] (hlr-1402) — `wss://` fails closed
//!   against a self-signed cert (a local `rcgen` cert, no real CA involved).
//! - [`state_dir_layout_and_perms`] (hlr-1403) — state dir paths, lock
//!   files, and 0600 permissions, parameterized (`rstest`) over the file set
//!   that actually carries a permission contract.
//! - [`no_control_socket_message`] (hlr-1404) — an unavailable control
//!   socket reports a clear message, exercised deterministically via the
//!   test-only `HOLLER_TEST_NO_CONTROL_SOCKET=1` env override (added by this
//!   issue to `holler_hub::control::exchange_with_timeout`, mirroring the
//!   existing `HOLLER_TEST_HOOKS=1` convention).
//!
//! The two **manual**, real-hardware cross-machine checkpoints
//! (`hlr-1405`/`1406` — join/run/ping/roster/reconnect/revoke over a
//! tailnet, and the say/interrupt/reprompt session checkpoints) are **out of
//! scope here** and tracked separately in issue #316; nothing in this file
//! exercises or claims that coverage.

use std::io::BufRead;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Child, Stdio};
use std::time::Duration;

use rstest::rstest;
use serde_json::Value;

mod support;
use support::{holler_cmd, Hub, StateDir};

// ---------------------------------------------------------------------------
// hlr-1400 — `localhost` is never resolved
// ---------------------------------------------------------------------------

/// `hub serve --listen localhost:0` is refused exit 3: `validate_loopback`
/// (`crates/holler-hub/src/serve.rs`) parses `--listen` with
/// `SocketAddr::from_str`, which never performs a DNS/hosts-file lookup — a
/// hostname (including `localhost`) simply fails to parse as a `SocketAddr`,
/// so the name is refused (with a message naming the bad address and, by
/// construction, steering the operator toward a numeric loopback literal
/// like `127.0.0.1:0` or `[::1]:0`) before any resolution could even happen.
/// Only `127.0.0.1:0` / `[::1]:0` (and other loopback IP literals) work.
#[test]
fn localhost_name_refused() {
    let state = StateDir::new();
    let out = holler_cmd(&state)
        .args(["hub", "serve", "--listen", "localhost:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn a `localhost`-name hub")
        .wait_with_output()
        .expect("wait on the `localhost`-name hub");

    assert_eq!(
        out.status.code(),
        Some(3),
        "a `--listen localhost:0` must be refused exit 3; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("localhost:0 is not a valid host:port address"),
        "the refusal must name the bad address (the hint toward a numeric \
         loopback literal); got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// hlr-1401 — IPv6 loopback works end to end, or skips
// ---------------------------------------------------------------------------

/// IPv6 loopback `[::1]` works end to end (a real `hub serve --listen
/// [::1]:0` plus a real `body join` over it) when the environment can bind
/// it; **skips** (never fails) when it cannot.
///
/// The availability probe binds `[::1]:0` directly with
/// `std::net::TcpListener` — cheap, synchronous, and the same underlying
/// call the hub itself makes — before paying for a subprocess spawn, so an
/// environment with IPv6 disabled (some sandboxes/CI runners) reports a
/// skip rather than a false failure. This mirrors the only sanctioned
/// "unavailable, don't fail" idiom in this harness (`wire_selftest`'s
/// closed-port canary tolerates environmental flakiness by drawing several
/// ports rather than asserting on one) — here the analogous move is probing
/// before committing to the real end-to-end path.
#[test]
fn ipv6_loopback_or_skip() {
    if std::net::TcpListener::bind("[::1]:0").is_err() {
        eprintln!(
            "SKIP ipv6_loopback_or_skip: [::1] is not available in this \
             environment (hlr-1401 skips, never fails, on IPv6 unavailability)"
        );
        return;
    }

    let state = StateDir::new();
    let (mut child, addr) = start_hub_on(&state, "[::1]:0");
    let ws_url = format!("ws://{addr}");

    let (token_id, secret) = support::mint_token(&state, "ipv6");
    support::join(&state, &state, &ws_url, &token_id, &secret);

    let doc = support::hub_status_json(&state);
    let listening = doc["listening"].as_array().expect("listening is an array");
    assert!(
        listening.iter().any(|a| a.as_str() == Some(addr.as_str())),
        "status.listening must include the IPv6 bound {addr}; got {listening:?}"
    );

    stop_hub_child(&mut child);
}

/// Spawn `holler hub serve --listen <listen>` and wait (≤10 s) for its
/// `{"event":"listening","addr":…}` line, returning the child and the bound
/// `addr` string exactly as reported (already bracketed for IPv6, e.g.
/// `[::1]:54321`). Generalizes `support::Hub::start` (hardcoded to
/// `127.0.0.1:0`) so this file can drive an arbitrary loopback listen
/// address with the same observed-readiness contract (ADR 0002: never a
/// blind sleep).
fn start_hub_on(state: &StateDir, listen: &str) -> (Child, String) {
    let mut cmd = holler_cmd(state);
    cmd.args(["hub", "serve", "--listen", listen])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    support::make_own_process_group(&mut cmd);
    let mut child = cmd.spawn().expect("spawn `holler hub serve`");

    let stderr = child.stderr.take().expect("hub stderr is piped");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stderr);
        let mut line = String::new();
        let mut sent = false;
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let v: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if !sent && v.get("event").and_then(|e| e.as_str()) == Some("listening") {
                sent = true;
                let addr = v.get("addr").and_then(|a| a.as_str()).unwrap_or_default();
                let _ = tx.send(addr.to_string());
            }
        }
    });
    let addr = rx.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| {
        let _ = child.kill();
        panic!("hub did not report a listening addr within 10s for --listen {listen}");
    });
    (child, addr)
}

/// Tear a [`start_hub_on`] child down: SIGINT (graceful) then a bounded wait,
/// then `kill_tree` as the hard fallback — the same two-step teardown
/// `support::Hub::stop` uses, reimplemented here only because `Hub`'s fields
/// are private to `support` and this file's hub is not a `support::Hub`
/// (it binds an address `Hub::start` does not accept).
fn stop_hub_child(child: &mut Child) {
    let pid = child.id() as i32;
    #[cfg(unix)]
    unsafe {
        libc::kill(-pid, libc::SIGINT);
    }
    let _ = support::wait_for(Duration::from_secs(5), || child.try_wait().ok().flatten());
    support::kill_tree(child);
}

// ---------------------------------------------------------------------------
// hlr-1402 — `wss://` fails closed on a self-signed cert
// ---------------------------------------------------------------------------

/// `wss://` dialing uses the OS trust store (the workspace's
/// `rustls-tls-native-roots`), so a self-signed cert must fail closed with a
/// clear error rather than being silently trusted. Automated against a
/// throwaway local `rcgen`-generated cert (issue #315 added `rcgen` and
/// `tokio-rustls` as `holler-cli` dev-dependencies for exactly this test) and
/// a minimal TLS-terminating listener stood up in-process — no real CA, no
/// network access.
#[test]
fn self_signed_wss_fails_closed() {
    let (port, server) = start_self_signed_tls_listener();

    let state = StateDir::new();
    let out = holler_cmd(&state)
        .args([
            "body",
            "join",
            "--server",
            &format!("wss://127.0.0.1:{port}"),
            "--token",
            "cli-id:cli-secret",
            "--hub-key",
            &"ab".repeat(32),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `holler body join` against the throwaway TLS listener")
        .wait_with_output()
        .expect("wait on `holler body join`");

    assert_eq!(
        out.status.code(),
        Some(1),
        "a self-signed wss:// cert must fail closed at exit 1 (join.rs's own \
         documented `Connect` error path), never exit 0 or a policy-3 refusal; \
         stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("could not reach the hub:"),
        "the refusal must be join.rs's own `Connect` error naming the TLS \
         failure, never a silent downgrade; got: {stderr}"
    );

    let _ = server.join();
}

/// Generate a throwaway self-signed cert for `127.0.0.1` and start a
/// TLS-terminating listener on a free loopback port. Returns the bound port
/// and the background thread driving the accept loop (the caller `.join()`s
/// it after the client attempt has run its course).
///
/// The server side deliberately does not need to *complete* the handshake:
/// this test's whole point is that the client aborts once it sees the
/// self-signed cert, so `acceptor.accept(..)` returning an `Err` here is the
/// expected, successful outcome, not a harness failure.
fn start_self_signed_tls_listener() -> (u16, std::thread::JoinHandle<()>) {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()])
            .expect("generate a throwaway self-signed cert for 127.0.0.1");
    let cert_der = cert.der().clone();
    let key_der = rustls::pki_types::PrivateKeyDer::try_from(key_pair.serialize_der())
        .expect("the rcgen key pair serializes as a valid PKCS#8 DER key");

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key_der)
        .expect("build a rustls ServerConfig from the self-signed cert");
    let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(server_config));

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a throwaway TLS listener");
    std_listener
        .set_nonblocking(true)
        .expect("set the throwaway listener nonblocking for tokio");
    let port = std_listener
        .local_addr()
        .expect("read the throwaway listener's bound port")
        .port();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build a tokio runtime for the throwaway TLS server");
        rt.block_on(async move {
            let listener =
                tokio::net::TcpListener::from_std(std_listener).expect("adopt the std listener into tokio");
            if let Ok((stream, _)) = listener.accept().await {
                if let Err(e) = acceptor.accept(stream).await {
                    eprintln!(
                        "throwaway TLS server: handshake did not complete \
                         (expected — the client must reject the self-signed cert): {e}"
                    );
                }
            }
        });
    });

    (port, handle)
}

// ---------------------------------------------------------------------------
// hlr-1403 — state dir paths, lock files, and 0600 permissions
// ---------------------------------------------------------------------------

/// The state-dir file set hlr-1403 pins, one `rstest` case per file.
#[derive(Clone, Copy, Debug)]
enum PlatformFile {
    /// `hub/tokens.json` (`holler_hub::token::tokens_path`).
    HubTokenStore,
    /// `hub/.pepper` (`holler_hub::token::pepper_path`), mode 0600.
    HubPepper,
    /// `body/credential.json` (`holler_body::identity::BodyIdentity::path`), mode 0600.
    BodyCredential,
    /// `hub/serve.lock` (the hub's instance lock).
    HubInstanceLock,
}

/// State dir paths, lock files, and 0600 permissions behave identically on
/// Linux and macOS. Parameterized (`rstest`) over the file set that actually
/// carries a permission contract in this codebase: the pepper file and the
/// body credential are both explicitly chmod'd 0600 on write
/// (`holler_hub::token`'s `resolve_pepper_uncached`,
/// `holler_body::identity::save`'s `set_mode_600`) so those two cases assert
/// the mode; the token store and the instance lock have no such contract
/// (`Store::save` and the lock file are written with the process's ambient
/// umask) so those two cases pin path/existence/lifecycle instead of a mode
/// this codebase does not actually guarantee. Every check here uses only
/// `std`/Unix APIs, so a single run behaves the same whichever of the two
/// CI OSes (`ubuntu-latest`/`macos-latest`) it runs on.
#[rstest]
#[case::hub_token_store(PlatformFile::HubTokenStore)]
#[case::hub_pepper(PlatformFile::HubPepper)]
#[case::body_credential(PlatformFile::BodyCredential)]
#[case::hub_instance_lock(PlatformFile::HubInstanceLock)]
fn state_dir_layout_and_perms(#[case] file: PlatformFile) {
    match file {
        PlatformFile::HubTokenStore => check_hub_token_store(),
        PlatformFile::HubPepper => check_hub_pepper(),
        PlatformFile::BodyCredential => check_body_credential(),
        PlatformFile::HubInstanceLock => check_hub_instance_lock(),
    }
}

fn check_hub_token_store() {
    let state = StateDir::new();
    support::mint_token(&state, "kiwi"); // no live hub needed — an offline store write.
    let path = state.hub().join("tokens.json");
    assert!(
        path.is_file(),
        "the hub token store must exist at hub/tokens.json; got {path:?}"
    );
}

fn check_hub_pepper() {
    let state = StateDir::new();
    support::mint_token(&state, "kiwi"); // mint autogenerates the pepper on first use.
    let path = state.hub().join(".pepper");
    assert!(
        path.is_file(),
        "the hub pepper file must exist at hub/.pepper; got {path:?}"
    );
    assert_eq!(
        mode_of(&path),
        0o600,
        "hub/.pepper must be 0600 (owner read/write only) on both Linux and macOS"
    );
}

fn check_body_credential() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret) = support::mint_token(&state, "kiwi");
    support::join(&state, &state, &hub.ws_url(), &token_id, &secret);

    let path = state.body().join("credential.json");
    assert!(
        path.is_file(),
        "the body credential file must exist at body/credential.json; got {path:?}"
    );
    assert_eq!(
        mode_of(&path),
        0o600,
        "body/credential.json must be 0600 (owner read/write only) on both Linux and macOS"
    );
    hub.stop(Duration::from_secs(5));
}

fn check_hub_instance_lock() {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let path = state.hub().join("serve.lock");
    assert!(
        path.is_file(),
        "the hub instance lock must exist at hub/serve.lock while serving; got {path:?}"
    );
    hub.stop(Duration::from_secs(5));
    assert!(
        !path.exists(),
        "the hub instance lock must be removed after a clean shutdown"
    );
}

/// The Unix permission bits of `path` (masked to the owner/group/other rwx
/// bits, ignoring the file-type bits `MetadataExt::mode` also carries).
fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path).expect("stat the file").permissions().mode() & 0o777
}

// ---------------------------------------------------------------------------
// hlr-1404 — an unavailable control socket reports a clear message
// ---------------------------------------------------------------------------

/// When the control socket is unavailable, a one-shot command (`hub status`)
/// reports a clear message rather than hanging or surfacing a raw I/O error.
///
/// Exercised via `HOLLER_TEST_NO_CONTROL_SOCKET=1` (issue #315 added this
/// env-gated bypass to `holler_hub::control::exchange_with_timeout`,
/// mirroring the existing `HOLLER_TEST_HOOKS=1` test-only convention issue
/// #192's `control/test_drop` and #184's `HOLLER_TEST_TOKEN_STORE_DELAY_MS`
/// use) against a **live** hub — proving the client's own "unavailable"
/// path, not just the already-covered "no hub was ever started" case
/// (`hub_serve_test.rs`'s `status_without_live_hub_exit_1`).
#[test]
fn no_control_socket_message() {
    let state = StateDir::new();
    let hub = Hub::start(&state); // genuinely live — the point is the CLIENT still reports "unavailable".

    let out = holler_cmd(&state)
        .env("HOLLER_TEST_NO_CONTROL_SOCKET", "1")
        .args(["hub", "status"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `hub status` with the control socket simulated unavailable")
        .wait_with_output()
        .expect("wait on `hub status`");

    assert_eq!(
        out.status.code(),
        Some(1),
        "an unavailable control socket must exit 1, same as no live hub; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no live holler hub reachable at"),
        "must report the spec's clear message even though a hub is actually \
         live (proving the env override, not a real absence, drove this); \
         got: {stderr}"
    );
    assert!(
        stderr.contains(state.path().to_str().unwrap()),
        "the message names the state dir; got: {stderr}"
    );

    hub.stop(Duration::from_secs(5));
}
