//! Unit tests for the hold registry (issues #442, #460).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #442

use super::*;

fn state_in(dir: &std::path::Path) -> HubState {
    let s = HubState::from_root(dir.to_path_buf());
    std::fs::create_dir_all(&s.hub_dir).unwrap();
    s
}

#[test]
fn hold_and_release_are_idempotent_and_keep_the_first_reason() {
    let h = Holds::in_memory();
    let first = h.hold("io/alpha", Some("freeze"));
    assert!(first.newly_held);
    let again = h.hold("io/alpha", Some("a different reason"));
    assert!(!again.newly_held);
    assert_eq!(again.info, first.info, "a repeat keeps the original reason and since");
    assert!(h.release("io/alpha").was_held);
    assert!(!h.release("io/alpha").was_held);
    assert!(!h.release("io/never-held").was_held);
    assert!(h.check("io/alpha").is_none());
}

#[cfg(unix)]
#[test]
fn a_repeated_hold_after_a_failed_write_retries_and_reports_the_truth() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let h = Holds::load(&state);
    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let can_write_anyway = std::fs::File::create(state.hub_dir.join("probe")).is_ok();
    let first = h.hold("io/alpha", Some("x"));
    let repeat = h.hold("io/alpha", Some("x"));
    let rel_repeat = h.release("io/never");
    if !can_write_anyway {
        assert!(!first.persisted);
        assert!(!repeat.persisted, "a repeat must not claim the hold is on disk when it is not");
        assert!(!rel_repeat.persisted, "nor may a no-op release");
    }
    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let healed = h.hold("io/alpha", Some("x"));
    assert!(healed.persisted && !healed.newly_held, "the repeat retries the write once the directory is writable");
    assert!(Holds::load(&state).check("io/alpha").is_some());
}

#[test]
fn a_hold_is_per_session() {
    let h = Holds::in_memory();
    h.hold("io/alpha", None);
    assert!(h.check("io/alpha").is_some());
    assert!(h.check("io/beta").is_none());
    assert!(h.check("other/alpha").is_none());
}

#[test]
fn reason_is_sanitised() {
    assert_eq!(sanitize_reason("  freeze  "), Some("freeze".into()));
    assert_eq!(sanitize_reason("\u{1b}[31mred\u{7}"), Some("[31mred".into()));
    assert_eq!(sanitize_reason("   "), None);
    assert_eq!(sanitize_reason(&"x".repeat(500)).map(|r| r.chars().count()), Some(MAX_REASON_CHARS));
}

#[test]
fn holds_survive_a_reload_and_the_file_is_private() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let h = Holds::load(&state);
    let out = h.hold("io/alpha", Some("freeze"));
    assert!(out.persisted);
    h.hold("io/beta", None);
    h.release("io/beta");
    let reloaded = Holds::load(&state);
    assert_eq!(reloaded.check("io/alpha").map(|(i, _)| i), Some(out.info));
    assert!(reloaded.check("io/beta").is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(state.hub_dir.join("holds.json")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    let temps: Vec<_> = std::fs::read_dir(&state.hub_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(temps.is_empty(), "no temp file is left behind: {temps:?}");
}

#[test]
fn a_corrupt_file_is_moved_aside_and_the_hub_starts() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    std::fs::write(state.hub_dir.join("holds.json"), b"{ not json").unwrap();
    let h = Holds::load(&state);
    assert!(h.snapshot().is_empty());
    let aside: Vec<_> = std::fs::read_dir(&state.hub_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("holds.json.corrupt-"))
        .collect();
    assert_eq!(aside.len(), 1, "the corrupt file is kept, not discarded");
    assert_eq!(std::fs::read(aside[0].path()).unwrap(), b"{ not json");
    // The registry works and writes a fresh, valid file.
    assert!(h.hold("io/alpha", None).persisted);
    assert!(Holds::load(&state).check("io/alpha").is_some());
}

#[test]
fn an_unknown_version_is_treated_as_corrupt_not_silently_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    std::fs::write(state.hub_dir.join("holds.json"), br#"{"version":99,"holds":{}}"#).unwrap();
    let h = Holds::load(&state);
    assert!(h.snapshot().is_empty());
    assert!(!state.hub_dir.join("holds.json").exists(), "moved aside");
}

#[cfg(unix)]
#[test]
fn an_unwritable_directory_keeps_the_hold_in_memory_and_says_so() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let h = Holds::load(&state);
    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    // Root ignores directory modes; skip the assertion there.
    let writable_anyway = std::fs::File::create(state.hub_dir.join("probe")).is_ok();
    let out = h.hold("io/alpha", Some("freeze"));
    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(h.check("io/alpha").is_some(), "the hold is in force even though it could not be saved");
    if !writable_anyway {
        assert!(!out.persisted);
    }
    // Once the directory is writable again the next change persists everything.
    assert!(h.hold("io/beta", None).persisted);
    let reloaded = Holds::load(&state);
    assert!(reloaded.check("io/alpha").is_some() && reloaded.check("io/beta").is_some());
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_is_never_overwritten() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let file = state.hub_dir.join("holds.json");
    std::fs::write(&file, br#"{"version":1,"holds":{"io/old":{"since":"2026-01-01T00:00:00Z"}}}"#).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&file).is_ok() {
        return; // running as a user that can read anything
    }
    let h = Holds::load(&state);
    assert!(h.snapshot().is_empty());
    assert!(!h.hold("io/alpha", None).persisted, "writing is blocked so the unreadable holds are not lost");
    assert!(h.check("io/alpha").is_some());
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(std::fs::read_to_string(&file).unwrap().contains("io/old"));
}

