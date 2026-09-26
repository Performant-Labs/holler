//! Selftests for the shared test harness (story #138).
//!
//! These prove the *harness itself* behaves correctly, independent of the hub/
//! body (which are not implemented yet). They cover the three failure classes a
//! broken harness would be blind to:
//!
//! 1. a `StateDir` is actually removed on drop (no leaked temp dirs),
//! 2. `wait_for` returns `None` cleanly and fast on timeout (no hang),
//! 3. `kill_tree` reaps a child *and its grandchildren* (no orphaned agents).
//!
//! `Hub` and `Body` are both exercised here: `hub_dropped_without_stop_reaps_its_tree`
//! pins that a `Hub` dropped without `stop` still reaps its process tree (no
//! orphaned hub), and `body_dropped_without_stop_reaps_its_tree` pins the same
//! for `Body` (no orphaned `holler body run` / `stub-acp`).
//!
//! Issue #420 adds the warm-up diagnostics: the body's retained log
//! (`Body::log_path`/`log_text`), the pure roster/`say` classifiers, the
//! fallible `try_roster_json`, and `wait_warm`'s retry-only-`unknown session`
//! contract and its three-section (`roster:` / `hub log:` / `body log:`) panic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #138

mod support;

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use support::{kill_tree, make_own_process_group, wait_for, StateDir};

/// `StateDir::new` creates a real directory, and dropping it removes it — so a
/// panicking test does not leak state dirs into the temp area.
#[test]
fn state_dir_is_removed_on_drop() {
    let path = {
        let dir = StateDir::new();
        assert!(dir.path().is_dir(), "fresh StateDir is a real directory");
        // Sanity: the subpaths the harness exposes are derived from it.
        assert!(dir.hub().starts_with(dir.path()));
        assert!(dir.body().starts_with(dir.path()));
        dir.path().to_path_buf()
    }; // `dir` drops here.

    // Give the drop a beat to settle, then assert the dir is gone.
    wait_for(Duration::from_secs(2), || (!path.exists()).then_some(()))
        .unwrap_or_else(|| panic!("StateDir was not removed on drop: {path:?}"));
}

/// `wait_for` must stop at the deadline and return `None` promptly. The spec's
/// bound is "well under `timeout + 100 ms`"; `wait_for` sleeps only the
/// *remaining* budget (capped at the poll interval), so the sleep can never
/// push the return past the deadline — only the final `check()` execution can
/// add latency. The hard bound below is the spec's 100 ms of slack plus one
/// poll interval (50 ms) to absorb OS wake-jitter on a loaded CI runner
/// (`thread::sleep` is a minimum: the OS may wake us late). A genuinely hung
/// check would run for *seconds*, so this bound still cleanly separates
/// "timed out" from "hung".
#[test]
fn wait_for_times_out_cleanly() {
    let timeout = Duration::from_millis(150);
    let started = Instant::now();
    let result: Option<i32> = wait_for(timeout, || None);
    let elapsed = started.elapsed();

    assert!(
        result.is_none(),
        "a never-satisfying check must time out to None"
    );
    assert!(
        elapsed <= timeout + Duration::from_millis(150),
        "wait_for overshot its budget: took {elapsed:?} for a {timeout:?} timeout"
    );
    assert!(
        elapsed.as_millis() >= 50,
        "wait_for returned before its first poll tick (took {elapsed:?})"
    );
}

/// And the positive half: `wait_for` *does* return a value once the check
/// starts succeeding (it does not only ever time out).
#[test]
fn wait_for_returns_when_check_succeeds() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let started = Instant::now();
    let got = wait_for(Duration::from_secs(2), || {
        // Succeed on the 3rd poll.
        if CALLS.fetch_add(1, Ordering::SeqCst) >= 2 {
            Some(42)
        } else {
            None
        }
    });
    assert_eq!(got, Some(42), "wait_for must surface the first Some");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "wait_for returned only by timeout, not by success"
    );
}

