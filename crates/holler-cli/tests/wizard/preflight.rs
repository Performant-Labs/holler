//! Setup-wizard collision preflight (#728, epic #726): `inventory.sh` lists what already runs
//! on a host (read-only) and `collide.sh` refuses a plan that collides with something the
//! wizard did not create.
//!
//! Every test runs the real scripts in its own temp directory, with fakes of `ss`, `lsof`,
//! `ps`, `tailscale` and `herdr` placed first on `PATH`. The fakes log every call so the
//! read-only property is checked, not assumed. Unix-only (bash scripts).
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #728

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const FIRST_HUB_CMD: &str = "/usr/bin/holler hub --listen 127.0.0.1:41807";
const FIRST_HUB_STARTED: &str = "Sat Oct 10 09:00:00 2026";

fn lib_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../agent-skills/setup-wizard/lib")
}

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Self {
        let e = Env {
            dir: tempfile::tempdir().unwrap(),
        };
        fs::create_dir(e.bin()).unwrap();
        fs::create_dir(e.work()).unwrap();
        e
    }
    fn bin(&self) -> PathBuf {
        self.dir.path().join("bin")
    }
    fn work(&self) -> PathBuf {
        self.dir.path().join("work")
    }
    fn log(&self) -> PathBuf {
        self.dir.path().join("calls.log")
    }
    fn file(&self, name: &str, body: &str) -> PathBuf {
        let p = self.dir.path().join(name);
        fs::write(&p, body).unwrap();
        p
    }
    /// A fake executable that logs `name args...` and then runs `body`.
    fn fake(&self, name: &str, body: &str) {
        let p = self.bin().join(name);
        let script = format!(
            "#!/bin/bash\necho \"{name} $*\" >> \"{log}\"\n{body}\n",
            log = self.log().display()
        );
        fs::write(&p, script).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.log())
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.bin().display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }
    fn script(&self, name: &str) -> Command {
        let mut c = Command::new("bash");
        c.arg(lib_dir().join(name))
            .current_dir(self.work())
            .env("PATH", self.path())
            .env("WIZARD_LIB", lib_dir())
            .env_remove("WIZARD_INVENTORY_FIXTURE")
            .stdin(Stdio::null());
        c
    }
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// An inventory holding one running instance: its hub, its body for session `alpha`, its
/// backend, a `tailscale serve` on 443 and a Herdr session `first`.
fn first_instance_inventory() -> String {
    let t = "\t";
    [
        format!("port{t}41807{t}111{t}holler"),
        format!("port{t}47001{t}222{t}opencode"),
        format!(
            "hub{t}111{t}<user>{t}/home/<user>/.holler{t}41807{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}",
        ),
        format!(
            "body{t}333{t}<user>{t}/home/<user>/.holler{t}alpha{t}{FIRST_HUB_STARTED}{t}/usr/bin/holler body run --session alpha",
        ),
        format!(
            "opencode{t}222{t}<user>{t}-{t}47001{t}{FIRST_HUB_STARTED}{t}opencode serve --port 47001",
        ),
        format!("serve{t}443{t}https://<hub-host>.example.ts.net (tailnet only)"),
        format!("herdr-session{t}first"),
    ]
    .join("\n")
        + "\n"
}

/// Plan env for a second instance that collides with nothing in the inventory above.
fn distinct_plan(c: &mut Command) {
    c.env("WIZARD_INSTANCE_NAME", "second")
        .env("WIZARD_HUB_PORT", "41808")
        .env("WIZARD_SERVE_HTTPS_PORT", "8443")
        .env("WIZARD_STATE_DIR", "/home/<user>/.holler-second")
        .env("WIZARD_HERDR_SESSION", "second")
        .env("WIZARD_BACKEND_PORTS", "47101 47102")
        .env("WIZARD_SESSION_NAMES", "gamma delta");
}

