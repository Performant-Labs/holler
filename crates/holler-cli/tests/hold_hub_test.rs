#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #442
//! Session hold, hub side (issue #442, umbrella #437), against a real hub, a
//! real body and `stub-acp`: no mocks of the circuit, and a real hub restart.
//! Tags: `test-grp-concurrency` (the race matrix) and lifecycle (the rest).
//!
//! The hold is set and cleared through the control socket
//! (`holler_hub::control::hold_at` / `release_at`; the CLI verbs are #443), and
//! prompts are sent the way `holler say` sends them (`control::say_at`).
//!
//! No fixed sleep is used as synchronisation: every wait polls an observable
//! outcome (a roster row, a reply) with a deadline, and the races synchronise
//! on a barrier. The invariant the races assert is never a timing: every
//! `say` is either delivered (a reply came back) or refused `session_held`,
//! never both and never lost, and a `say` started after a `hold` returned is
//! always refused.

mod support;

use std::sync::{Arc, Barrier};
use std::time::Duration;

use holler_hub::control::{self, ControlError};
use support::hold_rig::{
    assert_held, is_delivered, is_held, say_at_retrying, start_hub_on_free_port, turn_accepted, Rig, READY, SESSION,
};
use support::{join, kill_tree, mint_token, wait_for, write_sessions_toml, Body, StateDir};

// --- functional ------------------------------------------------------------------

#[test]
fn a_held_session_refuses_every_delivery_variant_and_others_are_unaffected() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    let out = rig.hold(SESSION, Some("deploy freeze")).unwrap();
    assert_eq!(out["hold"], true);
    assert_eq!(out["newly_held"], true);
    assert_eq!(out["persisted"], true);

    assert_held(rig.say(SESSION, false), Some("deploy freeze"));
    assert_held(rig.say(SESSION, true), Some("deploy freeze"));
    // The bare name resolves to the same session.
    assert_held(rig.say("alpha", false), Some("deploy freeze"));
    // `--replace` is the redirect half of `interrupt SESSION TEXT`.
    assert_held(control::interrupt_at(rig.root(), SESSION, Some("do this instead")), Some("deploy freeze"));

    // An unheld session is unaffected, in both variants.
    assert!(is_delivered(&rig.say("b/beta", false)));
    assert!(is_delivered(&rig.say("b/beta", true)));

    // A refusal delivered nothing: releasing makes the session usable at once,
    // with no turn or queue entry left behind by the refused prompts.
    rig.release(SESSION).unwrap();
    assert!(is_delivered(&rig.say(SESSION, false)));
}

#[test]
fn a_prompt_cannot_be_forwarded_around_the_hold() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    rig.hold(SESSION, None).unwrap();
    let prompt = serde_json::json!({
        "session": "alpha",
        "message": { "messageId": "m-x", "role": "ROLE_USER", "parts": [{ "text": "sneak" }] },
    });
    let res = control::query_remote_at(rig.root(), "b", "session/prompt", Some(prompt));
    match res {
        Err(ControlError::Refused(e)) => assert_eq!(e.code, -32601, "a prompt is not a forwardable method: {e:?}"),
        other => panic!("the generic forward must refuse a session/prompt: {other:?}"),
    }
    // And the session saw nothing: the roster row never left idle.
    assert_eq!(rig.row(SESSION).unwrap()["state"], "idle");
}

