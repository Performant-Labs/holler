//! A scratch Herdr server for the opt-in tests (#640 part 3): the first Rust code that
//! starts a real Herdr. It ports the spike's isolation (`scripts/spikes/herdr-lib.sh`)
//! and depends on nothing else of this crate's tests, so a later story can move it.
//!
//! Three guards keep the operator's Herdr out of reach, each a pure function that the
//! default run tests:
//!
//! - **The environment** ([`scratch_env`]): no `HERDR_*`, `TMUX` or `TMUX_PANE`, and
//!   `HOME` and every `XDG_*` directory inside a fresh root, so even Herdr's default
//!   session resolves to a socket inside the root.
//! - **The name** ([`check_name`]): every call carries `--session holler640-<8 hex>`,
//!   and `default` is refused by name.
//! - **The proof** ([`prove`]): the server must report that session and a socket inside
//!   the root before anything connects to it.
//!
//! The guard ([`ScratchHerdr`]) stops only the child it started, then removes the root.
//! Every `herdr` the harness runs goes through [`command`], and every wait through
//! [`poll`]: checked first, then paused, and bounded.

use std::collections::hash_map::RandomState;
use std::ffi::OsString;
use std::fs::{self, File};
use std::hash::{BuildHasher, Hasher};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use tempfile::TempDir;

/// The variable that opts in; only the exact value "1" runs a scratch server.
pub const GATE_VAR: &str = "HOLLER_HERDR_SCRATCH";
/// Every scratch session's name is this prefix and 8 lower-case hex digits.
pub const NAME_PREFIX: &str = "holler640-";
/// Every scratch root's directory-name prefix.
pub const ROOT_PREFIX: &str = "h640.";
/// The longest socket path accepted, in bytes (macOS's sockaddr_un holds 104).
pub const SOCKET_PATH_LIMIT: usize = 100;

/// How long a short `herdr` command (`status server --json`, `server stop`) may run.
const COMMAND_LIMIT: Duration = Duration::from_secs(5);
/// How often a short command is checked for its exit.
const COMMAND_POLL: Duration = Duration::from_millis(50);
/// How long a started server has to prove itself, and how often it is asked.
const START_LIMIT: Duration = Duration::from_secs(15);
const START_POLL: Duration = Duration::from_millis(100);
/// How long a stopped server has to exit before it is killed.
const STOP_LIMIT: Duration = Duration::from_secs(10);
/// What Herdr puts below the scratch base: the root, then the session's socket.
const SOCKET_TAIL: &str = "/h640.XXXXXX/home/.config/herdr/sessions/holler640-00000000/herdr.sock";
/// The longest scratch base used as it is, so that the socket stays under
/// [`SOCKET_PATH_LIMIT`] (29 bytes). A longer one (macOS's `TMPDIR`) gives way to `/tmp`.
/// The spike's 40 is too long for this limit: a 31-byte base made a 104-byte socket.
const BASE_LIMIT: usize = SOCKET_PATH_LIMIT - 1 - SOCKET_TAIL.len();
/// The scratch server's config: no onboarding, `/bin/sh` panes, no network checks.
const CONFIG_TOML: &str = "onboarding = false\n[terminal]\ndefault_shell = \"/bin/sh\"\n\
                           [update]\nversion_check = false\nmanifest_check = false\n";
