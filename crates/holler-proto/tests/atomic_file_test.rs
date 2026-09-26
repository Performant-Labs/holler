#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #483
//! `holler_proto::atomic_file` (issue #483): the one crash-safe way the hub
//! and the body persist a state file.
//!
//! Two entry points share one temp-write routine:
//!
//! - `write_atomic(path, bytes, mode)` replaces the destination: the bytes go
//!   to a temporary file in the destination's own directory, which is then
//!   renamed over the destination. A reader (or a process restarted after a
//!   kill) sees the old bytes or the new bytes, never an empty or partial file.
//! - `create_atomic(path, bytes, mode)` creates the destination only if it is
//!   absent. When another creator got there first it fails with
//!   `ErrorKind::AlreadyExists` and leaves the winner's bytes untouched.
//!
//! The temporary file's exact name is the helper's business. These tests only
//! assert that nothing but the destination is left in the directory.

#![cfg(unix)]

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use holler_proto::atomic_file::{create_atomic, write_atomic};

/// A per-test scratch directory under the OS temp dir, removed on drop.
struct Tdir {
    root: PathBuf,
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl Tdir {
    fn new(tag: &str) -> Self {
        let unique = format!(
            "holler-atomic-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let root = std::env::temp_dir().join(unique);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create scratch dir");
        Self { root }
    }
}

impl Drop for Tdir {
    fn drop(&mut self) {
        // Restore write permission first, in case a test made the dir read-only.
        let _ = std::fs::set_permissions(&self.root, std::fs::Permissions::from_mode(0o700));
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Every entry name in `dir`, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read scratch dir")
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path).expect("stat").permissions().mode() & 0o777
}

// --- write_atomic ----------------------------------------------------------

#[test]
fn write_atomic_writes_the_bytes_exactly_and_leaves_no_temp_file() {
    let dir = Tdir::new("exact");
    let dest = dir.root.join("tokens.json");
    let bytes: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
    write_atomic(&dest, &bytes, 0o600).expect("write_atomic");
    assert_eq!(std::fs::read(&dest).expect("read back"), bytes, "the bytes must be written exactly");
    assert_eq!(entries(&dir.root), vec!["tokens.json".to_string()], "no temporary file may be left behind");
}

#[test]
fn write_atomic_applies_the_requested_mode_to_the_final_file() {
    let dir = Tdir::new("mode");
    let secret = dir.root.join("tokens.json");
    write_atomic(&secret, b"[]\n", 0o600).expect("write 0600");
    assert_eq!(mode_of(&secret), 0o600, "a 0600 state file must end up 0600");

    let public = dir.root.join("listening.json");
    write_atomic(&public, b"{}\n", 0o644).expect("write 0644");
    assert_eq!(mode_of(&public), 0o644, "a 0644 state file must end up 0644");

    // Replacing an existing, looser file still yields the requested mode.
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644)).expect("loosen");
    write_atomic(&secret, b"[1]\n", 0o600).expect("rewrite 0600");
    assert_eq!(mode_of(&secret), 0o600, "a replaced file must carry the requested mode, not the old one");
}

#[test]
fn write_atomic_replaces_an_existing_destination() {
    let dir = Tdir::new("replace");
    let dest = dir.root.join("credential.json");
    write_atomic(&dest, b"old contents that are longer than the new ones\n", 0o600).expect("first write");
    write_atomic(&dest, b"new\n", 0o600).expect("second write");
    assert_eq!(std::fs::read(&dest).expect("read back"), b"new\n", "the old bytes must be fully replaced");
    assert_eq!(entries(&dir.root), vec!["credential.json".to_string()], "no temporary file may be left behind");
}

#[test]
fn write_atomic_into_a_missing_directory_fails_and_creates_nothing() {
    let dir = Tdir::new("missing");
    let dest = dir.root.join("no-such-dir").join("tokens.json");
    let err = write_atomic(&dest, b"[]\n", 0o600).expect_err("a missing parent directory must be an error");
    assert_eq!(err.kind(), ErrorKind::NotFound, "unexpected error: {err}");
    assert!(entries(&dir.root).is_empty(), "nothing may be created on failure: {:?}", entries(&dir.root));
}

#[test]
fn write_atomic_removes_its_temp_file_when_the_rename_fails() {
    // The destination is a non-empty directory, so the temporary file is
    // written but cannot be renamed over it.
    let dir = Tdir::new("rename-fails");
    let dest = dir.root.join("advertise.json");
    std::fs::create_dir(&dest).expect("mkdir dest");
    std::fs::write(dest.join("keep"), b"x").expect("populate dest");
    write_atomic(&dest, b"{}\n", 0o644).expect_err("renaming a file over a directory must fail");
    assert_eq!(entries(&dir.root), vec!["advertise.json".to_string()], "the temporary file must be removed on failure");
    assert!(dest.is_dir(), "the destination must be left as it was");
}