#[test]
fn the_running_turn_the_accepted_queue_and_interrupt_are_not_touched() {
    let rig = Rig::start(&[("alpha", &["--slow", "--chunks", "8"])]);
    let root = rig.root().to_path_buf();
    let prev = rig.turn_id(SESSION);
    let first = std::thread::spawn({
        let root = root.clone();
        move || say_at_retrying(&root, SESSION, "long turn", false)
    });
    rig.wait_dispatched(SESSION, prev);
    let queued = std::thread::spawn({
        let root = root.clone();
        move || say_at_retrying(&root, SESSION, "queued before the hold", true)
    });
    // The queued prompt was accepted (forwarded to the body) once the body's
    // own presence shows the session still working after a queue call was
    // made; poll the hub's talk log, which records a prompt when it is sent.
    let talklog = rig.hub_state.hub().join("talklog");
    let accepted = wait_for(READY, || {
        let n: usize = std::fs::read_dir(&talklog)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default().lines().filter(|l| l.contains("\"text\"")).count())
            .sum();
        (n >= 2).then_some(())
    });
    assert!(accepted.is_some(), "the queued prompt was never sent");

    rig.hold(SESSION, Some("drain")).unwrap();
    // New work is refused while both accepted turns still run.
    assert_held(rig.say(SESSION, true), Some("drain"));
    // The running turn completes and the queued one still runs.
    assert!(is_delivered(&first.join().unwrap()), "the running turn must complete");
    assert!(is_delivered(&queued.join().unwrap()), "the prompt accepted before the hold must still run");
    // Interrupt (no text) still works on a held session.
    let out = control::interrupt_at(rig.root(), SESSION, None).unwrap();
    assert_eq!(out["applied"], true);
    // And the session is still held afterwards.
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

#[test]
fn hold_and_release_are_idempotent_and_keep_the_first_reason() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let a = rig.hold(SESSION, Some("first")).unwrap();
    let b = rig.hold(SESSION, Some("second")).unwrap();
    assert_eq!(a["newly_held"], true);
    assert_eq!(b["newly_held"], false);
    assert_eq!(b["reason"], "first", "a repeated hold keeps the original reason");
    assert_eq!(b["since"], a["since"], "and the original since-time");
    assert_eq!(rig.release(SESSION).unwrap()["was_held"], true);
    assert_eq!(rig.release(SESSION).unwrap()["was_held"], false);
    assert_eq!(rig.hold(SESSION, None).unwrap()["newly_held"], true);
    assert_eq!(rig.release(SESSION).unwrap()["hold"], false);
    assert!(is_delivered(&rig.say(SESSION, false)));
}

#[test]
fn holding_or_releasing_an_unknown_session_is_refused() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    for res in [rig.hold("b/nope", None), rig.release("b/nope"), rig.hold("nope", Some("x"))] {
        match res {
            Err(ControlError::Refused(e)) => assert_eq!(e.code, -32003, "unknown_session: {e:?}"),
            other => panic!("expected unknown_session, got {other:?}"),
        }
    }
    assert!(rig.hold_file().is_null(), "no phantom hold was recorded");
}

#[test]
fn the_roster_shows_the_hold_and_a_held_idle_session_stays_held() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    rig.hold(SESSION, Some("deploy freeze")).unwrap();
    let held = rig.row(SESSION).unwrap();
    assert_eq!(held["state"], "idle", "the hold is orthogonal to the session's own state");
    assert_eq!(held["hold"], true);
    assert_eq!(held["hold_reason"], "deploy freeze");
    assert!(held["held_since"].as_str().is_some_and(|s| s.ends_with('Z')));
    // A session that is not held carries none of the fields (byte-identical to before).
    let open = rig.row("b/beta").unwrap();
    for f in ["hold", "hold_reason", "held_since"] {
        assert!(open.get(f).is_none(), "{f} must be absent on an unheld row: {open}");
    }
    // Still held (and still idle) after presence beats have refreshed the row.
    std::thread::sleep(Duration::from_millis(900));
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

// --- lifecycle: survives disconnect, re-join, prune, restart ---------------------

#[test]
fn a_hold_survives_body_disconnect_and_rejoin() {
    let mut rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    rig.hold(SESSION, Some("keep")).unwrap();
    rig.kill_body();
    rig.wait_row(SESSION, |r| r["conn_state"] != "connected");
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "held while disconnected");
    rig.start_body();
    rig.wait_row(SESSION, |r| r["conn_state"] == "connected");
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "a re-joining body is still held");
    assert_held(rig.say(SESSION, false), Some("keep"));
}

