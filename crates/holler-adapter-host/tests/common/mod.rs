//! The fake `tmux` and the fake `kill` the default-run tests drive `TmuxHost` against
//! (#641, AC 6 and Decision 14), and the one helper that builds every fake-side host
//! (W-20). Each test uses a few of the answer builders, so the rest would trip
//! `dead_code`.
#![allow(dead_code)] // #641
//!
//! Both fakes are `/bin/sh` scripts in one private directory `D`. Every call appends
//! its arguments to a record, one argument per line, then the line `-=END=-`; the fake
//! `kill` also appends its arguments joined by spaces to `kill.flat` and its
//! `${LC_ALL-unset}` to `kill.env`. A fake answers with a shell snippet the test
//! supplies (the "body"), in one of three modes (W-19):
//!
//! - one answer for every call ([`answer`]);
//! - call by call from a numbered queue ([`queue`]: answer N for the N-th call, the last
//!   answer repeating);
//! - by its arguments and by the kill record: a body may test `"$*"` and call
//!   `killed '<flat kill line>'`, which is true once `kill.flat` holds that line. A
//!   listing that shows a pane live until its `KILL` is recorded is one line, so no
//!   test sizes a queue to a number of polls that depends on timing.
//!
//! No fake ever sends a signal: a canned listing names whatever process group holds
//! that number on the machine running the tests (Decision 14).

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use holler_adapter_host::{TmuxHost, TmuxSocket};
use holler_pane::{Argv, PaneName};
use tempfile::TempDir;

/// The line that ends one call's arguments in a record.
const END: &str = "-=END=-";

/// The argument a fake answers at once without recording, used to prove the script
/// can be executed before a test hands it to the adapter.
const WARMUP: &str = "--hlr-641-warmup";

/// The fake `kill`'s default body: `-s 0` reports the procps-ng 4 text (Evidence round
/// 2, P7) for its group, every other signal succeeds.
pub const KILL_GONE: &str = r#"case "$*" in
'-s 0 -- '*) gone "$4" ;;
*) exit 0 ;;
esac"#;

/// The grace every fake-side host stops with (AC 6h).
pub const GRACE: Duration = Duration::from_millis(200);

/// The bound every fake-side host has, unless a test sets its own: long enough for
/// every stop, short enough that a wrong adapter fails rather than hangs.
pub const BOUND: Duration = Duration::from_secs(5);

/// A fake `tmux` and a fake `kill` in a private directory.
pub struct Fake {
    /// Owns the directory, unless the fake was reopened by [`Fake::at`].
    _guard: Option<TempDir>,
    pub dir: PathBuf,
}

impl Fake {
    /// A fake `tmux` answering with `tmux_body` and a fake `kill` answering with
    /// [`KILL_GONE`].
    pub fn new(tmux_body: &str) -> Fake {
        Fake::with_kill(tmux_body, KILL_GONE)
    }

    /// A fake `tmux` answering with `tmux_body` and a fake `kill` answering with
    /// `kill_body`.
    pub fn with_kill(tmux_body: &str, kill_body: &str) -> Fake {
        let guard = tempfile::Builder::new()
            .prefix("hlr-fake-tmux-")
            .tempdir()
            .unwrap();
        let dir = guard.path().to_path_buf();
        write_script(&dir.join("tmux"), &tmux_script(&dir, tmux_body));
        write_script(&dir.join("kill"), &kill_script(&dir, kill_body));
        Fake {
            _guard: Some(guard),
            dir,
        }
    }

    /// The fake another process made in `dir` (the re-executed child of AC 6f).
    pub fn at(dir: PathBuf) -> Fake {
        Fake { _guard: None, dir }
    }

    pub fn tmux(&self) -> PathBuf {
        self.dir.join("tmux")
    }

    pub fn kill(&self) -> PathBuf {
        self.dir.join("kill")
    }

    /// The socket path the fake-side hosts name. No server ever listens there.
    pub fn sock(&self) -> PathBuf {
        self.dir.join("s")
    }

    /// The socket path as a string, as the fake records it.
    pub fn sock_str(&self) -> String {
        self.sock().to_string_lossy().into_owned()
    }

    /// Every tmux call, in order, each as its separate arguments.
    pub fn calls(&self) -> Vec<Vec<String>> {
        read_record(&self.dir.join("tmux.calls"))
    }

    /// Every kill call, in order, its arguments joined by spaces.
    pub fn kills(&self) -> Vec<String> {
        read_lines(&self.dir.join("kill.flat"))
    }

    /// The `LC_ALL` of every kill call (`unset` when it had none).
    pub fn kill_env(&self) -> Vec<String> {
        read_lines(&self.dir.join("kill.env"))
    }

    /// The path of a file in the fake's directory.
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// The fake's directory as a string: an existing absolute directory, which the fake
    /// tmux gives as the session's directory (Decision 3's read).
    pub fn dir_str(&self) -> String {
        self.dir.to_string_lossy().into_owned()
    }
}

