#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)] // #651
//! Acceptance: per-pane display facts and unknown-never-stale, driven against two
//! fakes — a fake hub control socket answering `pane/list` and a fake Herdr socket
//! capturing the report lines. No real Herdr session, no live hub, no fleet: temp
//! dirs and Unix sockets only.
//!
//! The seams the tests force: the hub read honours `HOLLER_STATE_DIR` (the socket
//! is `<state>/hub/control.sock`, exactly what `holler_hub::control::run` resolves),
//! and the reports go to `HERDR_SOCKET_PATH` with one JSON line in and one out.

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use herdr_holler::{Endpoints, Refreshed, Reporter};
use holler_pane::pane::{Hold, LastObserved};
use holler_pane::{GridPos, Pane, PaneId, ProfileName};
use holler_pane_testkit::fixture::sample_pane;
use serde_json::{json, Value};

/// Serialises every test that overrides process-wide env vars; the tests in one
/// binary run on parallel threads and would otherwise race them.
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV: Mutex<()> = Mutex::new(());
    ENV.lock().unwrap_or_else(|e| e.into_inner())
}

/// The env vars a socket-driving test overrides: the hub's state dir (the
/// control-socket path the hub's one-shot client resolves), Herdr's session
/// socket, and the client's no-socket test hook (forced off).
const ENV_VARS: [&str; 3] = [
    "HOLLER_STATE_DIR",
    "HERDR_SOCKET_PATH",
    "HOLLER_TEST_NO_CONTROL_SOCKET",
];

/// Sets [`ENV_VARS`] for one test and restores them when dropped.
struct EnvGuard {
    saved: [Option<String>; ENV_VARS.len()],
}

impl EnvGuard {
    fn set(state_dir: &Path, herdr_socket: &Path) -> Self {
        let values = [
            Some(state_dir.to_string_lossy().into_owned()),
            Some(herdr_socket.to_string_lossy().into_owned()),
            None,
        ];
        let mut saved = [None, None, None];
        for (i, name) in ENV_VARS.iter().enumerate() {
            saved[i] = env::var(name).ok();
            match &values[i] {
                Some(value) => env::set_var(name, value),
                None => env::remove_var(name),
            }
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (i, name) in ENV_VARS.iter().enumerate() {
            match &self.saved[i] {
                Some(value) => env::set_var(name, value),
                None => env::remove_var(name),
            }
        }
    }
}

/// A fake one-line-in/one-line-out socket server: every request it reads is
/// logged, and `answer` builds its reply line.
struct FakeSocket {
    log: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl FakeSocket {
    /// A fake Herdr session socket: answers every report request with the
    /// success shape `{"type":"ok"}`, echoing the request's id.
    fn herdr_at(path: &Path) -> Self {
        Self::serve(
            path,
            Arc::new(
                |req: &Value| json!({"id": req.get("id").cloned().unwrap_or(Value::Null), "result": {"type": "ok"}}),
            ),
        )
    }

    /// A fake hub control socket: answers every request with a v2 envelope whose
    /// result is a successful `PaneReply` carrying `data`.
    fn hub_at(path: &Path, data: Arc<Mutex<Value>>) -> Self {
        Self::serve(
            path,
            Arc::new(move |req: &Value| {
                let data = data.lock().unwrap_or_else(|e| e.into_inner()).clone();
                json!({"jsonrpc": "2.0",
                   "id": req.get("id").cloned().unwrap_or(Value::Null),
                   "result": {"ok": true, "data": data}})
            }),
        )
    }

    fn serve(path: &Path, answer: Arc<dyn Fn(&Value) -> Value + Send + Sync>) -> Self {
        let listener = Arc::new(UnixListener::bind(path).unwrap());
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let (log, stop) = (log.clone(), stop.clone());
            let answer = answer.clone();
            thread::spawn(move || serve_until(&listener, &log, &stop, &answer))
        };
        Self {
            log,
            stop,
            handle: Some(handle),
        }
    }

    /// Every request line captured so far, in order.
    fn requests(&self) -> Vec<Value> {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for FakeSocket {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Accept until `stop`, answering one exchange per connection.
fn serve_until(
    listener: &UnixListener,
    log: &Mutex<Vec<Value>>,
    stop: &AtomicBool,
    answer: &Arc<dyn Fn(&Value) -> Value + Send + Sync>,
) {
    listener.set_nonblocking(true).unwrap();
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => answer_one(&stream, log, answer),
            Err(e) if e.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(5)),
            Err(_) => return,
        }
    }
}

/// One exchange: read a request line, log it, write one reply line.
fn answer_one(
    stream: &UnixStream,
    log: &Mutex<Vec<Value>>,
    answer: &Arc<dyn Fn(&Value) -> Value + Send + Sync>,
) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut line = String::new();
    if BufReader::new(stream).read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }
    let Ok(request) = serde_json::from_str::<Value>(&line) else {
        return;
    };
    log.lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(request.clone());
    if let Ok(reply) = serde_json::to_string(&answer(&request)) {
        let mut writer = stream;
        let _ = writer.write_all(format!("{reply}\n").as_bytes());
        let _ = writer.flush();
    }
}