/// A `check` that runs after a `hold` has returned always sees it, and one
/// that ran before it may not; racing threads never observe a hold that
/// then disappears without a release. Repeated many times.
#[test]
fn concurrent_checks_and_holds_are_totally_ordered() {
    for round in 0..200 {
        let h = Holds::in_memory();
        let barrier = Arc::new(std::sync::Barrier::new(5));
        let held_at = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut checkers = Vec::new();
        for _ in 0..4 {
            let (h, b, flag) = (h.clone(), barrier.clone(), held_at.clone());
            checkers.push(std::thread::spawn(move || {
                b.wait();
                let mut seen_held = false;
                for _ in 0..200 {
                    // Read the flag *before* the check: if `hold` had already
                    // returned, the check must see it.
                    let returned = flag.load(std::sync::atomic::Ordering::SeqCst);
                    let now = h.check("io/alpha").is_some();
                    assert!(!returned || now, "a check after hold() returned missed the hold (round {round})");
                    assert!(!seen_held || now, "a hold vanished without a release (round {round})");
                    seen_held |= now;
                }
            }));
        }
        barrier.wait();
        h.hold("io/alpha", None);
        held_at.store(true, std::sync::atomic::Ordering::SeqCst);
        for c in checkers {
            c.join().unwrap();
        }
    }
}

/// Two writers holding and releasing the same session at once leave the
/// file equal to the final in-memory state.
#[test]
fn racing_writers_leave_the_file_equal_to_memory() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    for _ in 0..30 {
        let h = Holds::load(&state);
        let mut joins = Vec::new();
        for t in 0..4 {
            let h = h.clone();
            joins.push(std::thread::spawn(move || {
                for i in 0..10 {
                    if (i + t) % 2 == 0 {
                        h.hold("io/alpha", Some("x"));
                    } else {
                        h.release("io/alpha");
                    }
                }
            }));
        }
        for j in joins {
            j.join().unwrap();
        }
        assert_eq!(Holds::load(&state).snapshot(), h.snapshot());
    }
}

// --- default holds and grants (issue #460) ------------------------------------

fn code(e: Result<(), holler_proto::WireError>) -> Option<(i64, Option<String>, Option<String>)> {
    e.err().map(|e| {
        let d = e.data.unwrap();
        let reason = d.reason.clone();
            (e.code, d.reason, d.hold_kind.or(reason))
    })
}

