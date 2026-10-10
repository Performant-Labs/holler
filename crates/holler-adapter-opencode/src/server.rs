//! Starting a pane's OpenCode server (`serve`) and asking a server whether it is healthy
//! (`health`, and `serve`'s own checks).
//!
//! `serve` alone owns the `Child` it spawns until it returns. Its boot poll checks
//! `try_wait` after each health try, so a child that exits first is reaped there. On the
//! deadline it kills the child's process group and then waits on the child itself. Only on
//! success does the `Child` move to a detached thread, which is then its one owner and waits
//! on it once. So every child is reaped exactly once, and the server is never killed by the
//! adapter once it is up: it outlives the CLI that started it.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use holler_pane::{PaneError, PaneName};
use serde_json::Value;

use crate::http::{self, HttpError, Reply};
use crate::{budget, deadline_after, excerpt, exec, json_of, one_line};
use crate::{OpenCodeConfig, ProcessEnv, OP_SERVE};

/// The route a health check asks.
const HEALTH: &str = "/global/health";

/// The least time from the start of one of `serve`'s boot health tries to the start of the
/// next (the brief: every 100-200 ms). A try that takes longer, up to `boot_try` when its GET
/// hangs in the boot race, is followed at once.
const BOOT_INTERVAL: Duration = Duration::from_millis(150);

/// How long the `kill` of a server that never came up may take.
const KILL_BOUND: Duration = Duration::from_secs(1);

/// `HarnessPort::serve`: start the server for `name` on `port` and return its pid, which is
/// also its process group id.
pub(crate) fn serve(config: &OpenCodeConfig, name: &PaneName, port: u16) -> Result<u32, PaneError> {
    let deadline = deadline_after(config.timeouts.call);
    refuse_a_held_port(port, budget(config.timeouts.health, deadline))?;
    let dir = (config.workdir)(name)?;
    let mut child = start(config, port, &dir)?;
    wait_until_up(&mut child, name, port, deadline, config.timeouts.boot_try)?;
    let pid = child.id();
    reap_when_it_exits(child);
    Ok(pid)
}

/// Whether the server on `port` answers `GET /global/health` with `{"healthy": true}`
/// within `within`. Refused, timed out, garbled or anything else is `false`.
pub(crate) fn healthy(port: u16, within: Duration) -> bool {
    matches!(
        http::request(port, "GET", HEALTH, None, within),
        Ok(reply) if is_healthy(&reply)
    )
}

/// A `200` whose JSON is an object with `"healthy": true`.
fn is_healthy(reply: &Reply) -> bool {
    json_of(reply)
        .as_ref()
        .and_then(|health| health.get("healthy"))
        .and_then(Value::as_bool)
        == Some(true)
}

/// `serve`'s first step, one health GET. Only a refusal (nothing listens) lets `serve` go
/// on. A server that answers holds the port, and `serve` never adopts one (decision 2); a
/// frozen one is `timeout`.
fn refuse_a_held_port(port: u16, within: Duration) -> Result<(), PaneError> {
    let what = match http::request(port, "GET", HEALTH, None, within) {
        Err(HttpError::Refused) => return Ok(()),
        Err(HttpError::TimedOut) => return Err(timeout()),
        Ok(reply) if is_healthy(&reply) => {
            format!("port {port} is in use: a harness server already answers there")
        }
        Ok(reply) => format!(
            "port {port} is in use: GET {HEALTH} answered {}, not a healthy harness server: \"{}\"",
            reply.status,
            excerpt(&reply.body)
        ),
        Err(HttpError::Garbled(why)) => {
            format!("cannot serve port {port}: GET {HEALTH}: {}", one_line(&why))
        }
    };
    Err(PaneError::Unavailable { what })
}

/// Spawn `<opencode_bin> serve --port P --hostname 127.0.0.1 <serve_args>` in `dir`, in a
/// process group of its own (so its pgid is its pid), with stdin, stdout and stderr null
/// and the environment `config.env` gives.
fn start(config: &OpenCodeConfig, port: u16, dir: &Path) -> Result<Child, PaneError> {
    let port_arg = port.to_string();
    let mut command = Command::new(&config.opencode_bin);
    command
        .args([
            "serve",
            "--port",
            port_arg.as_str(),
            "--hostname",
            "127.0.0.1",
        ])
        .args(&config.serve_args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    if let ProcessEnv::Isolated(vars) = &config.env {
        command.env_clear().envs(vars.iter().map(|(k, v)| (k, v)));
    }
    command.spawn().map_err(|error| PaneError::Unavailable {
        what: one_line(&format!(
            "cannot start the harness server {} in {}: {error}",
            config.opencode_bin.display(),
            dir.display()
        )),
    })
}

/// Ask the fresh server whether it is healthy, one `GET /global/health` of at most
/// `boot_try` per try, the tries starting at least [`BOOT_INTERVAL`] apart, until it is or
/// `deadline` passes. Nothing else is sent before the first healthy answer: a request sent
/// while the server boots can be accepted and never answered
/// (opencode-pane-spike.md:198-201).
fn wait_until_up(
    child: &mut Child,
    name: &PaneName,
    port: u16,
    deadline: Instant,
    boot_try: Duration,
) -> Result<(), PaneError> {
    loop {
        let tried = Instant::now();
        let up = healthy(port, budget(boot_try, deadline));
        // A healthy answer counts only while the child still runs: it must be this server's.
        match child.try_wait() {
            Ok(None) if up => return Ok(()),
            Ok(None) => {}
            Ok(Some(status)) => return Err(exited(name, port, status)),
            Err(error) => {
                stop(child);
                return Err(PaneError::Unavailable {
                    what: one_line(&format!(
                        "cannot watch the harness server for {name} on port {port}: {error}"
                    )),
                });
            }
        }
        let next = tried + BOOT_INTERVAL;
        if next >= deadline {
            stop(child);
            return Err(timeout());
        }
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
}

/// What `serve` answers when the server exits before it is healthy (`try_wait` has reaped
/// it).
fn exited(name: &PaneName, port: u16, status: ExitStatus) -> PaneError {
    PaneError::Unavailable {
        what: format!(
            "the harness server for {name} on port {port} exited before it answered ({status})"
        ),
    }
}

/// Kill what `serve` started and reap it: SIGKILL its process group (the server's own
/// children too), then the child itself in case the group kill failed, then wait on it.
fn stop(child: &mut Child) {
    let _ = exec::kill_group(child.id(), OP_SERVE, KILL_BOUND);
    let _ = child.kill();
    let _ = child.wait();
}

/// Hand a server that is up to a detached thread that waits on it once, so a long-lived
/// caller keeps no zombie. Nothing kills the server here.
fn reap_when_it_exits(mut child: Child) {
    let _ = std::thread::Builder::new()
        .name("opencode-serve-reaper".to_owned())
        .spawn(move || {
            let _ = child.wait();
        });
}

fn timeout() -> PaneError {
    PaneError::Timeout {
        op: OP_SERVE.to_owned(),
    }
}
