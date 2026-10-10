//! #732: the two-instance proof (epic #726). A second Holler instance is built beside a running
//! first one, on ONE loopback host, with the wizard's own helper scripts, and the first one is
//! shown to be untouched; then the second is torn down and the first is still untouched.
//!
//! Everything runs in a temp directory. `ps`, `ss`, `lsof`, `tailscale`, `herdr`, `holler` and
//! `opencode` are small fake bash scripts (`proof_fakes/`) placed first on `PATH`; the real ones
//! are never run. The only processes are harmless `sleep`s the fakes turn themselves into, each
//! started by this test and signalled only by the pid it recorded, never by name or pattern.
//! The fake `ps -e` lists only those registered processes, so no real process of the machine
//! can show up in an inventory.
//!
//! Unix-only: the scripts are bash and use `ps`/`kill`.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #732

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::hash::{Hash, Hasher};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const FAKES: [(&str, &str); 9] = [
    ("common.sh", include_str!("proof_fakes/common.sh")),
    ("ps", include_str!("proof_fakes/ps")),
    ("ss", include_str!("proof_fakes/ss")),
    ("lsof", include_str!("proof_fakes/lsof")),
    ("tailscale", include_str!("proof_fakes/tailscale")),
    ("herdr", include_str!("proof_fakes/herdr")),
    ("holler", include_str!("proof_fakes/holler")),
    ("opencode", include_str!("proof_fakes/opencode")),
    ("README", "fakes for the #732 two-instance proof\n"),
];

// ---------------------------------------------------------------------------------------------
// The host: one temp directory, the fakes, and the processes this test started.
// ---------------------------------------------------------------------------------------------

struct Host {
    root: tempfile::TempDir,
    real_ps: PathBuf,
    ports: RefCell<Vec<u16>>,
    owned: RefCell<Vec<(u32, String)>>,
}

impl Host {
    fn new() -> Host {
        let root = tempfile::tempdir().unwrap();
        for d in ["bin", "fake", "home", "tmp", "scratch"] {
            fs::create_dir_all(root.path().join(d)).unwrap();
        }
        for (name, body) in FAKES {
            let (dir, mode) = if name == "common.sh" || name == "README" {
                ("fake", 0o644)
            } else {
                ("bin", 0o755)
            };
            let p = root.path().join(dir).join(name);
            fs::write(&p, body).unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
        }
        let ports = free_ports(4);
        Host {
            root,
            real_ps: find_on_path("ps"),
            ports: RefCell::new(ports),
            owned: RefCell::new(Vec::new()),
        }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.path().join(rel)
    }

    fn bin(&self, name: &str) -> PathBuf {
        self.path("bin").join(name)
    }

    fn take_port(&self) -> u16 {
        self.ports.borrow_mut().pop().unwrap()
    }

