//! `holler pane watch` (story #643): the change feed as a stream, NDJSON in JSON mode.
//! The rig and the shared helpers are `crate::list`'s.
//!
//! "Each change once" is the port's contract (the brief's Decision 8): these cases pin
//! what the verb prints over the fake store's feed, including a write made while the
//! verb waits on it.

use std::thread;
use std::time::{Duration, Instant};

use holler_cli::output::Format;
use holler_pane::{Pane, PaneError, PaneName, PaneStore};
use holler_pane_testkit::envelope::check_ndjson;
use holler_pane_testkit::fixture::sample_profile;
use holler_pane_testkit::pane_store::PaneStoreOp;
use serde_json::{json, Value};

use crate::get::help;
use crate::list::{
    assert_fails, kv, member, ok_stream, ok_text, pane, scoped_rig, sync_rig, Rig, SYNC_WANT,
};

/// The store of AC 12. Seeded `demo-c1r1` (cursor 1) and `demo-c2r1` (2); then through
/// the port `cas_put` c1 (3) and c2 (4) and `delete` c1 (5); then another writer puts a
/// new `demo-c3r1` (6). The head is 6; c2 is at generation 2.
fn history_rig() -> Rig {
    let rig = Rig::new([pane("demo-c1r1"), pane("demo-c2r1")], []).unwrap();
    for name in ["demo-c1r1", "demo-c2r1"] {
        rig.panes
            .cas_put(
                &Pane {
                    generation: 1,
                    ..pane(name)
                },
                1,
            )
            .unwrap();
    }
    rig.panes
        .delete(&PaneName::parse("demo-c1r1").unwrap(), 2)
        .unwrap();
    rig.panes.concurrent_put(&pane("demo-c3r1")).unwrap();
    rig
}

/// `(cursor, name, change)` of each envelope of a JSON watch run that exits 0.
fn changes(rig: &Rig, argv: &[&str]) -> Vec<(u64, String, String)> {
    ok_stream(&rig.run(argv, Format::Json))
        .into_iter()
        .map(|e| {
            let d = e.data;
            let cursor = d["cursor"]
                .as_u64()
                .unwrap_or_else(|| panic!("data.cursor: {d}"));
            let name = d["name"]
                .as_str()
                .unwrap_or_else(|| panic!("data.name: {d}"));
            let change = d["change"]
                .as_str()
                .unwrap_or_else(|| panic!("data.change: {d}"));
            (cursor, name.to_owned(), change.to_owned())
        })
        .collect()
}

fn put(cursor: u64, name: &str) -> (u64, String, String) {
    (cursor, name.to_owned(), "put".to_owned())
}

#[test]
fn watch_from_the_start_prints_each_live_pane_once() {
    let rig = history_rig();
    let got = changes(&rig, &["pane", "watch", "--until-idle"]);
    assert_eq!(
        got,
        [put(4, "demo-c2r1"), put(6, "demo-c3r1")],
        "the current state, once"
    );
}

#[test]
fn watch_since_prints_each_later_change_once() {
    let rig = history_rig();
    let argv = ["pane", "watch", "--since", "2", "--until-idle"];
    let lines = ok_stream(&rig.run(&argv, Format::Json));
    let cursors: Vec<&Value> = lines.iter().map(|e| &e.data["cursor"]).collect();
    assert_eq!(
        cursors,
        [&json!(3), &json!(4), &json!(5), &json!(6)],
        "in order, each once"
    );
    let delete = json!({"cursor": 5, "name": "demo-c1r1", "change": "delete", "pane": null});
    assert_eq!(lines[2].data, delete);

    let run = rig.run(&argv, Format::Text);
    let text: Vec<&str> = ok_text(&run).lines().collect();
    assert_eq!(text.len(), 4, "one line per change: {run:?}");
    for (line, cursor) in text.iter().zip(3..) {
        assert!(line.starts_with(&format!("cursor={cursor} ")), "{line:?}");
    }
    assert_eq!(text[2], "cursor=5 delete demo-c1r1", "{run:?}");

    for format in [Format::Text, Format::Json] {
        let run = rig.run(&["pane", "watch", "--since", "6", "--until-idle"], format);
        assert_eq!(
            (run.code, run.out.as_str()),
            (0, ""),
            "at the head: nothing owed: {run:?}"
        );
    }
}

/// One run of AC 14: the verb waits at the head; a write lands while it waits.
fn concurrent_change_once() {
    let rig = history_rig();
    rig.panes.set_idle_wait(Duration::from_secs(2));
    let argv = ["pane", "watch", "--since", "6", "--until-idle"];
    let run = thread::scope(|s| {
        let verb = s.spawn(|| rig.run(&argv, Format::Json));
        let deadline = Instant::now() + Duration::from_secs(5);
        let waiting = || rig.panes.faults().calls().contains(&PaneStoreOp::WatchNext);
        // Observe the verb reach the stream's `next()` (or end), bounded; not a blind sleep.
        while !waiting() && !verb.is_finished() {
            assert!(
                Instant::now() < deadline,
                "the verb never waited on the stream"
            );
            thread::sleep(Duration::from_millis(5));
        }
        rig.panes
            .cas_put(
                &Pane {
                    generation: 2,
                    ..pane("demo-c2r1")
                },
                2,
            )
            .unwrap();
        verb.join().unwrap()
    });
    let lines = ok_stream(&run);
    let cursors: Vec<&Value> = lines.iter().map(|e| &e.data["cursor"]).collect();
    assert_eq!(
        cursors,
        [&json!(7)],
        "the concurrent change, exactly once: {run:?}"
    );
}

#[test]
fn watch_prints_a_concurrent_change_exactly_once() {
    for _ in 0..5 {
        concurrent_change_once();
    }
}