/// Plan env for an all-defaults second instance (the contract's defaults).
fn default_plan(c: &mut Command) {
    c.env("WIZARD_INSTANCE_NAME", "default")
        .env("WIZARD_HUB_PORT", "41807")
        .env("WIZARD_SERVE_HTTPS_PORT", "443")
        .env("WIZARD_STATE_DIR", "")
        .env("WIZARD_HERDR_SESSION", "")
        .env("WIZARD_BACKEND_PORTS", "47001 47002")
        .env("WIZARD_SESSION_NAMES", "alpha beta");
}

fn collide(env: &Env, inventory: &Path, plan: impl Fn(&mut Command)) -> Output {
    let mut c = env.script("collide.sh");
    c.arg("<hub-host>").arg(inventory);
    plan(&mut c);
    c.output().unwrap()
}

// ---------------------------------------------------------------- inventory.sh

#[test]
fn inventory_replays_a_fixture_without_running_anything() {
    let env = Env::new();
    for n in ["ss", "lsof", "ps", "tailscale", "herdr"] {
        env.fake(n, "exit 0");
    }
    let fixture = env.file("inv.tsv", &first_instance_inventory());
    let out = env
        .script("inventory.sh")
        .env("WIZARD_INVENTORY_FIXTURE", &fixture)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        first_instance_inventory()
    );
    assert!(env.calls().is_empty(), "fixture mode ran {:?}", env.calls());
}

fn install_live_fakes(env: &Env) {
    env.fake(
        "ss",
        "echo 'LISTEN 0 128 127.0.0.1:41807 0.0.0.0:* users:((\"holler\",pid=111,fd=5))'\n\
         echo 'LISTEN 0 128 127.0.0.1:47001 0.0.0.0:* users:((\"opencode\",pid=222,fd=7))'\n\
         echo 'LISTEN 0 128 [::]:22 [::]:*'",
    );
    env.fake(
        "ps",
        "echo '  111 <user> Sat Oct 10 09:00:00 2026 /usr/bin/holler hub --listen 127.0.0.1:41807'\n\
         echo '  222 <user> Sat Oct 10 09:00:01 2026 opencode serve --port 47001'\n\
         echo '  333 <user> Sat Oct 10 09:00:02 2026 /usr/bin/holler body run --session alpha'\n\
         echo '  444 <user> Sat Oct 10 09:00:03 2026 /usr/sbin/sshd -D'",
    );
    env.fake(
        "tailscale",
        "echo 'https://<hub-host>.example.ts.net (tailnet only)'\n\
         echo '|-- / proxy http://127.0.0.1:41807'\n\
         echo ''\n\
         echo 'https://<hub-host>.example.ts.net:8444 (tailnet only)'",
    );
    env.fake("herdr", "echo first");
    // Anything that could change state is also faked, so a call would be recorded.
    for n in ["kill", "pkill", "killall", "holler", "ssh", "curl", "systemctl"] {
        env.fake(n, "exit 0");
    }
}

fn assert_read_only(env: &Env) {
    let calls = env.calls();
    for c in &calls {
        let name = c.split_whitespace().next().unwrap();
        match name {
            "ss" | "lsof" | "ps" => {}
            "tailscale" => assert_eq!(c, "tailscale serve status", "write-capable call: {c}"),
            "herdr" => assert_eq!(c, "herdr session list", "write-capable call: {c}"),
            other => panic!("inventory made a non-read-only call: {other} ({c})"),
        }
    }
    let leftover: Vec<_> = fs::read_dir(env.work()).unwrap().collect();
    assert!(leftover.is_empty(), "inventory wrote files: {leftover:?}");
}