    /// A command with a hermetic environment: fakes first on PATH, a temp HOME, and none of the
    /// variables that would tie it to the operator's real Herdr, hub or wizard run.
    fn cmd<S: AsRef<OsStr>>(&self, program: S) -> Command {
        let path = format!(
            "{}:{}",
            self.path("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut c = Command::new(program);
        c.env("PATH", path)
            .env("FAKE_ROOT", self.path("fake"))
            .env("FAKE_REAL_PS", &self.real_ps)
            .env("HOME", self.path("home"))
            .env("TMPDIR", self.path("tmp"))
            .env("WIZARD_LIB", lib_dir())
            .env("STOP_OWNED_GRACE", "5")
            .env_remove("HOLLER_STATE_DIR")
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SESSION")
            .env_remove("WIZARD_INVENTORY_FIXTURE")
            .stdin(Stdio::null());
        c
    }

    fn script(&self, name: &str) -> Command {
        let mut c = self.cmd("bash");
        c.arg(lib_dir().join(name));
        c
    }

    fn fake_file(&self, name: &str) -> String {
        fs::read_to_string(self.path("fake").join(name)).unwrap_or_default()
    }

    fn running(&self, pid: u32) -> bool {
        let o = self
            .cmd(&self.real_ps)
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let st = String::from_utf8_lossy(&o.stdout).trim().to_string();
        !st.is_empty() && !st.starts_with('Z')
    }

    fn command_of(&self, pid: u32) -> String {
        let o = self
            .cmd(&self.real_ps)
            .args(["-o", "command=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }

    /// Wait until the pid shows the fake's final command line (a `sleep` renamed by `exec -a`),
    /// then remember it so a failed test still cleans up its own processes by pid.
    fn adopt(&self, pid: u32, prefix: &str) {
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            let cmd = self.command_of(pid);
            if cmd.starts_with(prefix) {
                self.owned.borrow_mut().push((pid, cmd));
                return;
            }
            assert!(
                Instant::now() < end,
                "pid {pid} never became `{prefix}`: {cmd}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        for (pid, cmd) in self.owned.borrow().iter() {
            if self.running(*pid) && self.command_of(*pid) == *cmd {
                // SAFETY: the pid is a harmless sleep this test started and just re-identified.
                unsafe {
                    libc::kill(*pid as libc::pid_t, libc::SIGTERM);
                }
            }
        }
    }
}

fn lib_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../agent-skills/setup-wizard/lib")
}

fn find_on_path(prog: &str) -> PathBuf {
    let path = std::env::var("PATH").unwrap_or_default();
    path.split(':')
        .map(|d| Path::new(d).join(prog))
        .find(|p| p.is_file())
        .unwrap()
}

/// `n` distinct free loopback ports (held together so they cannot repeat).
fn free_ports(n: usize) -> Vec<u16> {
    let held: Vec<TcpListener> = (0..n)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    held.iter()
        .map(|l| l.local_addr().unwrap().port())
        .collect()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn ok(o: Output) -> Output {
    assert!(o.status.success(), "{}", text(&o));
    o
}

// ---------------------------------------------------------------------------------------------
// An instance: the values the `[instance]` table would hold, and where its files live.
// ---------------------------------------------------------------------------------------------

struct Inst {
    name: String,
    session: String,
    hub_port: u16,
    serve_port: u16,
    backend_port: u16,
    herdr_session: String,
    state: PathBuf,
}

impl Inst {
    fn new(host: &Host, name: &str, session: &str, serve_port: u16) -> Inst {
        Inst {
            name: name.to_string(),
            session: session.to_string(),
            hub_port: host.take_port(),
            serve_port,
            backend_port: host.take_port(),
            herdr_session: name.to_string(),
            state: host.path(&format!("state-{name}")),
        }
    }

    fn first(host: &Host) -> Inst {
        Inst::new(host, "inst-a", "alpha", 8444)
    }

    fn second(host: &Host) -> Inst {
        Inst::new(host, "inst-b", "gamma", 8443)
    }

    fn logs(&self) -> PathBuf {
        self.state.join("logs")
    }

    fn ledger_file(&self) -> PathBuf {
        self.state.join("wizard-ledger.toml")
    }

    fn config(&self, host: &Host) -> PathBuf {
        host.path("scratch")
            .join(format!("{}-sessions.toml", self.name))
    }

    fn write_config(&self, host: &Host) -> PathBuf {
        let body = format!(
            "hub_host = \"hub-host.example.ts.net\"\n\n[instance]\nname = \"{}\"\n\
             hub_port = {}\nserve_https_port = {}\nstate_dir = \"{}\"\n\
             herdr_session = \"{}\"\nbackend_port_base = {}\n\n\
             [[session]]\nname = \"{}\"\nharness = \"opencode\"\n",
            self.name,
            self.hub_port,
            self.serve_port,
            self.state.display(),
            self.herdr_session,
            self.backend_port,
            self.session
        );
        let p = self.config(host);
        fs::write(&p, body).unwrap();
        p
    }

    /// The contract values as the skill exports them for `collide.sh` and `herdr.sh`.
    fn plan_env(&self, c: &mut Command) {
        c.env("WIZARD_INSTANCE_NAME", &self.name)
            .env("WIZARD_INSTANCE_PREFIX", &self.name)
            .env("WIZARD_HUB_PORT", self.hub_port.to_string())
            .env("WIZARD_SERVE_HTTPS_PORT", self.serve_port.to_string())
            .env("WIZARD_STATE_DIR", &self.state)
            .env("WIZARD_HERDR_SESSION", &self.herdr_session)
            .env("WIZARD_BACKEND_PORTS", self.backend_port.to_string())
            .env("WIZARD_SESSION_NAMES", &self.session)
            .env("WIZARD_LEDGER", self.ledger_file())
            .env("WIZARD_LOG_DIR", self.logs());
    }
}

// ---------------------------------------------------------------------------------------------
// The wizard's stages, driven the way the skill tells the agent to.
// ---------------------------------------------------------------------------------------------

/// Stage 1: validate the `[instance]` table with `instance.sh`.
fn stage1_validate(host: &Host, i: &Inst) {
    let cfg = i.write_config(host);
    let out = ok(host.script("instance.sh").arg(&cfg).output().unwrap());
    let t = stdout(&out);
    assert!(t.contains(&format!("name={} ", i.name)), "{t}");
    assert!(
        t.contains(&format!("backend_port={}", i.backend_port)),
        "{t}"
    );
}

/// Stage 2: the read-only inventory of this host, into the scratch directory.
fn stage2_inventory(host: &Host, i: &Inst) -> PathBuf {
    let out = ok(host.script("inventory.sh").output().unwrap());
    let p = host
        .path("scratch")
        .join(format!("inventory-{}.tsv", i.name));
    fs::write(&p, out.stdout).unwrap();
    p
}

/// Stage 3: the collision preflight; the caller decides what a refusal means.
fn stage3_preflight(host: &Host, i: &Inst, inventory: &Path) -> Output {
    let mut c = host.script("collide.sh");
    c.arg("this-host").arg(inventory);
    i.plan_env(&mut c);
    c.output().unwrap()
}

/// Start a program the way the skill does (`nohup ... > log 2>&1 & echo $!`), returning its pid
/// once it has become the fake's final process.
fn start_detached(host: &Host, state: Option<&Path>, log: &Path, argv: &[&str]) -> u32 {
    let mut c = host.cmd("bash");
    c.args([
        "-c",
        r#"log=$1; shift; nohup "$@" >"$log" 2>&1 & echo $!"#,
        "_",
    ])
    .arg(log)
    .args(argv);
    if let Some(s) = state {
        c.env("HOLLER_STATE_DIR", s);
    }
    let pid: u32 = stdout(&ok(c.output().unwrap())).trim().parse().unwrap();
    host.adopt(pid, &argv_prefix(argv));
    pid
}

/// What the process's command line starts with once the fake has become its final self.
fn argv_prefix(argv: &[&str]) -> String {
    match argv[0] {
        "holler" => argv[..2].join(" "),
        other => other.to_string(),
    }
}

fn record(host: &Host, i: &Inst, pid: u32, role: &str, stage: u32, session: &str) {
    let mut c = host.script("ledger.sh");
    c.env("HOLLER_STATE_DIR", &i.state).args([
        "record",
        "--pid",
        &pid.to_string(),
        "--role",
        role,
        "--stage",
        &stage.to_string(),
        "--session",
        session,
    ]);
    ok(c.output().unwrap());
}

/// `stop-owned.sh check-port` must say FREE before a planned port is used.
fn check_port_free(host: &Host, i: &Inst, port: u16) {
    let mut c = host.script("stop-owned.sh");
    c.args(["check-port"]).arg(&i.state).arg(port.to_string());
    let out = ok(c.output().unwrap());
    assert!(stdout(&out).contains("FREE"), "{}", text(&out));
}

/// Stage 4: the instance's backend, on its own port; recorded once it is up.
fn stage4_backend(host: &Host, i: &Inst) {
    fs::create_dir_all(i.logs()).unwrap();
    check_port_free(host, i, i.backend_port);
    let port = i.backend_port.to_string();
    let log = i
        .logs()
        .join(format!("{}-opencode-{}.log", i.name, i.session));
    let argv = [
        "opencode",
        "--port",
        &port,
        "--hostname",
        "0.0.0.0",
        "--model",
        "p/m",
    ];
    let pid = start_detached(host, None, &log, &argv);
    record(host, i, pid, "backend", 4, &i.session);
}

/// Stage 6: the hub on the instance's port and state directory, then `tailscale serve`.
fn stage6_hub(host: &Host, i: &Inst) {
    check_port_free(host, i, i.hub_port);
    let listen = format!("127.0.0.1:{}", i.hub_port);
    let log = i.logs().join(format!("{}-hub.log", i.name));
    let argv = [
        "holler",
        "hub",
        "serve",
        "--listen",
        &listen,
        "--advertise",
        "hub-host.example.ts.net",
    ];
    let pid = start_detached(host, Some(&i.state), &log, &argv);
    let serve = [
        "serve".to_string(),
        "--bg".to_string(),
        "--https".to_string(),
        i.serve_port.to_string(),
        i.hub_port.to_string(),
    ];
    ok(host
        .cmd(host.bin("tailscale"))
        .args(serve)
        .output()
        .unwrap());
    let status = stdout(&ok(client(host, i, &["hub", "status"])));
    assert!(status.contains(&format!("listening: {listen}")), "{status}");
    record(host, i, pid, "hub", 6, "");
}

/// Stage 7: mint a token for this hub, join, and run one body on this host with the same state
/// directory; recorded after the roster shows its session.
fn stage7_body(host: &Host, i: &Inst) {
    let label = format!("{}-this-host", i.name);
    let token = stdout(&ok(client(
        host,
        i,
        &["hub", "token", "mint", "--label", &label],
    )));
    let join = [
        "body",
        "join",
        "--server",
        "wss://hub-host.example.ts.net",
        "--token",
    ];
    ok(client_with(host, i, &join, token.trim()));
    let cfg = i.state.join(format!("{}-sessions.toml", i.name));
    fs::write(&cfg, format!("[[session]]\nname = \"{}\"\n", i.session)).unwrap();
    let log = i.logs().join(format!("{}-holler-body.log", i.name));
    let cfg_arg = cfg.display().to_string();
    let argv = [
        "holler", "body", "run", "--config", &cfg_arg, "--debug", "quiet",
    ];
    let pid = start_detached(host, Some(&i.state), &log, &argv);
    let roster = stdout(&ok(client(host, i, &["roster"])));
    assert!(
        roster.contains(&format!("{} connected", i.session)),
        "{roster}"
    );
    record(host, i, pid, "body", 7, &i.session);
}

/// Stage 8: the instance's own Herdr session, only through `herdr.sh`.
fn stage8_herdr(host: &Host, i: &Inst) {
    for verb in ["check-pane", "check-session", "server-start"] {
        ok(herdr_sh(host, i, &[verb]));
    }
    let pid = find_herdr_pid(host, i);
    let status = stdout(&ok(herdr_sh(host, i, &["run", "status"])));
    assert!(status.contains("server: running"), "{status}");
    record(host, i, pid, "herdr", 8, &i.herdr_session);
}

fn herdr_sh(host: &Host, i: &Inst, args: &[&str]) -> Output {
    let mut c = host.script("herdr.sh");
    i.plan_env(&mut c);
    c.args(args);
    c.output().unwrap()
}

/// `herdr.sh server-start` backgrounds the server and prints no pid, so the pid is read from
/// the inventory: the herdr process whose `--session` is this instance's.
fn find_herdr_pid(host: &Host, i: &Inst) -> u32 {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        let inv = stdout(&ok(host.script("inventory.sh").output().unwrap()));
        for line in inv.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() > 4 && f[0] == "herdr" && f[4] == i.herdr_session {
                let pid: u32 = f[1].parse().unwrap();
                host.adopt(pid, "herdr --session");
                return pid;
            }
        }
        assert!(
            Instant::now() < end,
            "no herdr server for session {}",
            i.herdr_session
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Client commands: always with the instance's own `HOLLER_STATE_DIR`.
fn client(host: &Host, i: &Inst, args: &[&str]) -> Output {
    client_with(host, i, args, "")
}

/// Like `client`, with one more argument (a token) when it is not empty.
fn client_with(host: &Host, i: &Inst, args: &[&str], extra: &str) -> Output {
    let mut c = host.cmd(host.bin("holler"));
    c.env("HOLLER_STATE_DIR", &i.state).args(args);
    if !extra.is_empty() {
        c.arg(extra).args(["--hub-key", "fake-identity-key"]);
    }
    c.output().unwrap()
}

/// The whole of the wizard's run for one instance. A refused preflight returns the refusal and
/// nothing has been started or written by then.
fn build(host: &Host, i: &Inst) -> Result<(), String> {
    stage1_validate(host, i);
    let inventory = stage2_inventory(host, i);
    let pre = stage3_preflight(host, i, &inventory);
    if !pre.status.success() {
        return Err(text(&pre));
    }
    stage4_backend(host, i);
    stage6_hub(host, i);
    stage7_body(host, i);
    stage8_herdr(host, i);
    Ok(())
}

fn teardown(host: &Host, i: &Inst) -> Output {
    let mut c = host.script("stop-owned.sh");
    c.arg("teardown").arg(&i.state).arg("--purge-state");
    c.output().unwrap()
}

// ---------------------------------------------------------------------------------------------
// Snapshots of one instance: roster, ledger, processes, ports and a digest of its state.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct Snap {
    ledger: String,
    roster: String,
    procs: String,
    ports: String,
    state: BTreeMap<String, String>,
}

fn ledger_list(host: &Host, i: &Inst) -> String {
    let mut c = host.script("ledger.sh");
    c.env("HOLLER_STATE_DIR", &i.state).arg("list");
    stdout(&ok(c.output().unwrap()))
}

fn ledger_pids(listing: &str) -> Vec<u32> {
    listing
        .lines()
        .filter_map(|l| l.split('\t').next()?.parse().ok())
        .collect()
}

fn process_rows(host: &Host, pids: &[u32]) -> String {
    if pids.is_empty() {
        return String::new();
    }
    let list: Vec<String> = pids.iter().map(u32::to_string).collect();
    let mut c = host.cmd(&host.real_ps);
    c.env("LC_ALL", "C")
        .args(["-o", "pid=,lstart=,command=", "-p", &list.join(",")]);
    stdout(&c.output().unwrap())
}

/// Listening ports held by the given pids (from the inventory) and the serve line of the
/// instance's own https port.
fn port_rows(host: &Host, i: &Inst, pids: &[u32]) -> String {
    let inv = stdout(&ok(host.script("inventory.sh").output().unwrap()));
    let mut rows: Vec<&str> = inv
        .lines()
        .filter(|l| held_by(l, pids) || is_serve_of(l, i))
        .collect();
    rows.sort_unstable();
    rows.join("\n")
}

fn held_by(line: &str, pids: &[u32]) -> bool {
    let f: Vec<&str> = line.split('\t').collect();
    f.len() > 2 && f[0] == "port" && pids.iter().any(|p| f[2] == p.to_string())
}

fn is_serve_of(line: &str, i: &Inst) -> bool {
    let f: Vec<&str> = line.split('\t').collect();
    f.len() > 1 && f[0] == "serve" && f[1] == i.serve_port.to_string()
}

/// Every file under a directory: relative path to a hash of its mode, length and content.
fn digest(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    collect_digest(dir, dir, &mut out);
    out
}

fn collect_digest(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect_digest(root, &p, out);
            continue;
        }
        let bytes = fs::read(&p).unwrap();
        let mode = fs::metadata(&p).unwrap().permissions().mode();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut h);
        let rel = p.strip_prefix(root).unwrap().display().to_string();
        out.insert(rel, format!("{mode:o}/{}/{:x}", bytes.len(), h.finish()));
    }
}

fn snapshot(host: &Host, i: &Inst) -> Snap {
    let ledger = ledger_list(host, i);
    let pids = ledger_pids(&ledger);
    Snap {
        roster: stdout(&client(host, i, &["roster"])),
        procs: process_rows(host, &pids),
        ports: port_rows(host, i, &pids),
        state: digest(&i.state),
        ledger,
    }
}

// ---------------------------------------------------------------------------------------------
// The scenario's tests.
// ---------------------------------------------------------------------------------------------

/// The first instance, built and snapshotted.
fn first_up(host: &Host) -> (Inst, Snap) {
    let a = Inst::first(host);
    build(host, &a).unwrap();
    let snap = snapshot(host, &a);
    assert_populated(&snap, &a);
    (a, snap)
}

/// A snapshot that proves nothing if it is empty: A must really be up before it is compared.
fn assert_populated(s: &Snap, a: &Inst) {
    assert_eq!(s.ledger.lines().count(), 4, "{}", s.ledger);
    assert!(
        s.ledger.lines().all(|l| l.contains("\tlive\t")),
        "{}",
        s.ledger
    );
    assert_eq!(s.procs.lines().count(), 4, "{}", s.procs);
    assert_eq!(
        s.ports.lines().filter(|l| l.starts_with("port\t")).count(),
        2
    );
    assert!(
        s.ports.contains(&format!("serve\t{}\t", a.serve_port)),
        "{}",
        s.ports
    );
    assert!(s.state.contains_key("wizard-ledger.toml") && s.state.contains_key("hub.info"));
    assert_eq!(s.roster.trim(), "alpha connected");
}

#[test]
fn a_second_instance_is_built_beside_the_first_and_the_first_is_untouched() {
    let host = Host::new();
    let (a, before) = first_up(&host);
    let b = Inst::second(&host);
    build(&host, &b).unwrap();
    assert_eq!(before, snapshot(&host, &a));
}

/// What a run of the wizard leaves in the fake host's registries, to prove a refused preflight
/// started nothing.
fn registries(host: &Host) -> Vec<String> {
    ["pids", "ports", "sessions", "serve"]
        .iter()
        .map(|f| host.fake_file(f))
        .collect()
}

/// Everything of B that must be gone after its teardown.
fn assert_b_gone(host: &Host, b: &Inst, pids: &[u32]) {
    assert!(!pids.is_empty(), "B recorded no process");
    for pid in pids {
        assert!(!host.running(*pid), "B's pid {pid} still runs");
    }
    let inv = stdout(&ok(host.script("inventory.sh").output().unwrap()));
    for port in [b.hub_port, b.backend_port] {
        assert!(
            !inv.contains(&format!("port\t{port}\t")),
            "port {port} still held:\n{inv}"
        );
    }
    assert!(!inv.contains(&b.state.display().to_string()), "{inv}");
    assert!(
        !inv.contains(&format!("herdr-session\t{}", b.herdr_session)),
        "{inv}"
    );
    assert!(!b.state.exists(), "B's state directory is still there");
    let roster = client(host, b, &["roster"]);
    assert!(
        !roster.status.success(),
        "B still has a hub: {}",
        text(&roster)
    );
}

#[test]
fn after_the_second_is_torn_down_the_first_is_still_identical_and_the_second_is_gone() {
    let host = Host::new();
    let (a, before) = first_up(&host);
    let b = Inst::second(&host);
    build(&host, &b).unwrap();
    assert_eq!(before, snapshot(&host, &a), "A changed during B's run");
    let b_pids = ledger_pids(&ledger_list(&host, &b));
    assert_eq!(b_pids.len(), 4, "backend, hub, body and herdr");
    let out = ok(teardown(&host, &b));
    assert!(text(&out).contains("stopped 4"), "{}", text(&out));
    assert_b_gone(&host, &b, &b_pids);
    assert_eq!(before, snapshot(&host, &a));
}

#[test]
fn neither_instance_writes_under_the_default_state_directory() {
    let host = Host::new();
    let (a, _) = first_up(&host);
    let b = Inst::second(&host);
    build(&host, &b).unwrap();
    let default_dir = host.path("home").join(".holler");
    assert!(
        !default_dir.exists(),
        "something wrote {}",
        default_dir.display()
    );
    assert!(a.state.join("hub.info").is_file() && b.state.join("hub.info").is_file());
    assert_eq!(fs::read_dir(host.path("home")).unwrap().count(), 0);
}

/// Every `holler` call the fakes saw: (state directory it carried, arguments).
fn holler_calls(host: &Host) -> Vec<(String, String)> {
    host.fake_file("calls")
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("state=")?;
            let (state, args) = rest.split_once(" args=")?;
            Some((state.to_string(), args.to_string()))
        })
        .collect()
}