/// A sample fixture pane re-addressed into Herdr and a project directory.
fn fixture(name: &str, pane_id: &str, workspace: &str, cwd: &str) -> Pane {
    let mut pane = sample_pane(name).unwrap();
    pane.herdr.pane_id = PaneId::new(pane_id);
    pane.herdr.workspace = workspace.to_owned();
    pane.host.cwd = cwd.to_owned();
    pane
}

/// The rig both refresh tests use: a fake hub serving `panes` and a fake Herdr,
/// with the env pointed at both.
struct Rig {
    hub: FakeSocket,
    herdr: FakeSocket,
    state_dir: PathBuf,
    herdr_socket: PathBuf,
    _guard: EnvGuard,
}

fn rig(panes: &[Pane]) -> (tempfile::TempDir, Rig) {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path().join("state");
    fs::create_dir_all(state_dir.join("hub")).unwrap();
    let data = Arc::new(Mutex::new(serde_json::to_value(panes).unwrap()));
    let hub = FakeSocket::hub_at(&state_dir.join("hub").join("control.sock"), data);
    let herdr_socket = tmp.path().join("herdr.sock");
    let herdr = FakeSocket::herdr_at(&herdr_socket);
    let _guard = EnvGuard::set(&state_dir, &herdr_socket);
    (
        tmp,
        Rig {
            hub,
            herdr,
            state_dir,
            herdr_socket,
            _guard,
        },
    )
}

/// A reporter wired to the rig's two sockets.
fn reporter_of(rig: &Rig) -> Reporter {
    Reporter::new(Endpoints {
        herdr_socket: rig.herdr_socket.clone(),
        hub_control_socket: rig.state_dir.join("hub").join("control.sock"),
    })
}

/// The sorted token names of a report's params.
fn token_names(params: &Value) -> Vec<&str> {
    params
        .get("tokens")
        .and_then(Value::as_object)
        .map(|tokens| tokens.keys().map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default()
}

/// Assert the SCHEMA limits on a report's tokens: at least one, at most 16,
/// names matching `^[A-Za-z0-9_-]{1,32}$`.
fn assert_valid_tokens(params: &Value) {
    let names = token_names(params);
    assert!(!names.is_empty(), "a report with no tokens shows nothing");
    assert!(
        names.len() <= 16,
        "SCHEMA keeps at most 16 tokens per report"
    );
    for name in names {
        let plain = (1..=32).contains(&name.chars().count())
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
        assert!(plain, "token name `{name}` disobeys the SCHEMA grammar");
    }
}

/// The one pane report addressed to `pane_id`, as a line of `lines`.
fn pane_line<'a>(lines: &'a [Value], pane_id: &str) -> &'a Value {
    lines
        .iter()
        .find(|line| {
            line.get("method").and_then(Value::as_str) == Some("pane.report_metadata")
                && line
                    .get("params")
                    .and_then(|p| p.get("pane_id"))
                    .and_then(Value::as_str)
                    == Some(pane_id)
        })
        .unwrap_or_else(|| panic!("no pane report addressed to {pane_id}"))
}

/// The one workspace report addressed to `workspace_id`, as a line of `lines`.
fn workspace_line<'a>(lines: &'a [Value], workspace_id: &str) -> &'a Value {
    lines
        .iter()
        .find(|line| {
            line.get("method").and_then(Value::as_str) == Some("workspace.report_metadata")
                && line
                    .get("params")
                    .and_then(|p| p.get("workspace_id"))
                    .and_then(Value::as_str)
                    == Some(workspace_id)
        })
        .unwrap_or_else(|| panic!("no workspace report addressed to {workspace_id}"))
}

/// Assert a pane report's shape: the method, the addressing pane id, the one
/// source id, a live `ttl_ms`, and valid tokens. Returns its params.
fn assert_pane_report<'a>(line: &'a Value, pane_id: &str) -> &'a Value {
    assert_eq!(
        line.get("method").and_then(Value::as_str),
        Some("pane.report_metadata")
    );
    let params = line.get("params").unwrap();
    assert_eq!(params.get("pane_id").and_then(Value::as_str), Some(pane_id));
    assert_eq!(params.get("source").and_then(Value::as_str), Some("holler"));
    assert!(
        params
            .get("ttl_ms")
            .and_then(Value::as_u64)
            .is_some_and(|t| t > 0),
        "the pane report for {pane_id} carries no ttl_ms; stale data must expire"
    );
    assert_valid_tokens(params);
    params
}

