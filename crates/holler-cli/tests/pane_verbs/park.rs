//! `holler pane park` and `holler pane unpark` (story #646 part 1, brief AC 1-7 and 9-10).
//!
//! Every case runs the verb in-process over the rig of fakes (`park/rig.rs`): a fake pane
//! store, a fake profile store, a `FakeProfileScope` over the two, and the four live fakes,
//! whose call logs prove the verbs never act on anything live (AC 9). The store-failure
//! cases (AC 8) are in `park/failures.rs`; unpark's own failure case is in `unpark.rs`.

pub(crate) mod rig;

pub(crate) mod failures;

use clap::error::ErrorKind;
use clap::Parser;
use holler_cli::output::Format;
use holler_cli::Cli;
use holler_pane::pane::Hold;
use holler_pane::{PaneName, ProfileName};
use holler_pane_testkit::envelope::check_envelope;
use holler_proto::clock::now_millis;
use serde_json::{json, Value};

use rig::{
    assert_failure, assert_failure_message, held, old_park, pane, profile_world, run_both, Rig,
    Verb, R, W,
};

/// The JSON `data` of one pane.
fn pane_data(name: &str, changed: bool, generation: u64, hold: &Hold) -> Value {
    json!({
        "name": name,
        "changed": changed,
        "generation": generation,
        "hold": serde_json::to_value(hold).unwrap(),
    })
}

/// A rig holding `demo-c1r1` alone, in no profile, with no hold.
fn one_pane() -> Rig {
    Rig::new([pane("demo-c1r1")], [])
}

/// AC 1: park sets the hold (and only the hold) by one write; unpark clears it by another.
#[test]
fn park_then_unpark_round_trips_one_pane() {
    let rig = one_pane();
    let seed = rig.seeded("demo-c1r1");
    let argv = Verb::Park.argv(&["demo-c1r1"]);

    let t0 = now_millis();
    let parked = rig.run(&argv, Format::Text);
    let t1 = now_millis();
    assert_eq!(parked.code, 0, "{parked:?}");
    assert_eq!(parked.err, "", "{parked:?}");
    assert_eq!(
        parked.out,
        format!("{}\n", Verb::Park.done_line("demo-c1r1"))
    );
    let record = rig.record("demo-c1r1");
    let Hold::Parked {
        reason,
        release_when,
        since,
    } = &record.hold
    else {
        panic!("demo-c1r1 is parked: {:?}", record.hold);
    };
    assert_eq!((reason.as_str(), release_when.as_str()), (R, W));
    assert!(
        t0 <= *since && *since <= t1,
        "since {since} is the verb's clock, within [{t0}, {t1}]"
    );
    assert_eq!(record.generation, 2, "one write");
    let rest = |mut pane: holler_pane::Pane| {
        pane.hold = Hold::None;
        pane.generation = 1;
        pane
    };
    assert_eq!(rest(record), seed, "only the hold changed");

    let unparked = rig.run(&Verb::Unpark.argv(&["demo-c1r1"]), Format::Text);
    assert_eq!(unparked.code, 0, "{unparked:?}");
    assert_eq!(unparked.err, "", "{unparked:?}");
    assert_eq!(unparked.out, "demo-c1r1: unparked\n");
    let record = rig.record("demo-c1r1");
    assert_eq!(record.hold, Hold::None);
    assert_eq!(record.generation, 3, "one more write");
    assert_eq!(rest(record), seed, "only the hold changed");
    rig.assert_no_live_call_and_no_profile_write();
}