/// The one place a fake-side test builds a `TmuxHost` (W-20): always on both fakes, so
/// no test can reach a real tmux server or send a real signal. `socket` is the only
/// choice a test makes; every other setting is a builder call on the result.
pub fn host_on(fake: &Fake, socket: TmuxSocket) -> TmuxHost {
    TmuxHost::new(socket)
        .with_tmux_binary(fake.tmux())
        .with_kill_binary(fake.kill())
        .with_timeout(BOUND)
        .with_stop_grace(GRACE)
}

/// A host on the fake's private socket path.
pub fn host(fake: &Fake) -> TmuxHost {
    host_on(fake, TmuxSocket::Path(fake.sock()))
}

/// `text` quoted for `/bin/sh`.
pub fn sq(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// A body that prints `out` and `err` and exits with `code`.
pub fn answer(out: &str, err: &str, code: i32) -> String {
    format!(
        "printf '%s' {}\nprintf '%s' {} >&2\nexit {code}",
        sq(out),
        sq(err)
    )
}

/// A body that answers the N-th call with the snippet `answers[N - 1]`, the last one
/// repeating.
pub fn queue(answers: &[String]) -> String {
    let mut body = String::from("case \"$n\" in\n");
    for (i, snippet) in answers.iter().enumerate() {
        let label = if i + 1 == answers.len() {
            "*".to_owned()
        } else {
            (i + 1).to_string()
        };
        body.push_str(&format!("{label})\n{snippet}\n;;\n"));
    }
    body.push_str("esac");
    body
}

/// A snippet that prints the fake's own directory, an existing absolute path, as the
/// session's directory (Decision 3's read), and exits 0.
pub const SAY_DIR: &str = "printf '%s\\n' \"$D\"; exit 0";

/// The answers of a successful `run`: the read, `new-window` (`4242 @7`), the tag.
pub fn run_ok() -> String {
    queue(&[
        SAY_DIR.to_owned(),
        answer("4242 @7\n", "", 0),
        answer("", "", 0),
    ])
}

/// A pane name the tests use.
pub fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

/// An argv, element by element.
pub fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|&p| p.to_owned()).collect())
}

/// The tmux subcommand of a recorded call: its first argument after the global flags.
pub fn subcommand(call: &[String]) -> &str {
    let mut i = 0;
    while i < call.len() && matches!(call[i].as_str(), "-S" | "-L" | "-f") {
        i += 2;
    }
    call.get(i).map_or("", String::as_str)
}

/// The value after the first `flag` in a recorded call, if any.
pub fn flag_value<'a>(call: &'a [String], flag: &str) -> Option<&'a str> {
    let at = call.iter().position(|a| a == flag)?;
    call.get(at + 1).map(String::as_str)
}

/// Whether `pid` still exists, asked with signal 0 through the `kill` binary (never
/// `/proc`, which the macOS CI leg lacks; W-19). Only ever asked of a process the test
/// itself started.
pub fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-s", "0", "--", &pid.to_string()])
        .env("LC_ALL", "C")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// `strings` as owned arguments, to compare with a recorded call.
pub fn args(strings: &[&str]) -> Vec<String> {
    strings.iter().map(|&s| s.to_owned()).collect()
}

fn header(dir: &Path) -> String {
    format!(
        "#!/bin/sh\n\
         [ \"$1\" = {WARMUP} ] && exit 0\n\
         D={}\n\
         killed() {{ grep -qxF -e \"$1\" \"$D/kill.flat\" 2>/dev/null; }}\n",
        sq(&dir.to_string_lossy())
    )
}

fn record(file: &str) -> String {
    format!(
        "{{ for a in \"$@\"; do printf '%s\\n' \"$a\"; done; printf '%s\\n' {END}; }} >> \"$D/{file}\"\n"
    )
}

fn tmux_script(dir: &Path, body: &str) -> String {
    format!(
        "{}{}n=$(grep -c '^{END}$' \"$D/tmux.calls\")\n{body}\n",
        header(dir),
        record("tmux.calls")
    )
}

fn kill_script(dir: &Path, body: &str) -> String {
    format!(
        "{}{}printf '%s\\n' \"$*\" >> \"$D/kill.flat\"\n\
         printf '%s\\n' \"${{LC_ALL-unset}}\" >> \"$D/kill.env\"\n\
         n=$(grep -c . \"$D/kill.flat\")\n\
         gone() {{ printf '/usr/bin/kill: (%s): No such process\\n' \"$1\" >&2; exit 1; }}\n\
         {body}\n",
        header(dir),
        record("kill.calls")
    )
}

/// Write an executable script, then run it once (unrecorded) until it executes: a
/// script just written can fail with `ETXTBSY` while a parallel test's fork still holds
/// the write descriptor, and the adapter must never meet that.
fn write_script(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..200 {
        match Command::new(path).arg(WARMUP).status() {
            Ok(status) => {
                assert!(status.success(), "{} warm-up: {status}", path.display());
                return;
            }
            Err(e) if e.kind() == ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("{} warm-up: {e}", path.display()),
        }
    }
    panic!("{} stayed busy", path.display());
}

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn read_record(path: &Path) -> Vec<Vec<String>> {
    let mut calls = Vec::new();
    let mut call = Vec::new();
    for line in read_lines(path) {
        if line == END {
            calls.push(std::mem::take(&mut call));
        } else {
            call.push(line);
        }
    }
    calls
}