#[test]
fn inventory_lists_ports_processes_serve_and_herdr_sessions_read_only() {
    let env = Env::new();
    install_live_fakes(&env);
    let out = env.script("inventory.sh").output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(s.contains("port\t41807\t111\tholler\n"), "{s}");
    assert!(s.contains("port\t47001\t222\topencode\n"), "{s}");
    assert!(
        s.contains(&format!("hub\t111\t<user>\t-\t41807\t{FIRST_HUB_STARTED}\t{FIRST_HUB_CMD}\n")),
        "{s}"
    );
    assert!(s.contains("\nbody\t333\t<user>\t-\talpha\t"), "{s}");
    assert!(s.contains("\nopencode\t222\t<user>\t-\t47001\t"), "{s}");
    assert!(!s.contains("sshd"), "unrelated process listed: {s}");
    assert!(s.contains("\nserve\t443\t"), "{s}");
    assert!(s.contains("\nserve\t8444\t"), "{s}");
    assert!(s.contains("herdr-session\tfirst\n"), "{s}");
    assert_read_only(&env);
}

#[test]
fn inventory_falls_back_to_lsof_when_ss_is_unusable() {
    let env = Env::new();
    install_live_fakes(&env);
    env.fake("ss", "exit 1");
    env.fake(
        "lsof",
        "echo 'COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME'\n\
         echo 'holler 111 <user> 5u IPv4 0x1 0t0 TCP 127.0.0.1:41807 (LISTEN)'",
    );
    let out = env.script("inventory.sh").output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(s.contains("port\t41807\t111\tholler\n"), "{s}");
    assert_read_only(&env);
}

#[test]
fn inventory_tolerates_missing_tailscale_and_herdr() {
    let env = Env::new();
    install_live_fakes(&env);
    env.fake("tailscale", "exit 1");
    env.fake("herdr", "exit 1");
    let out = env.script("inventory.sh").output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(!s.contains("\nserve\t") && !s.contains("herdr-session"), "{s}");
    assert_read_only(&env);
}

// ------------------------------------------------------------------ collide.sh

#[test]
fn each_colliding_item_is_refused_in_turn_and_named() {
    let t = "\t";
    // (inventory line, text the refusal must contain, config key it must name)
    let cases: Vec<(String, &str, &str)> = vec![
        (format!("port{t}47002{t}-{t}other-service"), "47002", "backend_port_base"),
        (format!("port{t}443{t}-{t}tailscaled"), "443", "serve_https_port"),
        (
            format!("hub{t}111{t}<user>{t}-{t}41807{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}"),
            "hub",
            "hub_port",
        ),
        (
            format!(
                "body{t}333{t}<user>{t}-{t}beta{t}{FIRST_HUB_STARTED}{t}/usr/bin/holler body run",
            ),
            "beta",
            "[[session]] name",
        ),
        (
            format!(
                "hub{t}111{t}<user>{t}/home/<user>/.holler-x{t}41900{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}",
            ),
            "/home/<user>/.holler-x",
            "state_dir",
        ),
    ];
    for (line, needle, key) in cases {
        let env = Env::new();
        let inv = env.file("inv.tsv", &format!("{line}\n"));
        let out = collide(&env, &inv, |c| {
            distinct_plan(c);
            // make each case collide only through its own item
            c.env("WIZARD_HUB_PORT", "41807")
                .env("WIZARD_SERVE_HTTPS_PORT", "443")
                .env("WIZARD_BACKEND_PORTS", "47002")
                .env("WIZARD_SESSION_NAMES", "beta")
                .env("WIZARD_STATE_DIR", "/home/<user>/.holler-x");
        });
        let s = text(&out);
        assert_eq!(out.status.code(), Some(1), "not refused: {line}\n{s}");
        assert!(s.contains("REFUSED"), "{s}");
        assert!(s.contains(needle), "item not named ({needle}): {s}");
        assert!(s.contains(key), "key not named ({key}): {s}");
    }
}

#[test]
fn a_herdr_session_of_the_same_name_is_refused() {
    let env = Env::new();
    let inv = env.file("inv.tsv", "herdr-session\tfirst\n");
    let out = collide(&env, &inv, |c| {
        distinct_plan(c);
        c.env("WIZARD_HERDR_SESSION", "first");
    });
    let s = text(&out);
    assert_eq!(out.status.code(), Some(1), "{s}");
    assert!(s.contains("REFUSED") && s.contains("first"), "{s}");
    assert!(s.contains("herdr_session"), "{s}");
}