/// AC 2: the same two runs in JSON: one envelope each, carrying `Hold`'s own serde form.
#[test]
fn park_and_unpark_json_pass_the_envelope_helper() {
    let rig = one_pane();
    let parked = rig.run(&Verb::Park.argv(&["demo-c1r1"]), Format::Json);
    assert_eq!(parked.err, "", "{parked:?}");
    let envelope =
        check_envelope(&parked.out, 0).unwrap_or_else(|fault| panic!("{fault}: {parked:?}"));
    let Hold::Parked { since, .. } = rig.record("demo-c1r1").hold else {
        panic!("demo-c1r1 is parked");
    };
    assert_eq!(
        envelope.data,
        json!({"panes": [{"name": "demo-c1r1", "changed": true, "generation": 2,
            "hold": {"parked": {"reason": R, "release_when": W, "since": since}}}]})
    );

    let unparked = rig.run(&Verb::Unpark.argv(&["demo-c1r1"]), Format::Json);
    assert_eq!(unparked.err, "", "{unparked:?}");
    let envelope =
        check_envelope(&unparked.out, 0).unwrap_or_else(|fault| panic!("{fault}: {unparked:?}"));
    assert_eq!(
        envelope.data,
        json!({"panes": [{"name": "demo-c1r1", "changed": true, "generation": 3, "hold": "none"}]})
    );
    rig.assert_no_live_call_and_no_profile_write();
}

/// Run `argv` in both formats over a rig of `seed` alone; expect exit 0, `line` as the
/// whole text output and the pane reported unchanged at generation 1 with its stored
/// hold, and no write at all.
fn assert_left_alone(seed: holler_pane::Pane, argv: &[&str], line: &str) {
    let name = seed.name.to_string();
    let hold = seed.hold.clone();
    let both = run_both(|| Rig::new([seed.clone()], []), argv);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(both.text.err, "", "{:?}", both.text);
    assert_eq!(both.text.out, format!("{line}\n"), "{argv:?}");
    assert_eq!(
        both.envelope.data,
        json!({"panes": [pane_data(&name, false, 1, &hold)]}),
        "{argv:?}"
    );
    both.assert_nothing_written();
}

/// AC 3: a pane already in the asked state, or drained, is reported and never written.
#[test]
fn park_and_unpark_leave_a_pane_already_in_that_state() {
    // (a) Already parked: park keeps the old reason, release condition and `since`.
    assert_left_alone(
        held(pane("demo-c1r1"), old_park()),
        &Verb::Park.argv(&["demo-c1r1"]),
        "demo-c1r1: already parked (reason \"old\", release when \"later\")",
    );
    // (b) Not parked: unpark has nothing to do.
    assert_left_alone(
        pane("demo-c2r1"),
        &Verb::Unpark.argv(&["demo-c2r1"]),
        "demo-c2r1: not parked",
    );
    // (c) Drained: neither verb touches it.
    assert_left_alone(
        held(pane("demo-c3r1"), Hold::Drained),
        &Verb::Park.argv(&["demo-c3r1"]),
        "demo-c3r1: drained, left as it is",
    );
    assert_left_alone(
        held(pane("demo-c3r1"), Hold::Drained),
        &Verb::Unpark.argv(&["demo-c3r1"]),
        "demo-c3r1: not parked",
    );
}

/// AC 4: `--profile P` alone takes every member of P, in name order, and nothing else.
#[test]
fn park_and_unpark_with_a_profile_take_every_member_in_name_order() {
    let rig = profile_world();
    assert_takes_the_members(&rig, Verb::Park, 2);
    let (one, two) = (rig.record("demo-c1r1"), rig.record("demo-c2r1"));
    assert_eq!(one.hold, two.hold, "one run, one `since` (Decision 6)");
    // On the same rig, after the park run.
    assert_takes_the_members(&rig, Verb::Unpark, 3);

    let rig = profile_world();
    for verb in [Verb::Park, Verb::Unpark] {
        assert_json_lists_the_members(&rig, verb);
    }

    // A profile with no member panes: nothing to do, nothing written.
    for verb in [Verb::Park, Verb::Unpark] {
        let both = run_both(profile_world, &verb.argv(&["--profile", "Demo Empty"]));
        assert_eq!(both.text.code, 0, "{:?}", both.text);
        assert_eq!(
            both.text.out, "no panes in profile \"Demo Empty\"\n",
            "{verb:?}"
        );
        assert_eq!(both.envelope.data, json!({"panes": []}), "{verb:?}");
        both.assert_nothing_written();
    }
}