#[test]
fn write_atomic_leaves_the_old_contents_intact_when_it_fails_before_the_rename() {
    let dir = Tdir::new("readonly");
    let dest = dir.root.join("tokens.json");
    write_atomic(&dest, b"[\"old\"]\n", 0o600).expect("first write");
    // A read-only directory refuses the temporary file, so the operation
    // fails before it can rename anything.
    std::fs::set_permissions(&dir.root, std::fs::Permissions::from_mode(0o500)).expect("make dir read-only");
    let probe = dir.root.join("probe");
    if std::fs::write(&probe, b"").is_ok() {
        // Privileged user (root ignores the mode): the failure cannot be staged.
        let _ = std::fs::remove_file(&probe);
        eprintln!("skipping: running with privileges that ignore directory modes");
        return;
    }
    write_atomic(&dest, b"[\"new\"]\n", 0o600).expect_err("a read-only directory must refuse the write");
    assert_eq!(std::fs::read(&dest).expect("read back"), b"[\"old\"]\n", "the old contents must survive a failed write");
    std::fs::set_permissions(&dir.root, std::fs::Permissions::from_mode(0o700)).expect("restore");
    assert_eq!(entries(&dir.root), vec!["tokens.json".to_string()], "nothing else may be left behind");
}

/// The property #483 needs: a concurrent reader of a file being rewritten
/// sees the old or the new contents, never an empty, short or mixed file.
/// A truncate-then-write implementation fails this within a few iterations.
#[test]
fn a_concurrent_reader_never_sees_an_empty_or_partial_file() {
    let dir = Tdir::new("reader");
    let dest = dir.root.join("tokens.json");
    let a = vec![b'a'; 256 * 1024];
    let b = vec![b'b'; 256 * 1024];
    write_atomic(&dest, &a, 0o600).expect("seed");

    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let (dest, stop, a, b) = (dest.clone(), stop.clone(), a.clone(), b.clone());
        std::thread::spawn(move || {
            let mut reads = 0usize;
            while !stop.load(Ordering::SeqCst) {
                let got = std::fs::read(&dest).expect("the destination must always exist");
                assert!(
                    got == a || got == b,
                    "a reader saw a torn file: {} bytes (expected {} bytes of all-a or all-b)",
                    got.len(),
                    a.len()
                );
                reads += 1;
            }
            reads
        })
    };
    for i in 0..200 {
        let next = if i % 2 == 0 { &b } else { &a };
        write_atomic(&dest, next, 0o600).expect("rewrite");
    }
    stop.store(true, Ordering::SeqCst);
    let reads = reader.join().expect("the reader must not see a torn file");
    assert!(reads > 0, "the reader must have run");
    assert_eq!(entries(&dir.root), vec!["tokens.json".to_string()], "no temporary file may be left behind");
}

// --- create_atomic ---------------------------------------------------------

#[test]
fn create_atomic_creates_an_absent_file_with_the_bytes_and_mode() {
    let dir = Tdir::new("create");
    let dest = dir.root.join("identity.key");
    let key = [7u8; 32];
    create_atomic(&dest, &key, 0o600).expect("create_atomic");
    assert_eq!(std::fs::read(&dest).expect("read back"), key.to_vec());
    assert_eq!(mode_of(&dest), 0o600, "a created key file must be 0600");
    assert_eq!(entries(&dir.root), vec!["identity.key".to_string()], "no temporary file may be left behind");
}

#[test]
fn create_atomic_refuses_to_overwrite_an_existing_file() {
    let dir = Tdir::new("no-clobber");
    let dest = dir.root.join(".pepper");
    create_atomic(&dest, &[1u8; 32], 0o600).expect("first create");
    let err = create_atomic(&dest, &[2u8; 32], 0o600).expect_err("a second create must be refused");
    assert_eq!(err.kind(), ErrorKind::AlreadyExists, "unexpected error: {err}");
    assert_eq!(std::fs::read(&dest).expect("read back"), vec![1u8; 32], "the existing bytes must be untouched");
    assert_eq!(entries(&dir.root), vec![".pepper".to_string()], "the loser's temporary file must be removed");
}

/// Two (here: many) concurrent creators of one file: exactly one wins, every
/// other gets `AlreadyExists`, and the file holds exactly the winner's bytes.
/// This is the race between `hub serve` and `hub token mint` creating the
/// pepper or the hub identity key.
#[test]
fn concurrent_creators_have_exactly_one_winner_whose_bytes_survive() {
    const CREATORS: usize = 16;
    for round in 0..20 {
        let dir = Tdir::new("race");
        let dest = dir.root.join("identity.key");
        let barrier = Arc::new(Barrier::new(CREATORS));
        let handles: Vec<_> = (0..CREATORS)
            .map(|i| {
                let (dest, barrier) = (dest.clone(), barrier.clone());
                std::thread::spawn(move || {
                    let bytes = vec![u8::try_from(i + 1).expect("small"); 32];
                    barrier.wait();
                    (bytes.clone(), create_atomic(&dest, &bytes, 0o600))
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().expect("creator thread")).collect();
        let winners: Vec<&Vec<u8>> = results.iter().filter(|(_, r)| r.is_ok()).map(|(b, _)| b).collect();
        assert_eq!(winners.len(), 1, "round {round}: exactly one creator must win, got {}", winners.len());
        for (_, r) in results.iter().filter(|(_, r)| r.is_err()) {
            let kind = r.as_ref().expect_err("loser").kind();
            assert_eq!(kind, ErrorKind::AlreadyExists, "round {round}: a loser must see AlreadyExists");
        }
        assert_eq!(
            &std::fs::read(&dest).expect("read back"),
            winners[0],
            "round {round}: the file must hold exactly the winner's bytes"
        );
        assert_eq!(entries(&dir.root), vec!["identity.key".to_string()], "round {round}: no temporary file may be left");
    }
}