/// The variables `scratch_env` points into the root, each with its path under it.
const ROOTED: [(&str, &str); 6] = [
    ("HOME", "home"),
    ("XDG_CONFIG_HOME", "home/.config"),
    ("XDG_STATE_HOME", "home/.local/state"),
    ("XDG_DATA_HOME", "home/.local/share"),
    ("XDG_CACHE_HOME", "home/.cache"),
    ("XDG_RUNTIME_DIR", "run"),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Gate {
    Run,
    Skip,
}

/// Pure: `Run` exactly when `value` is `Some("1")`.
pub fn gate(value: Option<&str>) -> Gate {
    if value == Some("1") {
        Gate::Run
    } else {
        Gate::Skip
    }
}

/// Pure: `Ok` only for `NAME_PREFIX` + 8 of `[0-9a-f]`; `default` is refused by name.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.eq_ignore_ascii_case("default") {
        return Err(format!(
            "refusing the Herdr session {name:?}: it is Herdr's default session, never a \
             scratch one"
        ));
    }
    let hex = name.strip_prefix(NAME_PREFIX).unwrap_or("");
    let is_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    if name.starts_with(NAME_PREFIX) && hex.len() == 8 && hex.chars().all(is_hex) {
        return Ok(());
    }
    Err(format!(
        "refusing the Herdr session {name:?}: a scratch session is {NAME_PREFIX} and 8 \
         lower-case hex digits"
    ))
}

/// A fresh name that passes `check_name` (std only, for example `RandomState`).
pub fn new_name() -> String {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(CALLS.fetch_add(1, Ordering::Relaxed));
    hasher.write_u32(std::process::id());
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    hasher.write_u128(nanos);
    format!("{NAME_PREFIX}{:08x}", hasher.finish() as u32)
}

/// Pure: `Ok` only when `socket` is absolute, has only normal components, lies under
/// `root` (by components) and is shorter than `SOCKET_PATH_LIMIT` bytes.
pub fn check_socket(root: &Path, socket: &Path) -> Result<(), String> {
    let refuse = |why: &str| Err(format!("refusing the socket {socket:?}: {why}"));
    let Some(text) = socket.to_str() else {
        return refuse("it is not UTF-8");
    };
    // `Path::components` drops a `.` in the middle, so the text is read segment by segment.
    let Some(rest) = text.strip_prefix('/') else {
        return refuse("it is not absolute");
    };
    if rest
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return refuse("it has a component that is not a plain name");
    }
    if !root.is_absolute() || root.components().any(|c| matches!(c, Component::ParentDir)) {
        return refuse("the scratch root is not a plain absolute path");
    }
    if !socket.starts_with(root) || socket.components().count() <= root.components().count() {
        return refuse("it is not inside the scratch root");
    }
    if text.len() >= SOCKET_PATH_LIMIT {
        return refuse(&format!(
            "it is {} bytes, the limit is {SOCKET_PATH_LIMIT}",
            text.len()
        ));
    }
    Ok(())
}

/// Pure: read `herdr status server --json`'s output; `Ok(socket)` only when its
/// `session` is `name` and its `socket` passes `check_socket(root, ..)`.
pub fn prove(status_json: &str, name: &str, root: &Path) -> Result<PathBuf, String> {
    check_name(name)?;
    let status: serde_json::Value = serde_json::from_str(status_json)
        .map_err(|_| "refusing the server: its status is not JSON".to_owned())?;
    let session = status.get("session").and_then(serde_json::Value::as_str);
    if session != Some(name) {
        return Err(format!(
            "refusing the server: it reports the session {session:?}, not {name:?}"
        ));
    }
    let Some(socket) = status.get("socket").and_then(serde_json::Value::as_str) else {
        return Err("refusing the server: its status names no socket".to_owned());
    };
    let socket = PathBuf::from(socket);
    check_socket(root, &socket)?;
    Ok(socket)
}

/// Pure: the whole environment of every `herdr` the harness runs, built from the
/// inherited one: no `HERDR_*`, `TMUX` or `TMUX_PANE`; `HOME`, `XDG_CONFIG_HOME`,
/// `XDG_STATE_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_RUNTIME_DIR` inside
/// `root`; `SHELL=/bin/sh`; everything else as inherited.
pub fn scratch_env(
    root: &Path,
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, OsString)> {
    let replaced = |name: &OsString| {
        let bytes = name.as_encoded_bytes();
        bytes.starts_with(b"HERDR_")
            || name == "TMUX"
            || name == "TMUX_PANE"
            || name == "SHELL"
            || ROOTED.iter().any(|(rooted, _)| name == rooted)
    };
    let mut env: Vec<(OsString, OsString)> = inherited
        .into_iter()
        .filter(|(name, _)| !replaced(name))
        .collect();
    for (name, under) in ROOTED {
        env.push((name.into(), root.join(under).into_os_string()));
    }
    env.push(("SHELL".into(), "/bin/sh".into()));
    env
}