#[test]
fn every_client_command_carries_its_own_instances_state_directory() {
    let host = Host::new();
    let (a, _) = first_up(&host);
    let b = Inst::second(&host);
    build(&host, &b).unwrap();
    let on_b = stdout(&ok(client(&host, &b, &["roster"])));
    let on_a = stdout(&ok(client(&host, &a, &["roster"])));
    assert_eq!(on_b.trim(), "gamma connected");
    assert_eq!(on_a.trim(), "alpha connected");
    let (a_dir, b_dir) = (a.state.display().to_string(), b.state.display().to_string());
    let calls = holler_calls(&host);
    assert!(!calls.is_empty());
    for (state, args) in &calls {
        assert!(
            *state == a_dir || *state == b_dir,
            "`holler {args}` carried `{state}`"
        );
    }
    assert!(calls.iter().any(|(s, a)| *s == b_dir && a == "roster"));
}

#[test]
fn a_colliding_hub_port_is_refused_by_the_preflight_before_anything_starts() {
    let host = Host::new();
    let (a, before) = first_up(&host);
    let mut b = Inst::second(&host);
    b.hub_port = a.hub_port;
    let regs = registries(&host);
    let refusal = build(&host, &b).unwrap_err();
    assert!(refusal.contains("REFUSED"), "{refusal}");
    assert!(refusal.contains("hub_port"), "{refusal}");
    assert_nothing_started(&host, &b, &regs);
    assert_eq!(before, snapshot(&host, &a));
}

#[test]
fn a_colliding_herdr_session_is_refused_by_the_preflight_before_anything_starts() {
    let host = Host::new();
    let (a, before) = first_up(&host);
    let mut b = Inst::second(&host);
    b.herdr_session = a.herdr_session.clone();
    let regs = registries(&host);
    let refusal = build(&host, &b).unwrap_err();
    assert!(refusal.contains("REFUSED"), "{refusal}");
    assert!(refusal.contains("herdr_session"), "{refusal}");
    assert_nothing_started(&host, &b, &regs);
    let guard = herdr_sh(&host, &b, &["check-session"]);
    assert_eq!(guard.status.code(), Some(1), "{}", text(&guard));
    assert_eq!(before, snapshot(&host, &a));
}

fn assert_nothing_started(host: &Host, b: &Inst, regs: &[String]) {
    assert_eq!(regs, registries(host).as_slice(), "the host changed");
    assert!(!b.state.exists(), "B's state directory was created");
    let b_dir = b.state.display().to_string();
    assert!(holler_calls(host).iter().all(|(s, _)| *s != b_dir));
}