/// `kill_tree` must kill the child *and its descendants*. We spawn a shell that
/// itself spawns a long-lived `sleep` (a grandchild) and waits on it; the shell
/// is in its own process group (as every harness spawn is). After `kill_tree`
/// on the shell, no `sleep` from that group may remain.
///
/// Unix-only (the spec says skip on Windows — `taskkill /F /T` is verified by
/// the Windows CI matrix separately, and `ps` is not portable).
#[cfg(unix)]
#[test]
fn kill_tree_kills_grandchildren() {
    // `sh -c 'sleep 30 & wait'`: the shell forks a `sleep 30` grandchild and
    // waits, so the shell stays alive as long as the sleep does. Both are in
    // the shell's process group because we spawn it with its own pgid (the same
    // setup every harness spawn uses).
    let mut shell_cmd = Command::new("sh");
    make_own_process_group(&mut shell_cmd);
    let mut shell = shell_cmd
        .args(["-c", "sleep 30 & wait"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn `sh -c 'sleep 30 & wait'`");

    // Let the grandchild actually start (observed, not guessed): poll `ps`
    // until a `sleep` in the shell's process group is visible.
    let pgid = shell.id() as i64;
    let started = Instant::now();
    wait_for(Duration::from_secs(3), || {
        (grandchildren_of(pgid) > 0).then_some(())
    })
    .unwrap_or_else(|| {
        kill_tree(&mut shell);
        panic!(
            "expected a `sleep` grandchild in pgid {pgid} within 3s; \
             the spawn pattern may not have produced one (took {:?})",
            started.elapsed()
        )
    });

    // Now kill the tree and confirm the grandchild is reaped.
    kill_tree(&mut shell);
    let reaped = wait_for(Duration::from_secs(3), || {
        (grandchildren_of(pgid) == 0).then_some(())
    });
    assert!(
        reaped.is_some(),
        "a grandchild of pgid {pgid} survived kill_tree (orphans leak agents)"
    );
}

/// A `Hub` dropped *without* calling [`Hub::stop`] must still reap its process
/// tree — otherwise the hub is orphaned (reparented to init) and leaks. This is
/// exactly what the WS first-frame tests in `hub_serve_test` do (they `drop(hub)`
/// at the end). Pinned here so a regression in the harness's `Drop` impl is
/// caught by the harness's own selftests, not just by a flaky "orphans left over"
/// observation.
///
/// (Story #143 implemented `hub serve`, so this selftest — which the header above
/// deferred to "the hub story" — now lands here.)
#[cfg(unix)]
#[test]
fn hub_dropped_without_stop_reaps_its_tree() {
    use support::Hub;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    // The hub is its own process-group leader (Hub::start calls
    // make_own_process_group), so pgid == pid.
    let pgid = hub.pid() as i64;
    assert!(
        hub_child_visible(pgid),
        "the hub we just started is not visible in its own process group"
    );
    // Let it drop *without* calling stop — the `Drop` impl must reap the tree.
    drop(hub);
    let reaped = wait_for(Duration::from_secs(5), || {
        (!hub_child_visible(pgid)).then_some(())
    });
    assert!(
        reaped.is_some(),
        "dropping a `Hub` without calling `stop` leaked its process (pgid {pgid} \
         still alive after 5s) — the harness `Drop` impl must reap the tree"
    );
    drop(state);
}

/// A `Body` dropped *without* calling [`support::Body::stop`] must still reap
/// its process tree — otherwise the body (and any agent it spawned, e.g.
/// `stub-acp`) is orphaned (reparented to init) and leaks. `Hub` has exactly
/// this protection (`hub_dropped_without_stop_reaps_its_tree` above); `Body`
/// did not (issue: orphaned `holler body run` / `stub-acp` processes observed
/// surviving well after `cargo test --workspace` exited — the
/// `holler-test-<pid>-<hash>-<n>` state-dir naming pins them to this harness,
/// not `holler-load-test`'s separately-verified `hlr-lt-*` cleanup). Any test
/// that panics, early-returns, or otherwise lets its `Body` fall out of scope
/// before reaching its own `body.stop(...)` call hit exactly this gap.
#[cfg(unix)]
#[test]
fn body_dropped_without_stop_reaps_its_tree() {
    use support::{write_sessions_toml, Body};
    let state = StateDir::new();
    let sessions = write_sessions_toml(&state, &[("s1", &["--chunks", "1"])]);
    let mut body = Body::start(&state, &sessions);
    // The body is its own process-group leader (Body::start calls
    // make_own_process_group), so pgid == pid.
    let pgid = body.child_mut().id() as i64;
    assert!(
        grandchildren_of(pgid) > 0,
        "the body we just started is not visible in its own process group"
    );
    // Let it drop *without* calling stop — the `Drop` impl must reap the tree.
    drop(body);
    let reaped = wait_for(Duration::from_secs(5), || {
        (grandchildren_of(pgid) == 0).then_some(())
    });
    assert!(
        reaped.is_some(),
        "dropping a `Body` without calling `stop` leaked its process (pgid {pgid} \
         still alive after 5s) — the harness `Drop` impl must reap the tree"
    );
    drop(state);
}

// --- #420: warm-up diagnostics ----------------------------------------------

/// Start a body with one `stub-acp` session `session`, joined to `hub` under
/// label `t`, in its own state dir (the same setup `interrupt_test` uses).
fn start_joined_body(
    hub_state: &StateDir,
    body_state: &StateDir,
    hub: &support::Hub,
    session: &str,
) -> support::Body {
    use support::{join, mint_token, write_sessions_toml, Body};
    let (token_id, secret) = mint_token(hub_state, "t");
    join(body_state, hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(body_state, &[(session, &["--chunks", "1"])]);
    Body::start(body_state, &config)
}

/// The panic payload of `f` as text (`String`, falling back to `&str`), or a
/// test failure if `f` returned instead of panicking.
fn panic_text_of<R>(f: impl FnOnce() -> R) -> String {
    let payload = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(_) => panic!("expected the call to panic, but it returned"),
        Err(p) => p,
    };
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        panic!("panic payload is neither String nor &str")
    }
}

/// Byte offsets of `roster:`, `hub log:` and `body log:` in `msg`, asserting
/// all three are present and in that order.
fn section_offsets(msg: &str) -> (usize, usize, usize) {
    let r = msg
        .find("roster:")
        .unwrap_or_else(|| panic!("no `roster:` section in: {msg}"));
    let h = msg
        .find("hub log:")
        .unwrap_or_else(|| panic!("no `hub log:` section in: {msg}"));
    let b = msg
        .find("body log:")
        .unwrap_or_else(|| panic!("no `body log:` section in: {msg}"));
    assert!(
        r < h && h < b,
        "sections out of order (roster:{r} hub log:{h} body log:{b}) in: {msg}"
    );
    (r, h, b)
}

/// A fake `say` result with the given exit code and stderr (Unix: the raw wait
/// status of exit code `c` is `c << 8`).
#[cfg(unix)]
fn fake_output(code: i32, stderr: &str) -> std::process::Output {
    use std::os::unix::process::ExitStatusExt;
    std::process::Output {
        status: std::process::ExitStatus::from_raw(code << 8),
        stdout: if code == 0 {
            b"reply\n".to_vec()
        } else {
            Vec::new()
        },
        stderr: stderr.as_bytes().to_vec(),
    }
}

/// A `roster --json` document with one row per `(name, conn_state)`, each row
/// carrying the full field set the real `holler roster --json` prints (shape
/// confirmed live at RED for #420).
fn roster_fixture(rows: &[(&str, &str)]) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = rows
        .iter()
        .map(|(name, conn)| {
            serde_json::json!({
                "name": name, "harness": "opencode", "mode": "spawn", "state": "idle",
                "conn_state": conn, "token_id": "tok", "client_id": "cid", "hostname": "h",
                "harness_session_id": null, "first_seen": 1, "last_seen": 1,
                "last_update_at": 1, "pending": null, "turn_id": null, "last_turn": null,
            })
        })
        .collect();
    serde_json::json!({ "rows": rows })
}