#[test]
fn a_hold_survives_a_roster_prune() {
    // A short TTL so the row is pruned within the test.
    let mut rig = Rig::start_with(
        &[("alpha", &["--chunks", "1"])],
        &[("HOLLER_ROSTER_RECONNECT_MS", "200"), ("HOLLER_ROSTER_GONE_MS", "400"), ("HOLLER_ROSTER_PRUNE_MS", "800"), ("HOLLER_ROSTER_SWEEP_MS", "100")],
    );
    rig.hold(SESSION, Some("keep")).unwrap();
    rig.kill_body();
    let pruned = wait_for(READY, || rig.row(SESSION).is_none().then_some(()));
    assert!(pruned.is_some(), "the row was never pruned");
    // The hold outlived its row: it can still be released by name, and the
    // registry still has it.
    assert!(rig.hold_file()["holds"].get(SESSION).is_some());
    rig.start_body();
    rig.wait_row(SESSION, |r| r["conn_state"] == "connected");
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true, "the re-created row is held again");
    assert_held(rig.say(SESSION, false), Some("keep"));
    assert_eq!(rig.release(SESSION).unwrap()["was_held"], true);
}

#[test]
fn holds_survive_a_real_hub_restart_and_a_rejoining_body_stays_held() {
    let mut rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"]), ("gamma", &["--chunks", "1"])]);
    let since = rig.hold("b/alpha", Some("one")).unwrap()["since"].clone();
    rig.hold("b/beta", None).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(rig.hub_state.hub().join("holds.json")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "the hold state file is private like the hub's other state");
    }

    rig.restart_hub(); // a real process kill and a fresh `holler hub serve`
    let reconnected = wait_for(Duration::from_secs(90), || {
        rig.row("b/gamma").filter(|r| r["conn_state"] == "connected")
    });
    assert!(reconnected.is_some(), "the body never re-joined the restarted hub");

    let alpha = rig.row("b/alpha").unwrap();
    assert_eq!(alpha["hold"], true);
    assert_eq!(alpha["hold_reason"], "one");
    assert_eq!(alpha["held_since"], since, "the since-time survived the restart");
    assert_eq!(rig.row("b/beta").unwrap()["hold"], true);
    assert!(rig.row("b/gamma").unwrap().get("hold").is_none());
    assert_held(rig.say("b/alpha", false), Some("one"));
    assert_held(rig.say("b/beta", true), None);
    assert!(is_delivered(&rig.say("b/gamma", false)), "an unheld session is unaffected by the restart");
}

// --- isolation -------------------------------------------------------------------

#[test]
fn holding_one_session_never_affects_another() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"]), ("beta", &["--chunks", "1"])]);
    rig.hold(SESSION, Some("only alpha")).unwrap();
    for _ in 0..5 {
        assert!(is_delivered(&rig.say("b/beta", false)));
        assert_held(rig.say(SESSION, false), Some("only alpha"));
    }
    rig.release("b/beta").unwrap(); // releasing an unheld session changes nothing
    assert_held(rig.say(SESSION, false), Some("only alpha"));
    assert!(rig.row("b/beta").unwrap().get("hold").is_none());
}

// --- persistence failure ---------------------------------------------------------

