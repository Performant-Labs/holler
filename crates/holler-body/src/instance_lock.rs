//! `body/run.lock` (issue #182): a per-state-dir advisory lock so at most one
//! `holler body run` is ever live against a given state dir at a time.
//!
//! Mirrors the hub's own instance lock (`holler_hub::serve`'s `LockGuard`,
//! story #143) exactly: an `flock` on a file holding the holder's PID, held
//! for the process lifetime and released (by the OS) on process death — so a
//! crashed `body run`'s lock is reclaimed by the next one without any stale
//! PID cleanup.

use std::path::{Path, PathBuf};

/// Why the instance lock could not be acquired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Another live `body run` holds it (`pid`, best-effort from the file).
    Held(String),
    /// The lock file itself could not be opened/locked (an OS I/O error).
    Io(String),
}

impl LockError {
    /// The CLI's one-line stderr reason (ADR 0003).
    pub fn message(&self) -> String {
        match self {
            LockError::Held(pid) => format!("another holler body run is active (pid {pid})"),
            LockError::Io(m) => format!("cannot acquire the body run lock: {m}"),
        }
    }
}

/// The lock file path: `<state>/body/run.lock`.
pub fn lock_path(state_root: &Path) -> PathBuf {
    state_root.join("body").join("run.lock")
}

/// A held advisory lock, released (the flock, and the file) when dropped —
/// mirroring the hub's `LockGuard`. A crash (no `Drop` run) still releases the
/// flock (the OS does that on process exit), so the next `body run` reclaims
/// it; only a *clean* exit also removes the file.
pub struct RunLockGuard {
    file: std::fs::File,
    path: PathBuf,
}

impl Drop for RunLockGuard {
    fn drop(&mut self) {
        let _ = fs4::FileExt::unlock(&self.file);
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Acquire the instance lock, creating `<state>/body/` as needed.
pub fn acquire(state_root: &Path) -> Result<RunLockGuard, LockError> {
    let path = lock_path(state_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| LockError::Io(e.to_string()))?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true) // the file holds only the *current* holder's PID.
        .write(true)
        .open(&path)
        .map_err(|e| LockError::Io(e.to_string()))?;

    match fs4::FileExt::try_lock(&file) {
        Ok(()) => {}
        Err(fs4::TryLockError::WouldBlock) => {
            let pid = std::fs::read_to_string(&path).unwrap_or_default();
            return Err(LockError::Held(pid));
        }
        Err(e) => return Err(LockError::Io(e.to_string())),
    }
    write_pid(&file).map_err(|e| LockError::Io(e.to_string()))?;
    Ok(RunLockGuard { file, path })
}

// Positioned write at offset 0 (the file was just truncated, so a plain
// `Write` would also land at 0 — `write_at`/`seek_write` are used anyway so
// this never depends on the file's current cursor position). The two traits
// are unix's `std::os::unix::fs::FileExt::write_at` and windows'
// `std::os::windows::fs::FileExt::seek_write` — same effect, different
// names, so the OS is picked by `cfg` rather than importing an absent trait
// unconditionally (the compile break this mirrors: holler-client#60; see
// `holler_hub::serve`'s twin `write_pid`, which had the same gap).
#[cfg(unix)]
fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.write_at(std::process::id().to_string().as_bytes(), 0)?;
    Ok(())
}

#[cfg(windows)]
fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    file.seek_write(std::process::id().to_string().as_bytes(), 0)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #182
mod tests {
    use super::*;

    #[test]
    fn second_acquire_is_held() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _first = acquire(dir.path()).expect("first acquire");
        let second = acquire(dir.path());
        assert!(matches!(second, Err(LockError::Held(_))));
    }

    #[test]
    fn lock_is_reclaimed_after_release() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let _g = acquire(dir.path()).expect("first acquire");
        } // dropped: the flock (and the file) are released here.
        let second = acquire(dir.path());
        assert!(second.is_ok(), "a released lock must be reclaimable");
    }
}