/// The opt-in, first statement of each ignored test: `None` (after printing one line
/// starting `skipped (HOLLER_HERDR_SCRATCH is not 1)`) unless the gate is `Run`; then
/// the absolute path of `herdr` on `PATH`, or a panic naming `herdr` and `PATH`.
pub fn opt_in() -> Option<PathBuf> {
    let value = std::env::var_os(GATE_VAR);
    if gate(value.as_deref().and_then(|v| v.to_str())) == Gate::Skip {
        eprintln!("skipped (HOLLER_HERDR_SCRATCH is not 1): no scratch Herdr server was started");
        return None;
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let found = std::env::split_paths(&path)
        .map(|dir| dir.join("herdr"))
        .find(|candidate| is_executable(candidate));
    let Some(herdr) = found else {
        panic!("{GATE_VAR}=1 asks for a scratch Herdr, but no executable `herdr` is on PATH");
    };
    Some(fs::canonicalize(&herdr).unwrap_or_else(|error| {
        panic!("{GATE_VAR}=1: the `herdr` found on PATH cannot be resolved ({error})")
    }))
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// Run `probe` until it returns `Some`, at most `limit`: checked first, then paused for
/// the lesser of `every` and the time left. `None` when the time ran out.
pub fn poll<T>(
    limit: Duration,
    every: Duration,
    mut probe: impl FnMut() -> Option<T>,
) -> Option<T> {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(found) = probe() {
            return Some(found);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        thread::sleep(every.min(left));
    }
}

/// The one way the harness runs `herdr` (AC 8): `herdr --session <name> <args>`, the
/// name checked, the environment exactly `scratch_env(root, ..)`, in `root/work`, with
/// no stdin.
fn command(herdr: &Path, root: &Path, name: &str, args: &[&str]) -> Result<Command, String> {
    check_name(name)?;
    let mut command = Command::new(herdr);
    command
        .arg("--session")
        .arg(name)
        .args(args)
        .env_clear()
        .envs(scratch_env(root, std::env::vars_os()))
        .current_dir(root.join("work"))
        .stdin(Stdio::null());
    Ok(command)
}

/// Run a short `command` to its end within [`COMMAND_LIMIT`]: its exit status and its
/// stdout. Stdout is drained on a thread so a full pipe cannot wedge the child, and a
/// child still running at the limit is killed and reaped.
fn run_bounded(mut command: Command) -> Result<(ExitStatus, String), String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("herdr did not start ({error})"))?;
    let mut stdout = child.stdout.take();
    let reader = thread::spawn(move || {
        let mut text = String::new();
        if let Some(stdout) = stdout.as_mut() {
            let _ = stdout.read_to_string(&mut text);
        }
        text
    });
    let Some(status) = poll(COMMAND_LIMIT, COMMAND_POLL, || {
        child.try_wait().ok().flatten()
    }) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("herdr ran past {COMMAND_LIMIT:?} and was killed"));
    };
    let text = reader.join().unwrap_or_default();
    Ok((status, text))
}

/// The scratch base: the temp directory when its canonical path is short, else `/tmp`.
fn scratch_base() -> PathBuf {
    let temp = fs::canonicalize(std::env::temp_dir()).expect("the temp directory");
    if temp.as_os_str().len() <= BASE_LIMIT {
        temp
    } else {
        fs::canonicalize("/tmp").expect("/tmp")
    }
}

