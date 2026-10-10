//! The committed fake tmux (`tests/fixtures/fake-tmux`) and its data files, for
//! `attach_test.rs` (#642b, AC 28-30, 32-33). The fixture is a POSIX `sh` script under git
//! mode 100755; a test writes only the data files it reads, never a script, so no `exec`
//! can meet a file still open for writing ("Text file busy").
//!
//! The calls record (the brief's B-6): one line per call, each argument after `-S <path>`
//! followed by one U+001F, then `\n`. [`Fake::calls`] reads it back as token lists.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tempfile::TempDir;

/// The committed fake tmux.
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-tmux");

/// The fixture's path, after checking it can be run (AC 33).
pub fn fixture() -> PathBuf {
    let path = PathBuf::from(FIXTURE);
    let mode = std::fs::metadata(&path)
        .unwrap_or_else(|e| panic!("{FIXTURE} is missing: {e}"))
        .permissions()
        .mode();
    assert!(
        mode & 0o111 != 0,
        "{FIXTURE} is not executable (mode {mode:o}); restore its git mode with \
         `git update-index --chmod=+x crates/holler-adapter-opencode/tests/fixtures/fake-tmux` \
         and check it out again"
    );
    path
}

/// The fake tmux's data files: `<sock>.calls`, and per subcommand `.out`, `.err`, `.code`.
pub struct Fake {
    dir: TempDir,
}

impl Fake {
    pub fn new() -> Self {
        Fake {
            dir: tempfile::Builder::new()
                .prefix("hlr642t-")
                .tempdir()
                .unwrap(),
        }
    }

    pub fn sock(&self) -> PathBuf {
        self.dir.path().join("sock")
    }

    fn file(&self, sub: &str, ext: &str) -> PathBuf {
        self.dir.path().join(format!("sock.{sub}.{ext}"))
    }

    /// `sub` prints `stdout` and exits 0.
    pub fn reply(&self, sub: &str, stdout: &str) {
        std::fs::write(self.file(sub, "out"), stdout).unwrap();
    }

    /// `sub` prints `stderr` and exits `code`.
    pub fn fail(&self, sub: &str, code: i32, stderr: &str) {
        std::fs::write(self.file(sub, "err"), stderr).unwrap();
        std::fs::write(self.file(sub, "code"), code.to_string()).unwrap();
    }

    /// The recorded calls as token lists (B-6): `None` when tmux was never called.
    pub fn calls(&self) -> Option<Vec<Vec<String>>> {
        let text = std::fs::read_to_string(self.dir.path().join("sock.calls")).ok()?;
        Some(
            text.lines()
                .map(|line| {
                    let mut tokens: Vec<String> = line.split('\u{1f}').map(str::to_owned).collect();
                    assert_eq!(
                        tokens.pop().as_deref(),
                        Some(""),
                        "every argument ends in U+001F: {line:?}"
                    );
                    tokens
                })
                .collect(),
        )
    }
}