#[test]
fn glob_matching() {
    assert!(glob_match("*", "io/alpha"));
    assert!(glob_match("io/*", "io/alpha"));
    assert!(glob_match("io/alp?a", "io/alpha"));
    assert!(glob_match("*/alpha", "io/alpha") && glob_match("*ph*", "io/alpha"));
    assert!(!glob_match("io/*", "other/alpha") && !glob_match("io/alpha", "io/alph") && !glob_match("", "x"));
    assert!(glob_match("", "") && glob_match("**", "") && !glob_match("?", ""));
}

#[test]
fn nothing_joins_held_unless_asked() {
    let h = Holds::in_memory();
    assert_eq!(h.note_joined(["io/alpha".to_string()]), 0);
    assert!(h.check("io/alpha").is_none());
    assert!(h.admit("io/alpha", None).is_ok());
}

#[test]
fn a_session_joins_held_once_and_a_release_is_not_undone_by_the_next_presence() {
    let h = Holds::in_memory();
    h.set_join_held(vec!["io/*".into()]);
    assert_eq!(h.note_joined(["io/alpha".to_string(), "other/beta".to_string()]), 1);
    let (info, kind) = h.check("io/alpha").unwrap();
    assert_eq!((kind, info.reason.as_deref()), (Kind::Default, Some(JOIN_REASON)));
    assert!(h.check("other/beta").is_none(), "a name that does not match the pattern joins open");
    // The next heartbeat does not re-hold what a release opened.
    assert_eq!(h.release("io/alpha").lifted, Some(Kind::Default));
    assert_eq!(h.note_joined(["io/alpha".to_string()]), 0);
    assert!(h.admit("io/alpha", None).is_ok());
}

#[test]
fn a_default_hold_refuses_without_a_grant_and_admits_exactly_one_prompt_with_one() {
    let h = Holds::in_memory();
    h.set_join_held(vec!["*".into()]);
    h.note_joined(["io/alpha".to_string(), "io/beta".to_string()]);
    assert_eq!(code(h.admit("io/alpha", None)), Some((-32011, Some("held on join".into()), Some("default".into()))));
    let g = h.mint_grant("io/alpha", Duration::from_secs(60)).unwrap();
    assert!(g.default_held && !g.operator_held);
    // A plain say is still refused while the grant is live.
    assert_eq!(code(h.admit("io/alpha", None)).map(|c| c.0), Some(-32011));
    // A grant for a different session is refused and is not consumed.
    assert_eq!(code(h.admit("io/beta", Some(&g.grant))), Some((-32012, Some("other_session".into()), Some("other_session".into()))));
    assert_eq!(h.live_grants("io/alpha"), 1);
    // Exactly once.
    assert!(h.admit("io/alpha", Some(&g.grant)).is_ok());
    assert_eq!(code(h.admit("io/alpha", Some(&g.grant))).map(|c| (c.0, c.1)), Some((-32012, Some("used".into()))));
    // And the session is held again in the same step.
    assert_eq!(code(h.admit("io/alpha", None)).map(|c| c.0), Some(-32011));
    assert_eq!(code(h.admit("io/alpha", Some("gnt_nope"))).map(|c| c.1), Some(Some("unknown".into())));
}

