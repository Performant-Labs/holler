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
        .truncate(false) // #489: only the winner truncates, once locked (`write_pid`).
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| LockError::Io(e.to_string()))?;

    match fs4::FileExt::try_lock(&file) {
        Ok(()) => {}
        Err(fs4::TryLockError::WouldBlock) => {
            // The holder's PID, read untouched from the file we failed to lock.
            let pid = std::io::read_to_string(&file).unwrap_or_default();
            return Err(LockError::Held(pid));
        }
        Err(e) => return Err(LockError::Io(e.to_string())),
    }
    write_pid(&file).map_err(|e| LockError::Io(e.to_string()))?;
    Ok(RunLockGuard { file, path })
}

// Replace the content with our PID: truncate, then one positioned write at offset 0 (no
// rewind needed). Only once the flock is held (#489: truncating on open let a loser blank the
// holder's PID); a loser reading in between sees an empty PID, never a garbled one. unix's
// `write_at` / windows' `seek_write` by `cfg`, not an unconditional import (holler-client#60).
// Twin of `holler_hub::serve`'s `write_pid`: keep the two in step.
#[cfg(unix)]
fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.set_len(0)?;
    file.write_at(std::process::id().to_string().as_bytes(), 0)?;
    Ok(())
}

#[cfg(windows)]
fn write_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    file.set_len(0)?;
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

    /// #489: a losing `acquire` must leave the holder's PID in the file, so
    /// its refusal names the holder. An `flock` is per open file description,
    /// so a second `acquire` in this same process loses. Unix only: Windows'
    /// mandatory `LockFileEx` blocks the loser's read regardless (not in CI).
    #[cfg(unix)]
    #[test]
    fn loser_names_holder_pid_and_leaves_it_in_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _first = acquire(dir.path()).expect("first acquire");
        let me = std::process::id().to_string();

        let second = acquire(dir.path());
        assert_eq!(second.err(), Some(LockError::Held(me.clone())), "the refusal must name the holder's pid");

        let on_disk = std::fs::read_to_string(lock_path(dir.path())).expect("read lock file");
        assert_eq!(on_disk, me, "a losing acquire must not blank the holder's pid");
    }

    /// #489: the winner writes exactly its own PID, even over a longer PID a
    /// dead holder left behind (the OS released its flock), with no trailing
    /// bytes from the stale content.
    #[cfg(unix)]
    #[test]
    fn winner_overwrites_a_longer_stale_pid_exactly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = lock_path(dir.path());
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir body/");
        std::fs::write(&path, "99999999999999999999").expect("seed a stale pid");

        let _g = acquire(dir.path()).expect("a dead holder's lock file must be reclaimed");
        let on_disk = std::fs::read_to_string(&path).expect("read lock file");
        assert_eq!(on_disk, std::process::id().to_string(), "the file must hold exactly the winner's pid");
    }
}