/// A fresh root `h640.XXXXXX` in the scratch base, laid out as Decision 2 says.
fn make_root() -> (TempDir, PathBuf) {
    let dir = tempfile::Builder::new()
        .prefix(ROOT_PREFIX)
        .tempdir_in(scratch_base())
        .expect("a scratch root");
    let root = fs::canonicalize(dir.path()).expect("the scratch root, canonical");
    for (_, under) in ROOTED {
        fs::create_dir_all(root.join(under)).expect("a scratch directory");
    }
    fs::create_dir_all(root.join("work")).expect("the scratch work directory");
    fs::set_permissions(root.join("run"), fs::Permissions::from_mode(0o700)).expect("run/ 0700");
    fs::create_dir_all(root.join("home/.config/herdr")).expect("the scratch config directory");
    fs::write(root.join("home/.config/herdr/config.toml"), CONFIG_TOML).expect("config.toml");
    (dir, root)
}

/// One scratch Herdr server: its root, its session, its proven socket, its child process.
pub struct ScratchHerdr {
    herdr: PathBuf,
    root: PathBuf,
    name: String,
    socket: PathBuf,
    child: Child,
    /// Dropped last: removes the root.
    _dir: TempDir,
}

impl ScratchHerdr {
    /// Start a server, prove it (Decision 4), and return once its socket is proven.
    /// Any refusal or failure panics with the reason.
    pub fn start(herdr: &Path) -> Self {
        let (dir, root) = make_root();
        let name = new_name();
        let log = File::create(root.join("server.log")).expect("server.log");
        let child = command(herdr, &root, &name, &["server"])
            .unwrap_or_else(|why| panic!("{why}"))
            .stdout(log.try_clone().expect("server.log, twice"))
            .stderr(log)
            .spawn()
            .unwrap_or_else(|error| panic!("the scratch Herdr server did not start ({error})"));
        // The guard exists before the proof, so a refusal still stops the server.
        let mut server = Self {
            herdr: herdr.to_owned(),
            root,
            name,
            socket: PathBuf::new(),
            child,
            _dir: dir,
        };
        server.socket = server.proven_socket();
        server
    }

    /// Ask the server for its status until it proves itself: its socket. Panics on a
    /// refusal, an exited server or [`START_LIMIT`].
    fn proven_socket(&mut self) -> PathBuf {
        let proven = poll(START_LIMIT, START_POLL, || {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(Err(format!("the scratch Herdr server exited ({status})")));
            }
            let status = command(
                &self.herdr,
                &self.root,
                &self.name,
                &["status", "server", "--json"],
            )
            .and_then(run_bounded);
            match status {
                Ok((exit, json)) if exit.success() => match prove(&json, &self.name, &self.root) {
                    Ok(socket) if socket.exists() => Some(Ok(socket)),
                    Ok(_) => None,
                    Err(refusal) => Some(Err(refusal)),
                },
                _ => None,
            }
        });
        let socket = match proven {
            Some(Ok(socket)) => socket,
            Some(Err(why)) => panic!("{why}"),
            None => panic!("the scratch Herdr server did not prove itself in {START_LIMIT:?}"),
        };
        let canonical = fs::canonicalize(&socket).expect("the proven socket, canonical");
        assert_eq!(
            canonical, socket,
            "the proven socket is not its canonical path"
        );
        socket
    }

    pub fn session(&self) -> &str {
        &self.name
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

/// Stops only this server, then removes its root (Decision 5). Never panics.
impl Drop for ScratchHerdr {
    fn drop(&mut self) {
        let stopped =
            command(&self.herdr, &self.root, &self.name, &["server", "stop"]).and_then(run_bounded);
        if let Err(why) = stopped {
            eprintln!("scratch Herdr: `server stop` failed: {why}");
        }
        let exited = poll(STOP_LIMIT, START_POLL, || {
            self.child.try_wait().ok().flatten()
        });
        if exited.is_none() {
            eprintln!("scratch Herdr: the server did not stop in {STOP_LIMIT:?}; killing it");
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