/// #420 criterion 1: the body's stdout/stderr are retained in `<state>/body.log`
/// (under the body's own state dir) with a start banner, and readable through
/// the `Body` handle.
#[test]
fn body_log_is_retained_and_readable() {
    use support::{roster_json, roster_row_connected, Hub};
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body = start_joined_body(&hub_state, &body_state, &hub, "alpha");

    assert!(
        body.log_path().starts_with(body_state.path()),
        "body log {:?} is not under the body's state dir {:?}",
        body.log_path(),
        body_state.path()
    );
    assert_eq!(
        body.log_path(),
        body_state.path().join("body.log"),
        "body log lives at <state>/body.log"
    );
    assert!(
        body.log_path().is_file(),
        "body log {:?} does not exist",
        body.log_path()
    );

    // The harness's banner, then the body's own stderr (its `logging_started` line).
    wait_for(Duration::from_secs(10), || {
        let t = body.log_text();
        (t.contains("--- body start") && t.contains("logging_started")).then_some(())
    })
    .unwrap_or_else(|| {
        panic!(
            "body log never showed the banner and `logging_started`: {:?}",
            body.log_text()
        )
    });

    // The predicate agrees with the real `roster --json` once the body is up.
    wait_for(Duration::from_secs(10), || {
        roster_row_connected(&roster_json(&hub_state), "alpha").then_some(())
    })
    .unwrap_or_else(|| {
        panic!(
            "alpha never connected per the real roster: {}",
            roster_json(&hub_state)
        )
    });
}