/// Assert a workspace report's shape: the method, the addressing workspace id,
/// the one source id, **no** `ttl_ms` (spike section 4) and valid tokens.
fn assert_workspace_report<'a>(line: &'a Value, workspace_id: &str) -> &'a Value {
    assert_eq!(
        line.get("method").and_then(Value::as_str),
        Some("workspace.report_metadata")
    );
    let params = line.get("params").unwrap();
    assert_eq!(
        params.get("workspace_id").and_then(Value::as_str),
        Some(workspace_id)
    );
    assert_eq!(params.get("source").and_then(Value::as_str), Some("holler"));
    assert!(
        params.get("ttl_ms").is_none(),
        "the workspace report for {workspace_id} carries a ttl_ms; TTL is pane-only"
    );
    assert_valid_tokens(params);
    params
}

/// The three registry panes of the facts test: A parked and in sync, B healthy
/// but showing another session, C drained and never observed.
fn three_panes() -> Vec<Pane> {
    let mut a = fixture("demo-alpha", "w1:p1", "w1", "/srv/proj-a");
    a.herdr.grid = GridPos { row: 2, col: 1 };
    a.hold = Hold::Parked {
        reason: "spike".to_owned(),
        release_when: "manual".to_owned(),
        since: 1,
    };
    a.session_of_record = Some("ses-a".to_owned());
    a.last_observed = LastObserved {
        shown: Some("ses-a".to_owned()),
        driven: Some("ses-a".to_owned()),
        at: 1_000,
    };
    a.profile = Some(ProfileName::parse("demo").unwrap());

    let mut b = fixture("demo-beta", "w1:p2", "w1", "/srv/proj-a");
    b.herdr.grid = GridPos { row: 1, col: 2 };
    b.session_of_record = Some("ses-b".to_owned());
    b.last_observed = LastObserved {
        shown: Some("ses-other".to_owned()),
        driven: Some("ses-b".to_owned()),
        at: 1_000,
    };
    b.profile = Some(ProfileName::parse("demo").unwrap());

    let mut c = fixture("demo-gamma", "w2:p1", "w2", "/srv/proj-c");
    c.hold = Hold::Drained;
    c.profile = Some(ProfileName::parse("wave").unwrap());

    vec![a, b, c]
}

#[test]
fn per_pane_display_facts() {
    let _lock = env_lock();
    let panes = three_panes();
    let (tmp, rig) = rig(&panes);
    let mut reporter = reporter_of(&rig);

    let refreshed = reporter.refresh().unwrap();
    assert_eq!(
        refreshed,
        Refreshed {
            pane_reports: 3,
            workspace_reports: 2,
            unknown: false
        }
    );

    // The hub side: the plugin reads the registry and nothing else.
    let hub_requests = rig.hub.requests();
    assert!(
        !hub_requests.is_empty(),
        "the plugin never read the registry"
    );
    for request in &hub_requests {
        assert_eq!(
            request.get("method").and_then(Value::as_str),
            Some("pane/list"),
            "the hub wire saw a method that is not the read-only registry list"
        );
    }

    // The Herdr side: three pane reports and two workspace reports, no other call.
    let lines = rig.herdr.requests();
    let mut methods: Vec<&str> = lines
        .iter()
        .filter_map(|line| line.get("method").and_then(Value::as_str))
        .collect();
    methods.sort_unstable();
    assert_eq!(
        methods,
        [
            "pane.report_metadata",
            "pane.report_metadata",
            "pane.report_metadata",
            "workspace.report_metadata",
            "workspace.report_metadata"
        ]
    );

    // pos is the GridPos display, project is host.cwd, shown/driven are the last
    // observation (`-` when absent), sync is the one rule's ok/mismatch/-, hold
    // is none/parked/drained.
    let expected: [(&str, [&str; 6]); 3] = [
        (
            "w1:p1",
            ["r2c1", "/srv/proj-a", "ses-a", "ses-a", "ok", "parked"],
        ),
        (
            "w1:p2",
            [
                "r1c2",
                "/srv/proj-a",
                "ses-other",
                "ses-b",
                "mismatch",
                "none",
            ],
        ),
        ("w2:p1", ["r1c1", "/srv/proj-c", "-", "-", "-", "drained"]),
    ];
    for (pane_id, tokens) in expected {
        let [pos, project, shown, driven, sync, hold] = tokens;
        let params = assert_pane_report(pane_line(&lines, pane_id), pane_id);
        for (name, want) in [
            ("pos", pos),
            ("project", project),
            ("shown", shown),
            ("driven", driven),
            ("sync", sync),
            ("hold", hold),
        ] {
            let got = params
                .get("tokens")
                .and_then(|tokens| tokens.get(name))
                .and_then(Value::as_str);
            assert_eq!(got, Some(want), "token {name} of the report for {pane_id}");
        }
        let mut names = token_names(params);
        names.sort_unstable();
        assert_eq!(
            names,
            ["driven", "hold", "pos", "project", "shown", "sync"],
            "the report for {pane_id} is not the six-fact vocabulary"
        );
    }

    // The profile name is a workspace token where the workspace maps.
    for (workspace, profile) in [("w1", "demo"), ("w2", "wave")] {
        let params = assert_workspace_report(workspace_line(&lines, workspace), workspace);
        let got = params
            .get("tokens")
            .and_then(|tokens| tokens.get("profile"))
            .and_then(Value::as_str);
        assert_eq!(
            got,
            Some(profile),
            "the profile token of workspace {workspace}"
        );
    }
    drop(reporter);
    drop(rig);
    drop(tmp);
}