#[test]
fn an_unused_grant_expires() {
    let h = Holds::in_memory();
    h.set_join_held(vec!["*".into()]);
    h.note_joined(["io/alpha".to_string()]);
    let g = h.mint_grant("io/alpha", Duration::from_millis(30)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    // Poll without consuming (`would_refuse`) until the ttl has passed.
    loop {
        match h.would_refuse("io/alpha", Some(&g.grant)) {
            Some(e) if e.data.as_ref().and_then(|d| d.reason.as_deref()) == Some("expired") => break,
            None if Instant::now() > deadline => panic!("the grant never expired"),
            _ => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    assert_eq!(code(h.admit("io/alpha", Some(&g.grant))).map(|c| c.1), Some(Some("expired".into())));
    assert_eq!(code(h.admit("io/alpha", None)).map(|c| c.0), Some(-32011), "held again once the grant expired");
}

#[test]
fn an_operator_hold_beats_a_grant_and_the_error_says_so() {
    let h = Holds::in_memory();
    h.set_join_held(vec!["*".into()]);
    h.note_joined(["io/alpha".to_string()]);
    let g = h.mint_grant("io/alpha", Duration::from_secs(60)).unwrap();
    h.hold("io/alpha", Some("drain"));
    let refused = h.admit("io/alpha", Some(&g.grant)).unwrap_err();
    assert_eq!(refused.code, -32011);
    let data = refused.data.unwrap();
    assert_eq!((data.hold_kind.as_deref(), data.reason.as_deref()), (Some("operator"), Some("drain")));
    assert_eq!(h.live_grants("io/alpha"), 1, "a refused-by-operator grant is not consumed");
    assert!(h.mint_grant("io/alpha", Duration::from_secs(60)).unwrap().operator_held);
    // Releasing the operator hold falls back to the default hold, not to open.
    let out = h.release("io/alpha");
    assert_eq!((out.lifted, out.still_held), (Some(Kind::Operator), true));
    assert_eq!(code(h.admit("io/alpha", None)).map(|c| c.0), Some(-32011));
    assert!(h.admit("io/alpha", Some(&g.grant)).is_ok(), "the grant minted earlier still works once the operator hold is gone");
    // A second release lifts the default hold.
    let out = h.release("io/alpha");
    assert_eq!((out.lifted, out.still_held), (Some(Kind::Default), false));
    assert!(h.admit("io/alpha", None).is_ok());
}

#[test]
fn the_roster_fields_name_the_top_hold() {
    let h = Holds::in_memory();
    h.set_join_held(vec!["*".into()]);
    h.note_joined(["io/alpha".to_string()]);
    let d = h.row_hold("io/alpha");
    assert_eq!((d.hold, d.hold_kind.as_deref(), d.hold_reason.as_deref(), d.hold_default), (true, Some("default"), Some(JOIN_REASON), false));
    h.hold("io/alpha", Some("drain"));
    let o = h.row_hold("io/alpha");
    assert_eq!((o.hold_kind.as_deref(), o.hold_reason.as_deref(), o.hold_default), (Some("operator"), Some("drain"), true));
}

#[test]
fn default_holds_persist_and_grants_do_not() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(dir.path());
    let h = Holds::load(&state);
    h.set_join_held(vec!["*".into()]);
    h.note_joined(["io/alpha".to_string()]);
    let g = h.mint_grant("io/alpha", Duration::from_secs(600)).unwrap();
    let after = Holds::load(&state); // a hub restart
    assert_eq!(after.check("io/alpha").map(|(_, k)| k), Some(Kind::Default), "the session is still held");
    assert_eq!(code(after.admit("io/alpha", Some(&g.grant))).map(|c| c.1), Some(Some("unknown".into())), "the live grant is void");
}

#[test]
fn racing_senders_with_one_grant_get_exactly_one_prompt_through() {
    for _ in 0..100 {
        let h = Holds::in_memory();
        h.set_join_held(vec!["*".into()]);
        h.note_joined(["io/alpha".to_string()]);
        let g = h.mint_grant("io/alpha", Duration::from_secs(60)).unwrap().grant;
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let admitted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let joins: Vec<_> = (0..8)
            .map(|i| {
                let (h, b, n, g) = (h.clone(), barrier.clone(), admitted.clone(), g.clone());
                std::thread::spawn(move || {
                    b.wait();
                    // Half present the grant, half do not.
                    let grant = (i % 2 == 0).then_some(g.as_str());
                    if h.admit("io/alpha", grant).is_ok() {
                        n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for j in joins {
            j.join().unwrap();
        }
        assert_eq!(admitted.load(std::sync::atomic::Ordering::SeqCst), 1, "exactly one prompt is delivered");
    }
}

#[test]
fn sessions_that_do_not_match_leave_nothing_behind() {
    let h = Holds::in_memory();
    h.set_join_held(vec!["io/*".into()]);
    for i in 0..5_000 {
        h.note_joined([format!("other/s{i}")]);
    }
    let st = h.state();
    assert!(st.default.is_empty() && st.released.is_empty(), "no per-session state is kept for names that do not match a pattern");
}
