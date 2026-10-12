#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! The Claude Code adapter with its three seams faked (#831, ADR-0021 section 15 items 5, 7, 8,
//! 9, 10, 14 and 15; plan review W8): no `claude` binary runs, no login is read, nothing is sent.
//!
//! The adapter is `ClaudeHarness`, built over a `ClaudeConfig` (the `claude` program, Claude
//! Code's configuration directory, where only a transcript's existence is checked, and the
//! adapter's own directory for the socket-path files its launch hook writes) and three seams,
//! each the raw I/O the adapter parses:
//!
//! - `AgentsList`: the stdout of `claude agents --json`;
//! - `InboxWrite`: one line written to a unix socket (the line holds no newline; the seam ends
//!   it);
//! - `LoginCheck`: the stdout of `claude auth status`, of which only `authMethod` is read.
//!
//! Delivery is checked end to end through the launch's own `SessionStart` hook: the test runs
//! the hook command the argv carries, as Claude Code would (through a shell, with the hook's JSON
//! on stdin and `CLAUDE_CODE_MESSAGING_SOCKET` set), so the file it writes and the key it uses
//! are the adapter's own choice (W8) and only the round trip is pinned.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};

use holler_adapter_claude::{AgentsList, ClaudeConfig, ClaudeHarness, InboxWrite, LoginCheck};
use holler_pane::pane::ModelSpec;
use holler_pane::ports::{Activity, LoginStatus, SessionChoice};
use holler_pane::{
    AgentKey, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
};
use serde_json::{json, Value};

/// The variables that silently change the login (item 10): each is unset on every launch argv.
const LOGIN_VARIABLES: [&str; 7] = [
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

const PANE: &str = "demo-c1r1";
const CWD: &str = "/srv/demo";
/// `CWD` with every non-alphanumeric character replaced by `-`: the transcript directory's name
/// (the spike's documented rule).
const PROJECT_DIR: &str = "-srv-demo";
const BIN: &str = "/opt/demo/bin/claude";
const SESSION: &str = "4f1c2b8e-0000-4000-8000-000000000001";

// --- the seams, faked ---------------------------------------------------------------------

/// `claude agents --json`, answered from a settable document.
struct Agents(Mutex<String>);

impl Agents {
    fn new(doc: Value) -> Arc<Self> {
        Arc::new(Self(Mutex::new(doc.to_string())))
    }
}

impl AgentsList for Agents {
    fn agents_json(&self) -> Result<String, PaneError> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone())
    }
}

/// Every line written, with the socket it was written to.
#[derive(Default)]
struct Inbox(Mutex<Vec<(PathBuf, String)>>);

impl Inbox {
    fn writes(&self) -> Vec<(PathBuf, String)> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl InboxWrite for Inbox {
    fn write_line(&self, socket: &Path, line: &str) -> Result<(), PaneError> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((socket.to_owned(), line.to_owned()));
        Ok(())
    }
}

/// `claude auth status`, answered from a fixed text.
struct Login(String);

impl LoginCheck for Login {
    fn auth_status_json(&self) -> Result<String, PaneError> {
        Ok(self.0.clone())
    }
}

/// One `agents --json` entry, in the documented shape.
fn entry(pid: u32, session: &str, status: &str, waiting_for: Option<&str>) -> Value {
    let mut e = json!({
        "cwd": CWD,
        "kind": "interactive",
        "name": "demo",
        "pid": pid,
        "sessionId": session,
        "startedAt": 1_760_000_000_000_i64,
        "status": status,
    });
    if let Some(reason) = waiting_for {
        e["waitingFor"] = json!(reason);
    }
    e
}

/// A scratch configuration directory and socket directory, and the harness over them.
struct Rig {
    _dir: tempfile::TempDir,
    config_dir: PathBuf,
    socket_dir: PathBuf,
    agents: Arc<Agents>,
    inbox: Arc<Inbox>,
    harness: ClaudeHarness,
}

