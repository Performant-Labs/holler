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
            "body{t}333{t}<user>{t}/home/<user>/.holler{t}/x/alpha.toml{t}{FIRST_HUB_STARTED}{t}/usr/bin/holler body run --config /x/alpha.toml",
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
         echo '  333 <user> Sat Oct 10 09:00:02 2026 /usr/bin/holler body run --config /x/alpha.toml'\n\
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
    for n in [
        "kill",
        "pkill",
        "killall",
        "holler",
        "ssh",
        "curl",
        "systemctl",
    ] {
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
    let out = env
        .script("inventory.sh")
        .env("WIZARD_INVENTORY_HERDR", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(s.contains("port\t41807\t111\tholler\n"), "{s}");
    assert!(s.contains("port\t47001\t222\topencode\n"), "{s}");
    assert!(
        s.contains(&format!(
            "hub\t111\t<user>\t-\t41807\t{FIRST_HUB_STARTED}\t{FIRST_HUB_CMD}\n"
        )),
        "{s}"
    );
    assert!(s.contains("\nbody\t333\t<user>\t-\t/x/alpha.toml\t"), "{s}");
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
    assert!(
        !s.contains("\nserve\t") && !s.contains("herdr-session"),
        "{s}"
    );
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
                "body{t}333{t}<user>{t}/home/<user>/.holler-x{t}/x/b.toml{t}{FIRST_HUB_STARTED}{t}/usr/bin/holler body run --config /x/b.toml",
            ),
            "used by a body",
            "state_dir",
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
    for key in [
        "hub_port",
        "backend_port_base",
        "serve_https_port",
        "herdr_session",
    ] {
        assert!(s.contains(key), "missing {key}: {s}");
    }
    assert!(s.contains("41807") && s.contains("47001"), "{s}");
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
    for n in [
        "ss",
        "lsof",
        "ps",
        "tailscale",
        "herdr",
        "kill",
        "pkill",
        "holler",
    ] {
        env.fake(n, "exit 0");
    }
    let inv = env.file("inv.tsv", &first_instance_inventory());
    let _ = collide(&env, &inv, default_plan);
    assert!(env.calls().is_empty(), "{:?}", env.calls());
    assert!(fs::read_dir(env.work()).unwrap().next().is_none());
}

// ------------------------------------------------------------ #750 fixes

const DAY_ONE_STARTED: &str = "Tue Oct  6 22:12:17 2026";

fn quiet_fakes(env: &Env) {
    for n in ["ss", "lsof", "tailscale", "herdr"] {
        env.fake(n, "exit 0");
    }
}

fn hub_ledger(started: &str, cmd: &str) -> String {
    format!(
        "[[process]]\npid = 111\nstarted = \"{started}\"\ncmd = \"{cmd}\"\nrole = \"hub\"\nstage = 6\nsession = \"\"\n"
    )
}

/// A plan of only the hub and the serve of a default instance.
fn hub_and_serve_plan(c: &mut Command) {
    default_plan(c);
    c.env("WIZARD_BACKEND_PORTS", "")
        .env("WIZARD_SESSION_NAMES", "");
}

#[test]
fn a_day_one_lstart_with_a_double_space_is_kept_and_recognised_as_ours() {
    let env = Env::new();
    quiet_fakes(&env);
    env.fake(
        "ps",
        &format!("echo '  111 <user> {DAY_ONE_STARTED} {FIRST_HUB_CMD}'"),
    );
    let out = env.script("inventory.sh").output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let inv = String::from_utf8_lossy(&out.stdout).into_owned();
    let want = format!("hub\t111\t<user>\t-\t41807\t{DAY_ONE_STARTED}\t{FIRST_HUB_CMD}\n");
    assert!(inv.contains(&want), "lstart not exact: {inv:?}");

    let inv_file = env.file("inv.tsv", &inv);
    let ledger = env.file("ledger.toml", &hub_ledger(DAY_ONE_STARTED, FIRST_HUB_CMD));
    let out = collide(&env, &inv_file, |c| {
        hub_and_serve_plan(c);
        c.env("WIZARD_LEDGER", &ledger);
    });
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "own process refused: {s}");
    assert!(s.contains("created by this instance"), "{s}");
}

fn own_serve_inventory(target: &str) -> String {
    let t = "\t";
    format!(
        "hub{t}111{t}<user>{t}-{t}41807{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}\n\
         serve{t}443{t}https://<hub-host>.example.ts.net (tailnet only){t}{target}\n"
    )
}