#[test]
fn a_corrupt_hold_file_does_not_stop_the_hub_and_is_kept() {
    let hub_state = StateDir::new();
    std::fs::create_dir_all(hub_state.hub()).unwrap();
    std::fs::write(hub_state.hub().join("holds.json"), b"{ this is not json").unwrap();
    let (mut hub, addr) = start_hub_on_free_port(&hub_state, &[], &[]); // panics if no hub comes up
    let (token_id, secret) = mint_token(&hub_state, "b");
    let body_state = StateDir::new();
    join(&body_state, &hub_state, &format!("ws://{addr}"), &token_id, &secret);
    let config = write_sessions_toml(&body_state, &[("alpha", &["--chunks", "1"])]);
    let body = Body::start_with_env(&body_state, &config, &[("HOLLER_HEARTBEAT_INTERVAL_MS", "300")]);
    let ready = wait_for(READY, || {
        let r = control::roster_at(hub_state.path(), false, None).ok()?;
        r["rows"].as_array()?.iter().any(|r| r["name"] == SESSION).then_some(())
    });
    assert!(ready.is_some());
    // The hub works, the corrupt file was moved aside intact, and a new hold persists.
    assert!(is_delivered(&say_at_retrying(hub_state.path(), SESSION, "hi", false)));
    let aside: Vec<_> = std::fs::read_dir(hub_state.hub())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("holds.json.corrupt-"))
        .collect();
    assert_eq!(aside.len(), 1, "the corrupt file must be kept, not deleted");
    assert_eq!(std::fs::read(aside[0].path()).unwrap(), b"{ this is not json");
    assert_eq!(control::hold_at(hub_state.path(), SESSION, None).unwrap()["persisted"], true);
    body.stop(&body_state, Duration::from_secs(5));
    kill_tree(&mut hub);
}

#[cfg(unix)]
#[test]
fn an_unwritable_state_directory_keeps_the_hold_in_force() {
    use std::os::unix::fs::PermissionsExt;
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let dir = rig.hub_state.hub();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let can_write_anyway = std::fs::File::create(dir.join("probe")).is_ok(); // running as root
    let out = rig.hold(SESSION, Some("in memory only")).unwrap();
    let repeat = rig.hold(SESSION, Some("in memory only")).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    if !can_write_anyway {
        assert_eq!(out["persisted"], false, "the caller is told the hold will not survive a restart");
        assert_eq!(repeat["persisted"], false, "and a repeat does not claim otherwise");
    }
    // Once the directory is writable, the next hold or release call saves it.
    assert_eq!(rig.hold(SESSION, None).unwrap()["persisted"], true);
    assert!(rig.hold_file()["holds"].get(SESSION).is_some());
    assert_held(rig.say(SESSION, false), Some("in memory only"));
    assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
}

// --- concurrency: the race matrix -------------------------------------------------

const ROUNDS: usize = 25;

/// Run `say` (plain) and `hold` at the same instant, many times. Every say is
/// delivered or refused, never lost; a say started after `hold` returned is
/// always refused.
#[test]
fn say_racing_hold_is_delivered_or_refused_never_lost() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    let (mut delivered, mut refused) = (0, 0);
    for round in 0..ROUNDS {
        let barrier = Arc::new(Barrier::new(2));
        let racer = std::thread::spawn({
            let (root, b) = (rig.root().to_path_buf(), barrier.clone());
            move || {
                b.wait();
                say_at_retrying(&root, SESSION, "race", false)
            }
        });
        barrier.wait();
        rig.hold(SESSION, Some("race")).unwrap();
        let res = racer.join().unwrap();
        if is_delivered(&res) {
            delivered += 1;
        } else if is_held(&res) {
            refused += 1;
        } else {
            panic!("round {round}: a say that raced a hold was neither delivered nor refused: {res:?}");
        }
        // Once hold() has returned, nothing new gets in.
        assert!(is_held(&rig.say(SESSION, false)), "round {round}: a say after hold() returned was not refused");
        assert!(is_held(&rig.say(SESSION, true)), "round {round}: a queued say after hold() returned was not refused");
        rig.release(SESSION).unwrap();
        rig.wait_row(SESSION, |r| r["state"] == "idle" && r.get("hold").is_none());
    }
    assert_eq!(delivered + refused, ROUNDS);
}