/// #420 criterion 2: only a `connected` row matching the session counts.
#[test]
fn roster_row_connected_rejects_reconnecting_gone_and_absent() {
    use support::roster_row_connected;
    assert!(!roster_row_connected(
        &roster_fixture(&[("b/alpha", "reconnecting")]),
        "alpha"
    ));
    assert!(!roster_row_connected(
        &roster_fixture(&[("b/alpha", "gone")]),
        "alpha"
    ));
    assert!(
        !roster_row_connected(&roster_fixture(&[]), "alpha"),
        "empty roster"
    );
    assert!(
        !roster_row_connected(&serde_json::json!({}), "alpha"),
        "no `rows` key"
    );
    assert!(
        !roster_row_connected(&roster_fixture(&[("b/beta", "connected")]), "alpha"),
        "other session"
    );
    assert!(
        !roster_row_connected(&roster_fixture(&[("b/xalpha", "connected")]), "alpha"),
        "suffix without `/`"
    );
    assert!(
        !roster_row_connected(&roster_fixture(&[("b/alpha", "connected")]), "c/alpha"),
        "other label"
    );
}

/// #420 criterion 2: a `connected` row matches by bare session or `label/session`.
#[test]
fn roster_row_connected_accepts_connected_qualified_and_bare() {
    use support::roster_row_connected;
    let roster = roster_fixture(&[("b/beta", "gone"), ("t/alpha", "connected")]);
    assert!(roster_row_connected(&roster, "alpha"), "bare session");
    assert!(roster_row_connected(&roster, "t/alpha"), "label/session");
    assert!(
        roster_row_connected(&roster_fixture(&[("alpha", "connected")]), "alpha"),
        "unlabelled row"
    );
}

/// #420 criterion 2: only `unknown session` is a retryable `say` failure.
#[test]
fn say_failure_retries_only_unknown_session() {
    use support::say_failure_is_retryable;
    assert!(say_failure_is_retryable("error: unknown session"));
    assert!(say_failure_is_retryable("unknown session: alpha"));
    assert!(!say_failure_is_retryable(
        "error: b/alpha is reconnecting (last seen 0s ago)"
    ));
    assert!(!say_failure_is_retryable(
        "error: b/alpha is not connected (gone)"
    ));
    assert!(!say_failure_is_retryable(""));
}

/// #420 criterion 3: with no hub, `try_roster_json` returns an `Err` naming the
/// command and carrying its stderr, instead of panicking like `roster_json`.
#[test]
fn try_roster_json_is_err_without_a_hub() {
    let state = StateDir::new();
    let err = support::try_roster_json(&state)
        .expect_err("no hub is running, so the roster must be an Err");
    assert!(
        err.contains("roster"),
        "error does not name the command: {err}"
    );
    assert!(
        err.contains("no live holler hub"),
        "error does not carry the CLI's stderr: {err}"
    );
}