/// `verb --profile "Demo Alpha"` in text over `rig`: exit 0, one line per member in name
/// order, both members changed to `generation`, the other two panes as seeded.
fn assert_takes_the_members(rig: &Rig, verb: Verb, generation: u64) {
    let run = rig.run(&verb.argv(&["--profile", "Demo Alpha"]), Format::Text);
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(run.err, "", "{run:?}");
    assert_eq!(
        run.out,
        format!(
            "{}\n{}\n",
            verb.done_line("demo-c1r1"),
            verb.done_line("demo-c2r1")
        )
    );
    for name in ["demo-c1r1", "demo-c2r1"] {
        let record = rig.record(name);
        assert!(verb.is_done(&record.hold), "{verb:?} {record:?}");
        assert_eq!(record.generation, generation, "{name}");
    }
    for other in ["demo-c3r1", "demo-c4r1"] {
        assert_eq!(rig.record(other), rig.seeded(other), "{other} is unchanged");
    }
    rig.assert_no_live_call_and_no_profile_write();
}

/// `verb --profile "Demo Alpha"` in JSON over `rig`: `data.panes` is the two members, in
/// name order, each changed.
fn assert_json_lists_the_members(rig: &Rig, verb: Verb) {
    let run = rig.run(&verb.argv(&["--profile", "Demo Alpha"]), Format::Json);
    let envelope = check_envelope(&run.out, 0).unwrap_or_else(|fault| panic!("{fault}: {run:?}"));
    let panes = envelope.data["panes"]
        .as_array()
        .expect("data.panes is an array");
    let names: Vec<&str> = panes
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["demo-c1r1", "demo-c2r1"], "{verb:?}");
    assert!(
        panes.iter().all(|entry| entry["changed"] == true),
        "{verb:?}: {}",
        envelope.data
    );
    rig.assert_no_live_call_and_no_profile_write();
}

/// AC 5: the scope refuses a pane outside the profile, and a missing profile, before
/// anything is written; a named member with the profile is the one pane changed.
#[test]
fn park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile() {
    for verb in [Verb::Park, Verb::Unpark] {
        let cases: [(&[&str], &str); 3] = [
            (
                &["demo-c3r1", "--profile", "Demo Alpha"],
                "pane-not-in-profile",
            ),
            (
                &["demo-c4r1", "--profile", "Demo Alpha"],
                "pane-not-in-profile",
            ),
            (&["--profile", "Demo Gamma"], "profile-not-found"),
        ];
        for (target, code) in cases {
            let both = run_both(profile_world, &verb.argv(target));
            assert_failure(&both, code, 3);
            both.assert_nothing_written();
        }
    }

    let both = run_both(
        profile_world,
        &Verb::Park.argv(&["demo-c1r1", "--profile", "Demo Alpha"]),
    );
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(
        both.text.out,
        format!("{}\n", Verb::Park.done_line("demo-c1r1"))
    );
    for rig in [&both.text_rig, &both.json_rig] {
        assert!(Verb::Park.is_done(&rig.record("demo-c1r1").hold));
        assert_eq!(rig.cas_puts(), 1, "only demo-c1r1 is written");
        for other in ["demo-c2r1", "demo-c3r1", "demo-c4r1"] {
            assert_eq!(rig.record(other), rig.seeded(other), "{other} is unchanged");
        }
    }
}

/// AC 6: a pane with no record, named without a profile, is `pane-not-found`.
#[test]
fn park_and_unpark_refuse_a_pane_with_no_record() {
    for verb in [Verb::Park, Verb::Unpark] {
        let both = run_both(one_pane, &verb.argv(&["demo-c9r9"]));
        assert_failure_message(&both, "pane-not-found", 3, "pane not found: demo-c9r9");
        both.assert_nothing_written();
    }
}

/// Run `argv` in both formats over [`one_pane`]; expect `usage` with exactly `message`
/// and no call through any port (AC 7).
fn assert_usage(argv: &[&str], message: &str) {
    let both = run_both(one_pane, argv);
    assert_failure_message(&both, "usage", 2, message);
    both.text_rig.assert_no_call_at_all();
    both.json_rig.assert_no_call_at_all();
}