#[test]
fn a_default_second_instance_is_refused_for_every_colliding_item() {
    let env = Env::new();
    let inv = env.file("inv.tsv", &first_instance_inventory());
    let out = collide(&env, &inv, |c| {
        default_plan(c);
        c.env("WIZARD_HERDR_SESSION", "first");
    });
    let s = text(&out);
    assert_eq!(out.status.code(), Some(1), "{s}");
    for key in ["hub_port", "backend_port_base", "serve_https_port", "herdr_session"] {
        assert!(s.contains(key), "missing {key}: {s}");
    }
    assert!(s.contains("41807") && s.contains("47001") && s.contains("alpha"), "{s}");
}

#[test]
fn distinct_ports_state_dir_and_herdr_session_are_accepted_and_first_items_listed() {
    let env = Env::new();
    let inv = env.file("inv.tsv", &first_instance_inventory());
    let out = collide(&env, &inv, distinct_plan);
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "{s}");
    assert!(!s.contains("REFUSED"), "{s}");
    for item in ["41807", "47001", "alpha", "first", "443"] {
        let line = s
            .lines()
            .find(|l| l.contains(item) && l.contains("present, not touched"))
            .unwrap_or_else(|| panic!("{item} not listed as present, not touched: {s}"));
        assert!(!line.contains("REFUSED"));
    }
}

#[test]
fn a_ledger_live_process_is_ours_and_a_stale_entry_is_not() {
    let env = Env::new();
    let inv = env.file("inv.tsv", &first_instance_inventory());
    let ledger = |cmd: &str| {
        format!(
            "[[process]]\npid = 111\nstarted = \"{FIRST_HUB_STARTED}\"\ncmd = \"{cmd}\"\nrole = \"hub\"\nstage = 6\nsession = \"\"\n"
        )
    };
    // Only the hub and its port are in this inventory, so ownership decides the verdict.
    let hub_only = env.file(
        "hub.tsv",
        &format!(
            "port\t41807\t111\tholler\nhub\t111\t<user>\t-\t41807\t{FIRST_HUB_STARTED}\t{FIRST_HUB_CMD}\n"
        ),
    );
    let _ = inv;
    let live = env.file("live.toml", &ledger(FIRST_HUB_CMD));
    let out = collide(&env, &hub_only, |c| {
        default_plan(c);
        c.env("WIZARD_BACKEND_PORTS", "")
            .env("WIZARD_SESSION_NAMES", "")
            .env("WIZARD_SERVE_HTTPS_PORT", "")
            .env("WIZARD_LEDGER", &live);
    });
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "live ledger entry refused: {s}");
    assert!(s.contains("created by this instance"), "{s}");

    let stale = env.file("stale.toml", &ledger("/usr/bin/some-other-program"));
    let out = collide(&env, &hub_only, |c| {
        default_plan(c);
        c.env("WIZARD_BACKEND_PORTS", "")
            .env("WIZARD_SESSION_NAMES", "")
            .env("WIZARD_SERVE_HTTPS_PORT", "")
            .env("WIZARD_LEDGER", &stale);
    });
    let s = text(&out);
    assert_eq!(out.status.code(), Some(1), "stale pid treated as ours: {s}");
    assert!(s.contains("hub_port"), "{s}");
}

#[test]
fn collide_itself_makes_no_external_calls() {
    let env = Env::new();
    for n in ["ss", "lsof", "ps", "tailscale", "herdr", "kill", "pkill", "holler"] {
        env.fake(n, "exit 0");
    }
    let inv = env.file("inv.tsv", &first_instance_inventory());
    let _ = collide(&env, &inv, default_plan);
    assert!(env.calls().is_empty(), "{:?}", env.calls());
    assert!(fs::read_dir(env.work()).unwrap().next().is_none());
}