#[test]
fn watch_named_pane_follows_only_that_pane() {
    let rig = history_rig();
    let got = changes(
        &rig,
        &["pane", "watch", "demo-c2r1", "--since", "0", "--until-idle"],
    );
    assert_eq!(got, [put(4, "demo-c2r1")]);
}

#[test]
fn watch_ends_at_a_store_error() {
    let rig = history_rig();
    let argv = ["pane", "watch", "--until-idle"];
    let fail = || {
        let error = PaneError::Unavailable { what: "hub".into() };
        rig.panes.faults().fail_next(PaneStoreOp::WatchNext, error);
    };

    fail();
    let text = rig.run(&argv, Format::Text);
    assert_eq!(text.code, 1, "{text:?}");
    assert_eq!(text.err.lines().count(), 1, "{text:?}");
    assert!(text.err.starts_with("error: unavailable: "), "{text:?}");

    fail();
    let json = rig.run(&argv, Format::Json);
    assert_eq!(json.code, 1, "{json:?}");
    assert!(json.err.is_empty(), "{json:?}");
    let lines = check_ndjson(&json.out, 1).unwrap_or_else(|f| panic!("{f}: {json:?}"));
    let codes: Vec<Option<String>> = lines.into_iter().map(|e| e.error.map(|e| e.code)).collect();
    assert_eq!(
        codes,
        [Some("unavailable".to_owned())],
        "one failure envelope: {json:?}"
    );
}

#[test]
fn watch_since_ahead_of_the_head_is_usage() {
    let rig = history_rig();
    assert_fails(
        &rig,
        &["pane", "watch", "--since", "99", "--until-idle"],
        2,
        "usage",
        false,
    );
}

#[test]
fn watch_with_nothing_owed_prints_nothing() {
    let rig = Rig::new([], []).unwrap();
    for format in [Format::Text, Format::Json] {
        let run = rig.run(&["pane", "watch", "--until-idle"], format);
        assert_eq!(run.code, 0, "{format:?}: {run:?}");
        assert!(
            run.out.is_empty() && run.err.is_empty(),
            "{format:?}: no output: {run:?}"
        );
    }
}

#[test]
fn watch_profile_prints_only_member_changes() {
    let rig = scoped_rig();
    let argv = ["pane", "watch", "--profile", "demo", "--until-idle"];
    assert_eq!(
        changes(&rig, &argv),
        [put(1, "demo-c1r1"), put(2, "demo-c2r1")]
    );
    let run = rig.run(&argv, Format::Text);
    let names: Vec<Vec<&str>> = ok_text(&run)
        .lines()
        .map(|l| l.split(' ').skip(1).take(2).collect())
        .collect();
    assert_eq!(
        names,
        [["put", "demo-c1r1"], ["put", "demo-c2r1"]],
        "{run:?}"
    );
}

/// Decision 7: a pane that leaves the profile, or is deleted while in it, prints that one
/// change, and its changes once outside print nothing. Seeded c4 with no profile (cursor 1),
/// then c1 (2) and c2 (3) in `demo`; c1 leaves (4), c2 is deleted (5), c1 changes again
/// outside the profile (6). From `--since 1` the replay shows c1 and c2 joining first.
#[test]
fn watch_profile_prints_a_pane_leaving_the_profile_once() {
    let profile = sample_profile("demo", &["demo-c1r1", "demo-c2r1"]).unwrap();
    let seeded = [
        pane("demo-c4r1"),
        member("demo-c1r1", "demo"),
        member("demo-c2r1", "demo"),
    ];
    let rig = Rig::new(seeded, [profile]).unwrap();
    let outside = |generation| Pane {
        generation,
        ..pane("demo-c1r1")
    };
    rig.panes.cas_put(&outside(1), 1).unwrap();
    rig.panes
        .delete(&PaneName::parse("demo-c2r1").unwrap(), 1)
        .unwrap();
    rig.panes.cas_put(&outside(2), 2).unwrap();

    let argv = [
        "pane",
        "watch",
        "--profile",
        "demo",
        "--since",
        "1",
        "--until-idle",
    ];
    assert_eq!(
        changes(&rig, &argv),
        [
            put(2, "demo-c1r1"),
            put(3, "demo-c2r1"),
            put(4, "demo-c1r1"),
            (5, "demo-c2r1".to_owned(), "delete".to_owned()),
        ]
    );
}

#[test]
fn watch_flags_a_mismatch() {
    let rig = sync_rig();
    let argv = ["pane", "watch", "--until-idle"];
    let run = rig.run(&argv, Format::Text);
    let sync: Vec<Option<&str>> = ok_text(&run).lines().map(|l| kv(l, "sync")).collect();
    let want: Vec<Option<&str>> = SYNC_WANT.iter().map(|(_, text, _)| Some(*text)).collect();
    assert_eq!(sync, want, "{run:?}");
    let lines = ok_stream(&rig.run(&argv, Format::Json));
    let sync: Vec<Value> = lines
        .iter()
        .map(|e| json!([e.data["name"], e.data["pane"]["sync"]]))
        .collect();
    let want: Vec<Value> = SYNC_WANT
        .iter()
        .map(|(name, _, json)| json!([name, json]))
        .collect();
    assert_eq!(sync, want);
}

#[test]
fn watch_help_documents_the_stream() {
    let help = help(&["pane", "watch"]);
    for needle in [
        "NDJSON",
        "--since",
        "--until-idle",
        "\"cursor\"",
        "\"change\"",
        // Amendment 1, A's W-4: "pane" is a row, not the record `pane get` prints.
        "pane get",
    ] {
        assert!(
            help.contains(needle),
            "`pane watch --help` names {needle:?}:\n{help}"
        );
    }
}