fn run_serve_case(target: &str, ledger_cmd: &str) -> (Option<i32>, String) {
    let env = Env::new();
    let inv = env.file("inv.tsv", &own_serve_inventory(target));
    let ledger = env.file("ledger.toml", &hub_ledger(FIRST_HUB_STARTED, ledger_cmd));
    let out = collide(&env, &inv, |c| {
        hub_and_serve_plan(c);
        c.env("WIZARD_LEDGER", &ledger);
    });
    (out.status.code(), text(&out))
}

#[test]
fn a_serve_to_our_own_hub_with_a_live_hub_row_is_present_not_touched() {
    let (code, s) = run_serve_case("http://127.0.0.1:41807", FIRST_HUB_CMD);
    assert_eq!(code, Some(0), "own serve refused: {s}");
    assert!(!s.contains("REFUSED"), "{s}");
    let line = s.lines().find(|l| l.contains("tailscale serve")).unwrap();
    assert!(line.contains("present, not touched"), "{s}");
}

#[test]
fn a_serve_without_a_live_hub_row_is_refused_as_before() {
    let (code, s) = run_serve_case("http://127.0.0.1:41807", "/usr/bin/some-other-program");
    assert_eq!(code, Some(1), "{s}");
    assert!(s.contains("serve_https_port"), "{s}");
}

#[test]
fn a_serve_to_another_target_is_refused_even_with_a_live_hub_row() {
    let (code, s) = run_serve_case("http://127.0.0.1:41999", FIRST_HUB_CMD);
    assert_eq!(code, Some(1), "{s}");
    assert!(s.contains("serve_https_port"), "{s}");
}

#[test]
fn inventory_carries_the_serve_target_of_each_serve_entry() {
    let env = Env::new();
    install_live_fakes(&env);
    let out = env.script("inventory.sh").output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    let line = s.lines().find(|l| l.starts_with("serve\t443\t")).unwrap();
    assert!(line.ends_with("\thttp://127.0.0.1:41807"), "{line}");
    let other = s.lines().find(|l| l.starts_with("serve\t8444\t")).unwrap();
    assert!(other.ends_with("\t-"), "{other}");
}

#[test]
fn a_body_row_carries_its_config_path_as_field_five() {
    let env = Env::new();
    quiet_fakes(&env);
    env.fake(
        "ps",
        "echo '  333 <user> Sat Oct 10 09:00:02 2026 /usr/bin/holler body run --config /x/y.toml'",
    );
    let out = env.script("inventory.sh").output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    let line = s.lines().find(|l| l.starts_with("body\t")).unwrap();
    let fields: Vec<&str> = line.split('\t').collect();
    assert_eq!(fields[4], "/x/y.toml", "{line}");
}

/// Links the named real tools into the fake bin directory and makes it the whole `PATH`.
fn only_these_tools(env: &Env, names: &[&str]) -> String {
    let real = std::env::var("PATH").unwrap_or_default();
    for n in names {
        let found = std::env::split_paths(&real)
            .map(|d| d.join(n))
            .find(|p| p.is_file())
            .unwrap_or_else(|| panic!("no {n} on PATH"));
        std::os::unix::fs::symlink(found, env.bin().join(n)).unwrap();
    }
    env.bin().display().to_string()
}

#[test]
fn a_missing_tailscale_gives_a_warn_line_not_silence() {
    let env = Env::new();
    for n in ["ss", "lsof", "herdr", "ps"] {
        env.fake(n, "exit 0");
    }
    let only = only_these_tools(&env, &["bash", "tr", "sed", "head"]);
    let out = env
        .script("inventory.sh")
        .env("PATH", only)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(s.contains("warn\tmissing tool tailscale\n"), "{s}");
    assert!(
        !s.contains("missing tool herdr") && !s.contains("missing tool ps"),
        "{s}"
    );
}

#[test]
fn collide_passes_an_inventory_warning_on() {
    let env = Env::new();
    let inv = env.file("inv.tsv", "warn\tmissing tool tailscale\n");
    let out = collide(&env, &inv, distinct_plan);
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "{s}");
    assert!(s.contains("missing tool tailscale"), "{s}");
}