/// #420 criterion 4: an expired deadline panics with `roster:`, `hub log:` and
/// `body log: <no body handle>`, in order, with the hub's real log inside.
#[test]
fn warmup_panic_carries_roster_hub_and_body_sections() {
    use support::{say, wait_warm, Hub};
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let msg = panic_text_of(|| {
        wait_warm(
            &hub_state,
            "alpha",
            Duration::from_secs(1),
            &hub,
            None,
            || say(&hub_state, "alpha", "x"),
        )
    });
    let (_, h, b) = section_offsets(&msg);
    assert!(
        msg[h..b].contains(r#""event":"listening""#),
        "hub section lacks the hub log: {msg}"
    );
    assert!(
        msg[b..].contains("body log: <no body handle>"),
        "body section wrong: {msg}"
    );
}

/// #420 criterion 4, with a real body: the `body log:` section carries that
/// body's retained log (its start banner).
#[test]
fn warmup_panic_with_body_carries_body_log() {
    use support::{say, wait_warm, Hub};
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body = start_joined_body(&hub_state, &body_state, &hub, "alpha");
    let msg = panic_text_of(|| {
        wait_warm(
            &hub_state,
            "never-advertised",
            Duration::from_secs(2),
            &hub,
            Some(&body),
            || say(&hub_state, "never-advertised", "x"),
        )
    });
    let (_, h, b) = section_offsets(&msg);
    assert!(
        msg[h..b].contains(r#""event":"listening""#),
        "hub section lacks the hub log: {msg}"
    );
    assert!(
        msg[b..].contains("--- body start"),
        "body section lacks the body log: {msg}"
    );
}

/// #420 criterion 3: the attempt waits for a `connected` roster row. With no
/// row for the session, even an attempt that would succeed is never made, and
/// the deadline panics instead of returning its output.
#[cfg(unix)]
#[test]
fn wait_warm_never_attempts_before_the_row_is_connected() {
    use support::{wait_warm, Hub};
    let hub_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let mut calls = 0u32;
    let msg = panic_text_of(|| {
        wait_warm(&hub_state, "alpha", Duration::from_secs(1), &hub, None, || {
            calls += 1;
            fake_output(0, "")
        })
    });
    assert_eq!(calls, 0, "attempted {calls} times before any connected row");
    section_offsets(&msg);
}

/// #420 criteria 3/5: once connected, a non-`unknown session` failure (the CI
/// flake's `reconnecting`) is fatal at once, never retried and never returned.
#[cfg(unix)]
#[test]
fn wait_warm_panics_at_once_on_non_retryable_failure() {
    use support::{wait_warm, Hub};
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body = start_joined_body(&hub_state, &body_state, &hub, "alpha");
    let mut calls = 0u32;
    let msg = panic_text_of(|| {
        wait_warm(
            &hub_state,
            "alpha",
            Duration::from_secs(10),
            &hub,
            Some(&body),
            || {
                calls += 1;
                fake_output(1, "error: b/alpha is reconnecting (last seen 0s ago)")
            },
        )
    });
    assert_eq!(
        calls, 1,
        "a non-retryable failure was retried ({calls} attempts)"
    );
    let (_, _, b) = section_offsets(&msg);
    assert!(
        msg[b..].contains("--- body start"),
        "body section lacks the body log: {msg}"
    );
}

/// #420 criterion 3: `unknown session` is retried until the attempt succeeds,
/// and the successful output is what comes back.
#[cfg(unix)]
#[test]
fn wait_warm_retries_unknown_session_then_returns_success() {
    use support::{wait_warm, Hub};
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let body = start_joined_body(&hub_state, &body_state, &hub, "alpha");
    let mut calls = 0u32;
    let out = wait_warm(
        &hub_state,
        "alpha",
        Duration::from_secs(10),
        &hub,
        Some(&body),
        || {
            calls += 1;
            if calls < 3 {
                fake_output(1, "error: unknown session")
            } else {
                fake_output(0, "")
            }
        },
    );
    assert!(out.status.success(), "wait_warm returned a failed output");
    assert_eq!(
        calls, 3,
        "expected two `unknown session` retries then success"
    );
}

// --- helpers (test-local; not part of the public harness API) ---------------

/// True if any process whose pgid is `pgid` is currently running (the hub or a
/// descendant of it). Unix-only (uses `ps`), like [`grandchildren_of`].
#[cfg(unix)]
fn hub_child_visible(pgid: i64) -> bool {
    grandchildren_of(pgid) > 0
}

/// The grandchild count of a process group, via `ps` on Unix.
#[cfg(unix)]
fn grandchildren_of(pgid: i64) -> u32 {
    // `ps -eo pgid,comm` lists every process's group + command. Count the rows
    // whose pgid is ours and whose comm is not the shell itself.
    let out = std::process::Command::new("ps")
        .args(["-eo", "pgid,comm"])
        .output()
        .expect("run ps");
    assert!(
        out.status.success(),
        "ps failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .skip(1) // header
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let row_pgid: i64 = parts.next()?.parse().ok()?;
            let comm = parts.next()?;
            (row_pgid == pgid && !matches!(comm, "sh" | "bash")).then_some(())
        })
        .count() as u32
}