/// `pane park demo-c1r1 --reason <reason> --release-when <release>`.
fn park_with<'a>(reason: &'a str, release: &'a str) -> Vec<&'a str> {
    vec![
        "pane",
        "park",
        "demo-c1r1",
        "--reason",
        reason,
        "--release-when",
        release,
    ]
}

/// AC 7: a malformed request is `usage`, refused by the verb before any store call.
#[test]
fn park_and_unpark_usage_errors_touch_no_store() {
    // (a) Neither PANE nor --profile.
    assert_usage(
        &["pane", "park", "--reason", R, "--release-when", W],
        "pane park needs a PANE or --profile NAME",
    );
    assert_usage(
        &["pane", "unpark"],
        "pane unpark needs a PANE or --profile NAME",
    );

    // (b) An invalid pane name: `PaneName::parse`'s own message.
    let bad_pane = PaneName::parse("a/b").unwrap_err().to_string();
    for verb in [Verb::Park, Verb::Unpark] {
        assert_usage(&verb.argv(&["a/b"]), &bad_pane);
    }

    // (c) An invalid profile name: `ProfileName::parse`'s own message.
    let bad_profile = ProfileName::parse("   ").unwrap_err().to_string();
    for verb in [Verb::Park, Verb::Unpark] {
        assert_usage(&verb.argv(&["--profile", "   "]), &bad_profile);
    }

    // (d) The text guards, on each flag in turn; the message never echoes the value.
    let long = "x".repeat(201);
    let long_wide = "é".repeat(201);
    let refused: [(&str, &str); 6] = [
        ("", "must not be blank"),
        ("   ", "must not be blank"),
        ("a\nb", "must not contain a control character"),
        ("a\u{1b}b", "must not contain a control character"),
        (&long, "must be at most 200 characters"),
        (&long_wide, "must be at most 200 characters"),
    ];
    for (bad, rule) in refused {
        assert_usage(&park_with(bad, W), &format!("--reason {rule}"));
        assert_usage(&park_with(R, bad), &format!("--release-when {rule}"));
    }

    // The order of the checks: only the first failure is reported (Decision 5).
    assert_usage(
        &["pane", "park", "--reason", "", "--release-when", ""],
        "pane park needs a PANE or --profile NAME",
    );
    assert_usage(
        &["pane", "park", "a/b", "--reason", "", "--release-when", ""],
        &bad_pane,
    );
    assert_usage(
        &[
            "pane",
            "park",
            "--profile",
            "   ",
            "--reason",
            "",
            "--release-when",
            "",
        ],
        &bad_profile,
    );
    assert_usage(&park_with("", ""), "--reason must not be blank");

    // At the limit, and trimmed: accepted and stored trimmed.
    let limit = "x".repeat(200);
    let limit_wide = "é".repeat(200);
    let accepted: [(&str, &str, &str, &str); 4] = [
        (&limit, W, &limit, W),
        (R, &limit, R, &limit),
        (&limit_wide, &limit_wide, &limit_wide, &limit_wide),
        ("  disk full  ", "  after the cleanup  ", R, W),
    ];
    for (reason, release, stored_reason, stored_release) in accepted {
        let rig = one_pane();
        let run = rig.run(&park_with(reason, release), Format::Text);
        assert_eq!(run.code, 0, "{reason:?} / {release:?}: {run:?}");
        let Hold::Parked {
            reason,
            release_when,
            ..
        } = rig.record("demo-c1r1").hold
        else {
            panic!("demo-c1r1 is parked");
        };
        assert_eq!(
            (reason.as_str(), release_when.as_str()),
            (stored_reason, stored_release)
        );
        rig.assert_no_live_call_and_no_profile_write();
    }

    // (e) A missing --reason or --release-when is clap's.
    for argv in [
        ["holler", "pane", "park", "demo-c1r1", "--reason", "r"],
        ["holler", "pane", "park", "demo-c1r1", "--release-when", "w"],
    ] {
        let error = Cli::try_parse_from(argv).expect_err("a required flag is missing");
        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument, "{argv:?}");
    }
}