#[test]
fn both_scripts_are_executable_in_git() {
    let root = lib_dir();
    let out = Command::new("git")
        .current_dir(&root)
        .args(["ls-files", "-s", "inventory.sh", "collide.sh"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(s.lines().count(), 2, "{s}");
    assert!(s.lines().all(|l| l.starts_with("100755 ")), "{s}");
    for n in ["inventory.sh", "collide.sh"] {
        let mode = fs::metadata(root.join(n)).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0, "{n} is not executable");
    }
}

fn run_default_dir_case(env: &Env, row: &str, plan_dir: &str) -> Output {
    let inv = env.file("inv.tsv", &format!("{row}\n"));
    let home = env.dir.path().join("home");
    collide(env, &inv, |c| {
        distinct_plan(c);
        c.env("HOME", &home).env(
            "WIZARD_STATE_DIR",
            plan_dir.replace("<home>", home.to_str().unwrap()),
        );
    })
}

#[test]
fn a_dash_state_dir_on_a_hub_or_body_row_is_the_default_state_directory() {
    let t = "\t";
    let hub = format!("hub{t}111{t}<user>{t}-{t}41807{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}");
    let body = format!(
        "body{t}333{t}<user>{t}-{t}/x/y.toml{t}{FIRST_HUB_STARTED}{t}/usr/bin/holler body run --config /x/y.toml"
    );
    for (row, dir) in [
        (&hub, "<home>/.holler"),
        (&body, "<home>/.holler"),
        (&hub, "~/.holler"),
    ] {
        let env = Env::new();
        let out = run_default_dir_case(&env, row, dir);
        let s = text(&out);
        assert_eq!(out.status.code(), Some(1), "{dir}: {s}");
        assert!(s.contains("state_dir") && s.contains("REFUSED"), "{s}");
    }
}

#[test]
fn a_dash_state_dir_does_not_collide_with_a_distinct_plan_directory() {
    let t = "\t";
    let hub = format!("hub{t}111{t}<user>{t}-{t}41807{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}");
    let env = Env::new();
    let out = run_default_dir_case(&env, &hub, "<home>/.holler-second");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
}

// ------------------------------------------------------------ #758 fixes

#[test]
fn inventory_first_line_is_the_hosts_home() {
    let env = Env::new();
    install_live_fakes(&env);
    let out = env
        .script("inventory.sh")
        .env("HOME", "/home/<remote-user>")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(s.lines().next(), Some("home\t/home/<remote-user>"), "{s}");
}

#[test]
fn inventory_has_no_herdr_section_or_warning_unless_asked() {
    let env = Env::new();
    install_live_fakes(&env);
    let out = env.script("inventory.sh").output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        !s.contains("herdr-session") && !s.contains("missing tool herdr"),
        "{s}"
    );
    assert!(
        !env.calls().iter().any(|c| c.starts_with("herdr")),
        "herdr was called: {:?}",
        env.calls()
    );
}

#[test]
fn a_missing_herdr_still_warns_when_the_herdr_section_is_asked_for() {
    let env = Env::new();
    for n in ["ss", "lsof", "tailscale", "ps"] {
        env.fake(n, "exit 0");
    }
    let only = only_these_tools(&env, &["bash", "tr", "sed", "head"]);
    let out = env
        .script("inventory.sh")
        .env("PATH", only)
        .env("WIZARD_INVENTORY_HERDR", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(s.contains("warn\tmissing tool herdr\n"), "{s}");
}

#[test]
fn herdr_bin_names_the_binary_for_the_herdr_section() {
    let env = Env::new();
    install_live_fakes(&env);
    let elsewhere = env.dir.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    let bin = elsewhere.join("my-herdr");
    fs::write(
        &bin,
        format!(
            "#!/bin/bash\necho \"my-herdr $*\" >> \"{}\"\necho viaenv\n",
            env.log().display()
        ),
    )
    .unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_file(env.bin().join("herdr")).unwrap();
    let out = env
        .script("inventory.sh")
        .env("WIZARD_INVENTORY_HERDR", "1")
        .env("HERDR_BIN", &bin)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(s.contains("herdr-session\tviaenv\n"), "{s}");
    assert!(!s.contains("missing tool herdr"), "{s}");
    assert!(env.calls().contains(&"my-herdr session list".to_owned()));
}

#[test]
fn a_dash_state_dir_is_compared_against_the_inventorys_home_not_the_local_one() {
    let t = "\t";
    let hub = format!("hub{t}111{t}<user>{t}-{t}41807{t}{FIRST_HUB_STARTED}{t}{FIRST_HUB_CMD}");
    let env = Env::new();
    let inv = env.file("inv.tsv", &format!("home{t}/home/<remote-user>\n{hub}\n"));
    let local_home = env.dir.path().join("home");
    let run = |plan_dir: &str| {
        collide(&env, &inv, |c| {
            distinct_plan(c);
            c.env("HOME", &local_home)
                .env("WIZARD_STATE_DIR", plan_dir)
                .env("WIZARD_HUB_PORT", "41808");
        })
    };
    let out = run("/home/<remote-user>/.holler");
    let s = text(&out);
    assert_eq!(out.status.code(), Some(1), "{s}");
    assert!(s.contains("state_dir") && s.contains("REFUSED"), "{s}");
    let out = run("~/.holler");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    // the local home's default directory is not that host's default directory
    let out = run(&format!("{}/.holler", local_home.display()));
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
}

fn serve_only_inventory(extra: &str) -> String {
    let t = "\t";
    format!(
        "serve{t}443{t}https://<hub-host>.example.ts.net (tailnet only){t}http://127.0.0.1:41807\n{extra}"
    )
}

#[test]
fn an_own_serve_pair_with_no_live_hub_is_reported_not_refused() {
    let env = Env::new();
    let inv = env.file("inv.tsv", &serve_only_inventory(""));
    let out = collide(&env, &inv, hub_and_serve_plan);
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "{s}");
    assert!(!s.contains("REFUSED"), "{s}");
    assert!(s.contains("this instance's port pair, no live hub"), "{s}");
}

