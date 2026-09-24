//! The org-convention dev scripts: `scripts/run app:hub:*` / `app:body:*` (issue #425).
//!
//! Drives the real `scripts/run` dispatcher against the real `holler` binary. Every test:
//! - uses its own temp [`StateDir`] (never the developer's `~/.holler`),
//! - points the scripts at the just-built test binary via `HOLLER_BIN` (which also skips the
//!   release build the scripts would otherwise do first),
//! - listens on an OS-picked port (`HOLLER_HUB_LISTEN=127.0.0.1:0`), so parallel tests and a
//!   developer's running hub never collide.
//!
//! Unix-only: the scripts are bash and the hub's control socket is a Unix socket.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)] // #425

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

mod support;
use support::{holler_bin, wait_for, Hub, StateDir, STARTUP_WAIT};

/// Every name `scripts/run` must list in its usage line.
const NAMES: &[&str] = &[
    "build",
    "app:hub:run",
    "app:hub:launch",
    "app:hub:stop",
    "app:hub:status",
    "app:body:run",
    "app:body:stop",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `scripts/run <name>` with the isolated environment described in the module docs.
fn run_script(state: &StateDir, args: &[&str]) -> Output {
    Command::new(repo_root().join("scripts/run"))
        .args(args)
        .current_dir(repo_root())
        .env("HOLLER_STATE_DIR", state.path())
        .env("HOLLER_BIN", holler_bin())
        .env("HOLLER_HUB_LISTEN", "127.0.0.1:0")
        .env_remove("HOLLER_HUB_ADVERTISE")
        .env_remove("HOLLER_CONFIG")
        .stdin(Stdio::null())
        .output()
        .expect("spawn scripts/run")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn lock_pid(state: &StateDir) -> Option<i32> {
    std::fs::read_to_string(state.hub().join("serve.lock"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn alive(pid: i32) -> bool {
    // SAFETY: signal 0 only probes for existence.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Safety net: if an assertion panics between launch and stop, do not leak the backgrounded hub.
struct ReapHub<'a>(&'a StateDir);
impl Drop for ReapHub<'_> {
    fn drop(&mut self) {
        if let Some(pid) = lock_pid(self.0) {
            // SAFETY: the pid came from this test's own state dir's lock, i.e. the hub this test launched.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

#[test]
fn run_usage_lists_all_names_exit_1() {
    let state = StateDir::new();
    for args in [&[][..], &["no:such:command"][..]] {
        let out = run_script(&state, args);
        assert_eq!(out.status.code(), Some(1), "args {args:?}: {out:?}");
        let err = text(&out.stderr);
        assert!(
            err.contains("usage: ./scripts/run"),
            "usage line missing: {err}"
        );
        for name in NAMES {
            assert!(err.contains(name), "usage must list `{name}`: {err}");
        }
    }
}

#[test]
fn hub_launch_then_status_then_stop_round_trip() {
    let state = StateDir::new();
    let _reap = ReapHub(&state);

    let launch = run_script(&state, &["app:hub:launch"]);
    assert!(launch.status.success(), "launch failed: {launch:?}");
    let out = text(&launch.stdout);
    assert!(
        out.contains("hub token mint"),
        "next steps must name mint: {out}"
    );
    assert!(
        out.contains("holler roster"),
        "next steps must name roster: {out}"
    );

    let pid = lock_pid(&state).expect("hub wrote its pid to hub/serve.lock");
    assert!(alive(pid), "the launched hub must be running");
    assert!(
        state.hub().join("serve.log").exists(),
        "hub output goes to hub/serve.log"
    );

    let status = run_script(&state, &["app:hub:status"]);
    assert!(status.status.success(), "status failed: {status:?}");
    let out = text(&status.stdout);
    assert!(out.contains("hub 0."), "hub status output missing: {out}");
    assert!(
        out.contains("no sessions on the hub"),
        "roster output missing: {out}"
    );

    let stop = run_script(&state, &["app:hub:stop"]);
    assert!(stop.status.success(), "stop failed: {stop:?}");
    assert!(
        wait_for(Duration::from_secs(10), || (!alive(pid)).then_some(())).is_some(),
        "hub pid {pid} must be gone after stop"
    );

    let after = run_script(&state, &["app:hub:status"]);
    assert!(
        !after.status.success(),
        "status against a stopped hub must fail: {after:?}"
    );
}

#[test]
fn hub_launch_refuses_when_lock_held() {
    let state = StateDir::new();
    let hub = Hub::start(&state); // a real hub holding this state dir's serve.lock
    let out = run_script(&state, &["app:hub:launch"]);
    assert!(!out.status.success(), "launch must refuse: {out:?}");
    assert!(
        text(&out.stderr).contains("already running"),
        "stderr: {}",
        text(&out.stderr)
    );
    assert!(alive(hub.pid() as i32), "the first hub must be untouched");
    hub.stop(STARTUP_WAIT);
}

#[test]
fn hub_stop_refuses_a_pid_that_is_not_a_holler_hub() {
    let state = StateDir::new();
    let mut bystander = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("spawn sleep");
    std::fs::create_dir_all(state.hub()).unwrap();
    std::fs::write(
        state.hub().join("serve.lock"),
        format!("{}\n", bystander.id()),
    )
    .unwrap();

    let out = run_script(&state, &["app:hub:stop"]);
    let survived = bystander.try_wait().unwrap().is_none();
    let _ = bystander.kill();
    let _ = bystander.wait();

    assert!(!out.status.success(), "stop must refuse: {out:?}");
    assert!(
        text(&out.stderr).contains("not a holler hub"),
        "stderr: {}",
        text(&out.stderr)
    );
    assert!(
        survived,
        "the unrelated process must not have been signalled"
    );
}

#[test]
fn app_body_run_without_join_names_the_join_command() {
    let state = StateDir::new();
    let cfg = state.path().join("sessions.toml");
    std::fs::write(&cfg, "[[session]]\nname = \"hello\"\n").unwrap();
    let out = Command::new(repo_root().join("scripts/run"))
        .arg("app:body:run")
        .current_dir(repo_root())
        .env("HOLLER_STATE_DIR", state.path())
        .env("HOLLER_BIN", holler_bin())
        .env("HOLLER_CONFIG", &cfg)
        .output()
        .expect("spawn scripts/run");
    assert!(
        !out.status.success(),
        "an unjoined body must not run: {out:?}"
    );
    assert!(
        text(&out.stderr).contains("holler body join"),
        "stderr: {}",
        text(&out.stderr)
    );
}
