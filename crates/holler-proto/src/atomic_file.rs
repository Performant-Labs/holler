//! Crash-safe writes of the hub's and the body's state files (issue #483).
//!
//! `std::fs::write` truncates its destination before it writes, so a process
//! killed in between leaves the file empty: a 0-byte `tokens.json` made every
//! later authentication fail ("tokens store is corrupted"). The two entry
//! points here never let the destination's name point at an incomplete file.
//! The bytes go to a temporary file in the destination's own directory, which
//! is created at the requested mode, and only the finished file gets the
//! destination's name:
//!
//! - [`write_atomic`] **replaces** the destination with `rename`. A reader, or
//!   a process restarted after a kill, sees the old bytes or the new bytes,
//!   never an empty or partial file.
//! - [`create_atomic`] **creates** the destination only if it is absent, with
//!   `hard_link`, which fails with [`ErrorKind::AlreadyExists`] instead of
//!   clobbering. Of concurrent creators exactly one wins, and a loser reads the
//!   winner's bytes back. It is for files that must never change once written:
//!   the token pepper and the hub's and the body's X25519 identity keys.
//!
//! The temporary file is `.<file name>.<pid>.<n>.tmp`, where `n` counts the
//! calls in this process, so no two concurrent calls (threads or processes)
//! share one. It is created with `O_EXCL` and **never opened when it already
//! exists**: a process killed between [`create_atomic`]'s `hard_link` and its
//! cleanup leaves its temporary file as a second name for the destination's
//! inode, and a later process that reuses the pid (the counter restarts at
//! zero) must not truncate it, which would empty the destination. A name that
//! is already taken is skipped and the next one tried. The temporary file is
//! removed whenever a call fails; one left behind by a killed process is inert
//! and is not swept.
//!
//! **Durability.** The failure #483 guards against is a process that dies
//! (SIGKILL, a crash), and `rename` and `hard_link` are atomic against that
//! without an `fsync`. [`write_atomic`] does not `fsync`: the token store
//! saves under its `flock` on mint, bind, revoke and the presence heartbeat,
//! and on macOS `sync_all` is a full `F_FULLFSYNC` that holds the lock long
//! enough to exhaust the store's bounded retry (#301, #401). [`create_atomic`]
//! runs once per file, so it `sync_all`s the temporary file before linking it:
//! a lost identity key means pairing again, and a lost pepper voids every
//! outstanding join secret.
//!
//! On Unix the mode is applied exactly, not filtered through the umask; other
//! platforms ignore it. This is the only module in the crate that touches the
//! filesystem: the standard library's file calls, with no network or async
//! dependency.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Counts the temporary files this process has made, so that concurrent calls
/// never share one (a pid alone is shared by every thread).
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// How many temporary names a call tries before giving up, when leftovers from
/// earlier processes hold the next ones.
const MAX_TEMP_TRIES: usize = 64;

/// Replace `path` with `bytes` at `mode`, atomically (see the module docs).
///
/// # Errors
///
/// Any I/O error making, writing or renaming the temporary file. The
/// destination is then untouched and the temporary file is removed.
pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let tmp = write_temp(path, bytes, mode, false)?;
    std::fs::rename(&tmp, path).inspect_err(|_| discard(&tmp))
}

/// Create `path` holding `bytes` at `mode`, only if it does not exist yet.
///
/// # Errors
///
/// [`ErrorKind::AlreadyExists`] when `path` already exists: another creator
/// won, its bytes are untouched, and the caller reads them back. Otherwise any
/// I/O error making or writing the temporary file. The temporary file is
/// removed in every case.
pub fn create_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let tmp = write_temp(path, bytes, mode, true)?;
    let linked = std::fs::hard_link(&tmp, path);
    discard(&tmp);
    linked
}