impl Rig {
    fn new(agents: Value) -> Self {
        Self::with_login(agents, r#"{"loggedIn":true,"authMethod":"claude.ai"}"#)
    }

    fn with_login(agents: Value, login: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("claude-config");
        let socket_dir = dir.path().join("holler-state").join("claude");
        std::fs::create_dir_all(&config_dir).unwrap();
        let agents = Agents::new(agents);
        let inbox = Arc::new(Inbox::default());
        let harness = ClaudeHarness::with_seams(
            ClaudeConfig::new(BIN, &config_dir, &socket_dir),
            agents.clone(),
            inbox.clone(),
            Arc::new(Login(login.to_owned())),
        );
        Self {
            _dir: dir,
            config_dir,
            socket_dir,
            agents,
            inbox,
            harness,
        }
    }

    /// Make `session`'s transcript exist (its contents are a secret the adapter never reads).
    fn seed_transcript(&self, session: &str) {
        let dir = self.config_dir.join("projects").join(PROJECT_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{session}.jsonl")), "not-to-be-read\n").unwrap();
    }

    fn argv(&self, choice: SessionChoice) -> Argv {
        self.harness
            .process_command(&name(PANE), &spec(), &choice)
            .unwrap()
    }
}

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn spec() -> ServeSpec {
    ServeSpec::default()
        .with_model(ModelSpec {
            provider: "demo-provider".to_owned(),
            model_id: "demo-model".to_owned(),
            effort: "high".to_owned(),
        })
        .with_cwd(CWD)
}

fn endpoint(pid: u32) -> HarnessEndpoint {
    HarnessEndpoint::Process {
        pane: name(PANE),
        pid,
    }
}

fn has_pair(argv: &Argv, flag: &str, value: &str) -> bool {
    argv.as_slice()
        .windows(2)
        .any(|w| w[0] == flag && w[1] == value)
}

fn has(argv: &Argv, element: &str) -> bool {
    argv.as_slice().iter().any(|e| e == element)
}

/// Where the program is in `argv`.
fn program_at(argv: &Argv) -> usize {
    argv.as_slice()
        .iter()
        .position(|e| e == BIN)
        .unwrap_or_else(|| panic!("the argv runs {BIN}: {argv:?}"))
}

/// The `--settings` JSON of `argv`.
fn settings(argv: &Argv) -> Value {
    let at = argv
        .as_slice()
        .iter()
        .position(|e| e == "--settings")
        .unwrap_or_else(|| panic!("--settings in {argv:?}"));
    serde_json::from_str(&argv.as_slice()[at + 1]).expect("--settings is inline JSON")
}

/// Every command string of the `SessionStart` hooks in `settings`.
fn session_start_commands(settings: &Value) -> Vec<String> {
    let mut found = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(map) => {
                if let Some(Value::String(command)) = map.get("command") {
                    out.push(command.clone());
                }
                map.values().for_each(|v| walk(v, out));
            }
            Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    walk(&settings["hooks"]["SessionStart"], &mut found);
    found
}

/// The `NAME=value` assignments `env` sets before the program (not the `-u NAME` unsets).
fn env_assignments(argv: &Argv) -> BTreeMap<String, String> {
    let before = &argv.as_slice()[..program_at(argv)];
    let mut out = BTreeMap::new();
    let mut skip = false;
    for element in before {
        if skip {
            skip = false;
            continue;
        }
        if element == "-u" {
            skip = true;
            continue;
        }
        if let Some((k, v)) = element.split_once('=') {
            out.insert(k.to_owned(), v.to_owned());
        }
    }
    out
}

// --- process_command (item 5, 10, 11) --------------------------------------------------------

#[test]
fn a_fresh_launch_argv_holds_the_session_the_model_and_the_settings() {
    let rig = Rig::new(json!([]));
    let argv = rig.argv(SessionChoice::Fresh(SESSION.to_owned()));
    assert_eq!(argv.as_slice()[0], "env", "{argv:?}");
    assert!(has_pair(&argv, "--session-id", SESSION), "{argv:?}");
    assert!(!has(&argv, "--resume"), "{argv:?}");
    assert!(has_pair(&argv, "--model", "demo-model"), "{argv:?}");
    assert!(has_pair(&argv, "--effort", "high"), "{argv:?}");
    assert!(
        !has(&argv, "demo-provider"),
        "the provider is recorded, not passed: {argv:?}"
    );
    assert!(
        has_pair(&argv, "--disallowed-tools", "AskUserQuestion"),
        "{argv:?}"
    );
    let settings = settings(&argv);
    assert_eq!(
        settings["crossSessionInbound"],
        json!("accept"),
        "{settings}"
    );
    assert!(
        !session_start_commands(&settings).is_empty(),
        "a SessionStart hook tells Holler the socket path: {settings}"
    );
}

#[test]
fn every_login_variable_is_unset_before_the_program_and_none_is_set() {
    let rig = Rig::new(json!([]));
    let argv = rig.argv(SessionChoice::Fresh(SESSION.to_owned()));
    let before = &argv.as_slice()[..program_at(&argv)];
    for variable in LOGIN_VARIABLES {
        assert!(
            before.windows(2).any(|w| w[0] == "-u" && w[1] == variable),
            "env -u {variable} before the program: {argv:?}"
        );
        assert!(
            !argv
                .as_slice()
                .iter()
                .any(|e| e.starts_with(&format!("{variable}="))),
            "{variable} is never given a value: {argv:?}"
        );
    }
}

#[test]
fn keep_resumes_a_session_whose_transcript_exists_and_starts_one_that_has_none() {
    let rig = Rig::new(json!([]));
    let without = rig.argv(SessionChoice::Keep(SESSION.to_owned()));
    assert!(has_pair(&without, "--session-id", SESSION), "{without:?}");
    assert!(!has(&without, "--resume"));

    rig.seed_transcript(SESSION);
    let with = rig.argv(SessionChoice::Keep(SESSION.to_owned()));
    assert!(has_pair(&with, "--resume", SESSION), "{with:?}");
    assert!(!has(&with, "--session-id"), "{with:?}");
}

#[test]
fn resume_of_a_session_with_no_transcript_is_session_not_found() {
    let rig = Rig::new(json!([]));
    let refused = rig
        .harness
        .process_command(
            &name(PANE),
            &spec(),
            &SessionChoice::Resume(SESSION.to_owned()),
        )
        .unwrap_err();
    assert_eq!(refused.code(), "session-not-found", "{refused}");

    rig.seed_transcript(SESSION);
    let argv = rig.argv(SessionChoice::Resume(SESSION.to_owned()));
    assert!(has_pair(&argv, "--resume", SESSION), "{argv:?}");
}

#[test]
fn an_agent_is_refused_at_the_plan() {
    let rig = Rig::new(json!([]));
    let with_agent = spec().with_agent(AgentKey::parse("reviewer").unwrap());
    let refused = rig
        .harness
        .process_command(
            &name(PANE),
            &with_agent,
            &SessionChoice::Fresh(SESSION.to_owned()),
        )
        .unwrap_err();
    assert_eq!(refused.code(), "agent-unsupported", "{refused}");
}

#[test]
fn new_session_ids_are_distinct_uuids() {
    let rig = Rig::new(json!([]));
    let a = rig.harness.new_session_id().unwrap();
    let b = rig.harness.new_session_id().unwrap();
    assert_ne!(a, b);
    for id in [a, b] {
        let groups: Vec<usize> = id.split('-').map(str::len).collect();
        assert_eq!(groups, [8, 4, 4, 4, 12], "a UUID: {id}");
        assert!(
            id.chars().all(|c| c == '-' || c.is_ascii_hexdigit()),
            "{id}"
        );
    }
}

// --- the login (item 10) --------------------------------------------------------------------

#[test]
fn login_status_reads_the_auth_method_alone() {
    let ok = Rig::with_login(
        json!([]),
        r#"{"loggedIn":true,"authMethod":"claude.ai","email":"someone@example.invalid","orgName":"Example Org"}"#,
    );
    assert_eq!(ok.harness.login_status().unwrap(), LoginStatus::Ok);
    for other in ["api_key", "oauth_token", "none"] {
        let rig = Rig::with_login(json!([]), &format!(r#"{{"authMethod":"{other}"}}"#));
        assert_eq!(
            rig.harness.login_status().unwrap(),
            LoginStatus::Missing,
            "{other}"
        );
    }
}

#[test]
fn an_unreadable_login_answer_quotes_none_of_it() {
    let rig = Rig::with_login(json!([]), "not json: secret-token-0123456789");
    match rig.harness.login_status() {
        Ok(status) => assert_eq!(
            status,
            LoginStatus::Missing,
            "never Ok on an unreadable answer"
        ),
        Err(error) => assert!(
            !error.to_string().contains("secret-token"),
            "the error quotes no output: {error}"
        ),
    }
}

// --- observation (items 3 and 8) ----------------------------------------------------------

#[test]
fn the_process_is_located_and_read_from_agents_json_by_its_pid() {
    let rig = Rig::new(json!([
        entry(4242, SESSION, "idle", None),
        entry(5151, "other", "busy", None)
    ]));
    let h = &rig.harness;
    assert_eq!(h.locate_process(&name(PANE), SESSION).unwrap(), Some(4242));
    assert_eq!(
        h.locate_process(&name(PANE), "no-such-session").unwrap(),
        None
    );
    assert!(h.health(&endpoint(4242)).unwrap());
    assert!(
        !h.health(&endpoint(7777)).unwrap(),
        "no entry for the pid: not healthy"
    );
    let pane = PaneId::new("scratch:demo-c1r1");
    assert_eq!(
        h.shown_session(&pane, &endpoint(4242)).unwrap(),
        Some(SESSION.to_owned())
    );
}

#[test]
fn activity_maps_status_and_waiting_for() {
    for (status, waiting_for, want) in [
        ("idle", None, Activity::Idle),
        ("busy", None, Activity::Busy),
        (
            "waiting",
            Some("permission prompt"),
            Activity::WaitingPermission,
        ),
        (
            "waiting",
            Some("sandbox request"),
            Activity::WaitingPermission,
        ),
        (
            "waiting",
            Some("worker request"),
            Activity::WaitingPermission,
        ),
        ("waiting", Some("input needed"), Activity::HoldingQuestion),
    ] {
        let rig = Rig::new(json!([entry(4242, SESSION, status, waiting_for)]));
        assert_eq!(
            rig.harness
                .session_activity(&endpoint(4242), SESSION)
                .unwrap(),
            want,
            "{status} {waiting_for:?}"
        );
    }
}

// --- abort (item 9) -----------------------------------------------------------------------

#[test]
fn abort_is_refused_abort_unsupported() {
    let rig = Rig::new(json!([entry(4242, SESSION, "busy", None)]));
    let refused = rig.harness.abort(&endpoint(4242), SESSION).unwrap_err();
    assert_eq!(refused.code(), "abort-unsupported", "{refused}");
}

// --- delivery (item 7, W8) ------------------------------------------------------------------

/// Run every `SessionStart` hook of `argv` as Claude Code would: through `sh -c`, with the
/// argv's own `NAME=value` assignments, `CLAUDE_CODE_MESSAGING_SOCKET` set to `socket`, and the
/// hook's JSON on stdin. The hook's parent is this test process, whose pid the agents entry
/// then carries.
fn run_session_start_hook(argv: &Argv, session: &str, socket: &Path) {
    let commands = session_start_commands(&settings(argv));
    assert!(!commands.is_empty(), "a SessionStart hook: {argv:?}");
    for command in commands {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .envs(env_assignments(argv))
            .env("CLAUDE_CODE_MESSAGING_SOCKET", socket)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("sh runs the hook");
        let input = json!({
            "session_id": session,
            "hook_event_name": "SessionStart",
            "source": "startup",
            "cwd": CWD,
        });
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "the hook succeeds: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Every file and directory under `root`, with its permission bits.
fn modes_under(root: &Path) -> Vec<(PathBuf, bool, u32)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = std::fs::metadata(&path).unwrap();
            let mode = meta.permissions().mode() & 0o777;
            if meta.is_dir() {
                stack.push(path.clone());
            }
            out.push((path, meta.is_dir(), mode));
        }
    }
    out
}

#[test]
fn a_prompt_reaches_the_socket_the_launch_hook_reported_as_one_framed_line() {
    let me = std::process::id();
    let rig = Rig::new(json!([entry(me, SESSION, "idle", None)]));
    let argv = rig.argv(SessionChoice::Fresh(SESSION.to_owned()));
    let socket = rig
        .socket_dir
        .parent()
        .unwrap()
        .join("cc-socks")
        .join(format!("{me}.sock"));
    run_session_start_hook(&argv, SESSION, &socket);

    let text = "say \"hello\"\nthen stop";
    rig.harness
        .send_prompt(&endpoint(me), SESSION, text)
        .unwrap();

    let writes = rig.inbox.writes();
    assert_eq!(writes.len(), 1, "one line: {writes:?}");
    let (to, line) = &writes[0];
    assert_eq!(to, &socket, "the socket the hook reported");
    assert!(!line.contains('\n'), "one line, no newline in it: {line:?}");
    let frame: Value = serde_json::from_str(line).expect("the line is JSON");
    assert_eq!(
        frame,
        json!({"type": "user", "message": {"role": "user", "content": text}})
    );

    let own = std::fs::metadata(&rig.socket_dir)
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(own, 0o700, "the adapter's directory is the user's alone");
    let written = modes_under(&rig.socket_dir);
    assert!(
        !written.is_empty(),
        "the hook wrote under the adapter's directory"
    );
    for (path, is_dir, mode) in written {
        let want = if is_dir { 0o700 } else { 0o600 };
        assert_eq!(mode, want, "{} is {mode:o}", path.display());
    }
}

#[test]
fn a_prompt_with_no_hook_report_fails_closed_and_never_guesses_a_socket() {
    let me = std::process::id();
    let rig = Rig::new(json!([entry(me, SESSION, "idle", None)]));
    // Claude Code's own registry file holds a socket path: the adapter must not read it (the
    // note's decision 4), nor guess `/tmp/cc-socks/<pid>.sock`.
    let sessions = rig.config_dir.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::write(
        sessions.join(format!("{me}.json")),
        json!({"pid": me, "sessionId": SESSION, "messagingSocketPath": "/tmp/cc-socks/guess.sock"})
            .to_string(),
    )
    .unwrap();

    let refused = rig
        .harness
        .send_prompt(&endpoint(me), SESSION, "hello")
        .unwrap_err();
    assert_eq!(refused.code(), "unavailable", "{refused}");
    assert_eq!(rig.inbox.writes(), vec![], "nothing written");
}

#[test]
fn a_prompt_to_a_session_the_process_does_not_hold_is_session_mismatch() {
    let me = std::process::id();
    let rig = Rig::new(json!([entry(me, SESSION, "idle", None)]));
    let argv = rig.argv(SessionChoice::Fresh(SESSION.to_owned()));
    let socket = rig.socket_dir.parent().unwrap().join("inbox.sock");
    run_session_start_hook(&argv, SESSION, &socket);

    let refused = rig
        .harness
        .send_prompt(
            &endpoint(me),
            "4f1c2b8e-0000-4000-8000-00000000ffff",
            "hello",
        )
        .unwrap_err();
    assert_eq!(refused.code(), "session-mismatch", "{refused}");
    assert_eq!(rig.inbox.writes(), vec![], "nothing written");
}

#[test]
fn a_prompt_after_a_persons_clear_follows_the_new_session_only() {
    let me = std::process::id();
    let rig = Rig::new(json!([entry(me, SESSION, "idle", None)]));
    let argv = rig.argv(SessionChoice::Fresh(SESSION.to_owned()));
    let socket = rig.socket_dir.parent().unwrap().join("inbox.sock");
    run_session_start_hook(&argv, SESSION, &socket);

    // A person's `/clear`: the same process now holds another session, and the hook runs again.
    let cleared = "4f1c2b8e-0000-4000-8000-00000000c1ea";
    *rig.agents.0.lock().unwrap() = json!([entry(me, cleared, "idle", None)]).to_string();
    run_session_start_hook(&argv, cleared, &socket);

    let stale = rig
        .harness
        .send_prompt(&endpoint(me), SESSION, "hello")
        .unwrap_err();
    assert_eq!(stale.code(), "session-mismatch", "{stale}");
    rig.harness
        .send_prompt(&endpoint(me), cleared, "hello")
        .unwrap();
    assert_eq!(rig.inbox.writes().len(), 1);
}