#[test]
fn unknown_never_stale() {
    let _lock = env_lock();
    let mut a = fixture("demo-alpha", "w1:p1", "w1", "/srv/proj-a");
    a.session_of_record = Some("ses-a".to_owned());
    a.last_observed = LastObserved {
        shown: Some("ses-a".to_owned()),
        driven: Some("ses-a".to_owned()),
        at: 1_000,
    };
    a.profile = Some(ProfileName::parse("demo").unwrap());
    let (tmp, rig) = rig(&[a]);
    let mut reporter = reporter_of(&rig);

    // One successful refresh: two reports, live values.
    let first = reporter.refresh().unwrap();
    assert_eq!(
        first,
        Refreshed {
            pane_reports: 1,
            workspace_reports: 1,
            unknown: false
        }
    );
    let live = rig.herdr.requests().len();

    // The hub goes away: its socket is gone, so the read fails.
    fs::remove_file(rig.state_dir.join("hub").join("control.sock")).unwrap();

    // The next refresh reports `unknown` for the pane it had been showing, under
    // the same source — never the first set repeated.
    let second = reporter.refresh().unwrap();
    assert_eq!(
        second,
        Refreshed {
            pane_reports: 1,
            workspace_reports: 0,
            unknown: true
        }
    );
    let degraded = &rig.herdr.requests()[live..];
    assert_eq!(
        degraded.len(),
        1,
        "the degraded refresh sent more than the unknown report"
    );
    let params = assert_pane_report(&degraded[0], "w1:p1");
    let labels = params
        .get("state_labels")
        .and_then(Value::as_array)
        .map(|l| l.len());
    assert!(
        params
            .get("state_labels")
            .and_then(Value::as_array)
            .is_some_and(|labels| labels.iter().any(|label| label == "unknown")),
        "the degraded report carries no `unknown` state label (labels: {labels:?})"
    );
    let tokens = params
        .get("tokens")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    assert!(!tokens.is_empty(), "the degraded report must re-send the tokens as unknown, not drop them (Herdr may merge reports per source)");
    for (name, value) in &tokens {
        assert_eq!(
            value.as_str(),
            Some("unknown"),
            "token {name} is stale, not unknown"
        );
    }

    // Across both waves: ttl_ms on pane reports only (spike section 4).
    for line in rig.herdr.requests() {
        match line.get("method").and_then(Value::as_str) {
            Some("pane.report_metadata") => assert!(line
                .get("params")
                .and_then(|p| p.get("ttl_ms"))
                .and_then(Value::as_u64)
                .is_some_and(|t| t > 0)),
            Some("workspace.report_metadata") => assert!(
                line.get("params").and_then(|p| p.get("ttl_ms")).is_none(),
                "a workspace report carries ttl_ms"
            ),
            other => {
                panic!("a method other than the two reports crossed the Herdr wire: {other:?}")
            }
        }
    }
    drop(reporter);
    drop(rig);
    drop(tmp);
}

#[test]
fn endpoints_come_from_the_environment() {
    let _lock = env_lock();
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("state");
    let herdr_socket = tmp.path().join("herdr.sock");
    let _guard = EnvGuard::set(&state, &herdr_socket);
    let endpoints = Endpoints::from_env().unwrap();
    assert_eq!(
        endpoints.herdr_socket, herdr_socket,
        "HERDR_SOCKET_PATH names the Herdr socket"
    );
    assert_eq!(
        endpoints.hub_control_socket,
        state.join("hub").join("control.sock"),
        "the hub socket resolves from HOLLER_STATE_DIR exactly as the hub's client does"
    );
}