/// The temporary-file routine both entry points share: write `bytes` to a new
/// temporary file beside `path`, at `mode`, and return its path, closed. When
/// `durable`, the bytes are on disk before it returns. Nothing is left behind
/// when it fails.
fn write_temp(path: &Path, bytes: &[u8], mode: u32, durable: bool) -> io::Result<PathBuf> {
    for _ in 0..MAX_TEMP_TRIES {
        let tmp = temp_path(path)?;
        let file = match open_temp(&tmp, mode) {
            Ok(file) => file,
            // Taken by a leftover from an earlier process with this pid: leave
            // it alone and try the next name.
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        return match fill(file, bytes, mode, durable) {
            Ok(()) => Ok(tmp),
            Err(e) => {
                discard(&tmp);
                Err(e)
            }
        };
    }
    Err(io::Error::new(
        ErrorKind::AlreadyExists,
        format!("no free temporary file name beside {} after {MAX_TEMP_TRIES} tries", path.display()),
    ))
}

/// `.<file name>.<pid>.<n>.tmp`, in `path`'s own directory (a `rename` or a
/// `hard_link` never crosses a filesystem).
fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return Err(io::Error::new(ErrorKind::InvalidInput, format!("{} does not name a file", path.display())));
    };
    let mut tmp = OsString::from(".");
    tmp.push(name);
    tmp.push(format!(".{}.{}.tmp", std::process::id(), TEMP_SEQ.fetch_add(1, Ordering::Relaxed)));
    Ok(path.with_file_name(tmp))
}

/// Create the temporary file for writing, failing with
/// [`ErrorKind::AlreadyExists`] when the name is taken (`O_EXCL`; this also
/// refuses a symlink at that name). An existing file is never opened, so a
/// leftover that is another name for a live destination is never truncated. On
/// Unix it is created at `mode` (the umask can only tighten that), so it is
/// never readable by anyone the finished file will not be.
fn open_temp(tmp: &Path, mode: u32) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, mode);
    #[cfg(not(unix))]
    let _ = mode; // no permission bits to set
    opts.open(tmp)
}

/// Give `file` exactly `mode`, write `bytes`, and when `durable` flush them to
/// disk. Takes `file` by value, so it is closed before the caller renames or
/// links it.
fn fill(mut file: File, bytes: &[u8], mode: u32, durable: bool) -> io::Result<()> {
    set_mode(&file, mode)?;
    file.write_all(bytes)?;
    if durable {
        file.sync_all()?;
    }
    Ok(())
}

/// Set the open file's mode exactly (the umask applied at creation may have
/// cleared bits the caller asked for).
#[cfg(unix)]
fn set_mode(file: &File, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(mode))
}

/// No permission bits off Unix.
#[cfg(not(unix))]
fn set_mode(_file: &File, _mode: u32) -> io::Result<()> {
    Ok(())
}

/// Remove a temporary file after a failure, best effort: the error worth
/// reporting is the one that caused the failure.
fn discard(tmp: &Path) {
    let _ = std::fs::remove_file(tmp);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #483
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("holler-atomic-{tag}-{}-{}", std::process::id(), TEMP_SEQ.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn open_temp_never_opens_a_file_that_is_another_name_for_the_destination() {
        // A killed create_atomic leaves its temp as a second link to the
        // destination's inode; opening it (truncating) would empty the destination.
        let dir = scratch("link");
        let dest = dir.join("pepper");
        std::fs::write(&dest, b"secret bytes").unwrap();
        let leftover = dir.join(".pepper.1.0.tmp");
        std::fs::hard_link(&dest, &leftover).unwrap();

        let err = open_temp(&leftover, 0o600).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&dest).unwrap(), b"secret bytes", "the destination is intact");
        assert_eq!(std::fs::read(&leftover).unwrap(), b"secret bytes", "the leftover is untouched");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn open_temp_refuses_a_symlink_at_the_temp_name() {
        let dir = scratch("symlink");
        let victim = dir.join("victim");
        std::fs::write(&victim, b"keep").unwrap();
        let tmp = dir.join(".x.1.0.tmp");
        std::os::unix::fs::symlink(&victim, &tmp).unwrap();

        assert_eq!(open_temp(&tmp, 0o600).unwrap_err().kind(), ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_taken_temp_name_is_skipped_and_the_destination_survives() {
        // Fill the names this call could pick next with links to the destination:
        // it must skip them all, never touch the destination through one, and
        // still replace the destination with the new bytes.
        let dir = scratch("skip");
        let dest = dir.join("tokens.json");
        std::fs::write(&dest, b"old").unwrap();
        let start = TEMP_SEQ.load(Ordering::Relaxed);
        for n in start..start + 8 {
            std::fs::hard_link(&dest, dir.join(format!(".tokens.json.{}.{n}.tmp", std::process::id()))).unwrap();
        }

        write_atomic(&dest, b"new bytes", 0o600).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"new bytes");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