#[test]
fn an_own_serve_pair_is_still_refused_when_a_foreign_process_holds_the_hub_port() {
    let t = "\t";
    for extra in [
        format!("port{t}41807{t}999{t}other-service\n"),
        format!("port{t}41807{t}-{t}-\n"),
        format!(
            "hub{t}999{t}<user>{t}-{t}41807{t}{FIRST_HUB_STARTED}{t}/usr/bin/holler hub --listen 127.0.0.1:41807\n"
        ),
    ] {
        let env = Env::new();
        let inv = env.file("inv.tsv", &serve_only_inventory(&extra));
        let out = collide(&env, &inv, hub_and_serve_plan);
        let s = text(&out);
        assert_eq!(out.status.code(), Some(1), "{extra}: {s}");
        assert!(s.contains("serve_https_port"), "{s}");
        assert!(!s.contains("no live hub"), "{s}");
    }
}

fn unnamed_herdr_inventory() -> String {
    let t = "\t";
    format!("herdr{t}555{t}<user>{t}-{t}-{t}{FIRST_HUB_STARTED}{t}/usr/bin/herdr server\n")
}

fn herdr_plan(c: &mut Command, name: &str) {
    if name == "default" {
        hub_and_serve_plan(c);
    } else {
        distinct_plan(c);
    }
    c.env("WIZARD_INSTANCE_NAME", name);
}

#[test]
fn an_unnamed_herdr_server_without_a_ledger_row_is_foreign_for_the_default_instance_only() {
    let env = Env::new();
    let inv = env.file("inv.tsv", &unnamed_herdr_inventory());
    let out = collide(&env, &inv, |c| herdr_plan(c, "default"));
    let s = text(&out);
    assert_eq!(out.status.code(), Some(1), "{s}");
    assert!(s.contains("REFUSED") && s.contains("unnamed herdr"), "{s}");
    assert!(
        s.contains("herdr_session") && s.contains("build in it"),
        "{s}"
    );
    assert!(s.contains("present, not touched: herdr"), "{s}");

    let out = collide(&env, &inv, |c| herdr_plan(c, "second"));
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "{s}");
    assert!(!s.contains("REFUSED"), "{s}");
}

#[test]
fn an_unnamed_herdr_server_with_a_live_ledger_row_is_ours() {
    let env = Env::new();
    let inv = env.file("inv.tsv", &unnamed_herdr_inventory());
    let ledger = env.file(
        "ledger.toml",
        &format!(
            "[[process]]\npid = 555\nstarted = \"{FIRST_HUB_STARTED}\"\ncmd = \"/usr/bin/herdr server\"\nrole = \"herdr\"\nstage = 8\nsession = \"\"\n"
        ),
    );
    let out = collide(&env, &inv, |c| {
        herdr_plan(c, "default");
        c.env("WIZARD_LEDGER", &ledger);
    });
    let s = text(&out);
    assert_eq!(out.status.code(), Some(0), "{s}");
    assert!(s.contains("created by this instance"), "{s}");
}