#[test]
fn say_racing_release_is_delivered_or_refused_never_lost() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    for round in 0..ROUNDS {
        rig.hold(SESSION, Some("race")).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let racer = std::thread::spawn({
            let (root, b) = (rig.root().to_path_buf(), barrier.clone());
            move || {
                b.wait();
                say_at_retrying(&root, SESSION, "race", false)
            }
        });
        barrier.wait();
        rig.release(SESSION).unwrap();
        let res = racer.join().unwrap();
        assert!(is_delivered(&res) || is_held(&res), "round {round}: neither delivered nor refused: {res:?}");
        // Once release() has returned, a say goes through.
        rig.wait_row(SESSION, |r| r["state"] == "idle");
        assert!(is_delivered(&rig.say(SESSION, false)), "round {round}: a say after release() returned was refused");
    }
}

/// A `--queue` prompt is in flight behind a running turn while the hold is
/// set: it is either accepted (and runs) or refused, and the running turn is
/// never affected.
#[test]
fn hold_while_a_queued_prompt_is_in_flight() {
    let rig = Rig::start(&[("alpha", &["--slow", "--chunks", "6"])]);
    for round in 0..8 {
        let root = rig.root().to_path_buf();
        let prev = rig.turn_id(SESSION);
        let running = std::thread::spawn({
            let root = root.clone();
            move || say_at_retrying(&root, SESSION, "running", false)
        });
        let dispatched = wait_for(READY, || {
            if running.is_finished() {
                return Some(false);
            }
            rig.row(SESSION).filter(|r| turn_accepted(r, prev.as_ref())).map(|_| true)
        });
        if dispatched != Some(true) {
            let res = if running.is_finished() { format!("{:?}", running.join().unwrap()) } else { "still running".into() };
            panic!("round {round}: the running say never dispatched a new turn; its result: {res}; roster: {:?}", rig.rows());
        }
        let barrier = Arc::new(Barrier::new(2));
        let queued = std::thread::spawn({
            let b = barrier.clone();
            move || {
                b.wait();
                say_at_retrying(&root, SESSION, "queued", true)
            }
        });
        barrier.wait();
        rig.hold(SESSION, None).unwrap();
        let q = queued.join().unwrap();
        assert!(is_delivered(&q) || is_held(&q), "round {round}: the queued say was lost: {q:?}");
        let r = running.join().unwrap();
        assert!(is_delivered(&r), "round {round}: the running turn was affected by the hold: {r:?}");
        rig.release(SESSION).unwrap();
        rig.wait_row(SESSION, |r| r["state"] == "idle");
    }
}

/// Two clients hold and release the same session at once: every call
/// succeeds, and the final hub state (roster, refusals and the file on disk)
/// agrees with the last operation.
#[test]
fn two_clients_holding_and_releasing_the_same_session_stay_consistent() {
    let rig = Rig::start(&[("alpha", &["--chunks", "1"])]);
    for round in 0..10 {
        let barrier = Arc::new(Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|t| {
                let (root, b) = (rig.root().to_path_buf(), barrier.clone());
                std::thread::spawn(move || {
                    b.wait();
                    for i in 0..20 {
                        let res = if (i + t) % 2 == 0 {
                            control::hold_at(&root, SESSION, Some("x"))
                        } else {
                            control::release_at(&root, SESSION)
                        };
                        assert!(res.is_ok(), "hold/release must always succeed: {res:?}");
                    }
                })
            })
            .collect();
        for w in workers {
            w.join().unwrap();
        }
        // Settle on a known final state, then check every view agrees.
        if round % 2 == 0 {
            rig.hold(SESSION, Some("final")).unwrap();
            let file_has = rig.hold_file()["holds"].get(SESSION).is_some();
            assert!(file_has, "round {round}: the file disagrees with the hub");
            assert_eq!(rig.row(SESSION).unwrap()["hold"], true);
            assert!(is_held(&rig.say(SESSION, false)));
        } else {
            rig.release(SESSION).unwrap();
            assert!(rig.hold_file()["holds"].get(SESSION).is_none(), "round {round}: the file disagrees with the hub");
            assert!(rig.row(SESSION).unwrap().get("hold").is_none());
            assert!(is_delivered(&rig.say(SESSION, false)));
        }
    }
}
