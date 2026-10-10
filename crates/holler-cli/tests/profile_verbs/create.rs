//! `holler profile create` (story #662b, AC 2): an empty profile, a detached copy of
//! another (`--from`), or a snapshot of every pane that makes each a member
//! (`--from-current`), with the undo of a failed join (Decisions 8, B2).

use clap::error::ErrorKind;
use holler_pane::pane::{ContextCeilings, ModelSpec, PaneRole};
use holler_pane::profile_snapshot::profile_from_panes;
use holler_pane::{
    Argv, EnvVarName, GridPos, Pane, PaneError, PaneName, PaneStore, Profile, ProfileName,
    ProfileSpec, ProfileStore,
};
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::{json, Value};

use crate::list::rig::{
    assert_failure, assert_message_contains, matching_spec, member, pane, profile, run_both,
    run_both_seamed, run_both_with, NthPut, Rig,
};
use crate::verb_harness::parse::try_parse;

const NAME: &str = "Some Profile";

fn name(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

/// The profile `text` as `rig`'s store holds it.
fn stored(rig: &Rig, text: &str) -> Option<Profile> {
    rig.profiles.get(&name(text)).unwrap()
}

fn pane_writes(rig: &Rig) -> usize {
    let calls = rig.panes.faults().calls();
    calls
        .iter()
        .filter(|op| **op == PaneStoreOp::CasPut)
        .count()
}

fn profile_writes(rig: &Rig) -> usize {
    let calls = rig.profiles.faults().calls();
    calls
        .iter()
        .filter(|op| matches!(op, ProfileStoreOp::CasPut | ProfileStoreOp::Delete))
        .count()
}

/// `data.members` of a successful JSON run, as names.
fn members(data: &Value) -> Vec<&str> {
    data["members"]
        .as_array()
        .expect("members is an array")
        .iter()
        .map(|m| m.as_str().expect("a member is a pane name"))
        .collect()
}

/// A sample pane at `row`/`col`, configured with values that differ from every other pane
/// built here, so a snapshot that mixes two panes up shows.
fn configured(pane_name: &str, row: u16, col: u16, n: u32) -> Pane {
    let mut p = pane(pane_name);
    p.herdr.grid = GridPos { row, col };
    p.host.cwd = format!("/srv/demo/w{n}");
    p.harness.port = 48100 + u16::try_from(n).unwrap();
    p.model = ModelSpec {
        provider: "demo-provider".to_owned(),
        model_id: format!("demo-model-{n}"),
        effort: ["low", "medium", "high"][n as usize % 3].to_owned(),
    };
    p.role = if n == 1 {
        PaneRole::Orchestrator
    } else {
        PaneRole::Agent
    };
    p.env = vec![
        EnvVarName::parse(&format!("DEMO_VAR_{n}")).unwrap(),
        EnvVarName::parse("DEMO_SHARED").unwrap(),
    ];
    p.context = ContextCeilings {
        soft: 10_000 * n,
        hard: 20_000 * n,
    };
    p.command = Some(argv(&["opencode", "serve", &format!("--tag={n}")]));
    p.probe.check = Some(argv(&[
        "curl",
        "-s",
        &format!("http://127.0.0.1:{}", 48100 + n),
    ]));
    p.probe.expect = vec![format!("demo-model-{n}")];
    p
}

/// Three panes at r1c1, r1c2 and r2c1, each configured differently, in no profile.
fn three_panes() -> Vec<Pane> {
    vec![
        configured("demo-c1r1", 1, 1, 1),
        configured("demo-c2r1", 1, 2, 2),
        configured("demo-c1r2", 2, 1, 3),
    ]
}

/// The seeded records of `panes` (generation 1, in `list()` order, by name).
fn seeded_records(panes: Vec<Pane>) -> Vec<Pane> {
    Rig::new(panes, []).panes.list().unwrap()
}

#[test]
fn create_makes_an_empty_profile_in_both_formats() {
    let ran = run_both_with(
        || Rig::new([pane("demo-c1r1")], []),
        &["profile", "create", "Demo"],
        Rig::run,
    );
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert!(both.text.err.is_empty(), "{:?}", both.text);
    assert_eq!(
        both.text.out,
        "created profile Demo (demo): 0 specs, generation 1\n"
    );

    let data = &both.envelope.data;
    assert_eq!(data["members"], json!([]));
    assert_eq!(data["profile"]["generation"], 1);
    assert_eq!(data["profile"]["panes"], json!([]));
    for rig in ran.rigs() {
        let profile = stored(rig, "Demo").expect("Demo is stored");
        assert_eq!(profile.generation, 1);
        assert!(profile.panes.is_empty(), "{profile:?}");
        assert_eq!(pane_writes(rig), 0, "no pane is written");
        assert_eq!(
            rig.panes.list().unwrap(),
            seeded_records(vec![pane("demo-c1r1")])
        );
    }
    // The record as the store returned it, not the one the verb built (generation 0).
    assert_eq!(
        data["profile"],
        serde_json::to_value(stored(&ran.json_rig, "Demo").unwrap()).unwrap()
    );
}

#[test]
fn create_refuses_a_taken_name_or_slug() {
    let seed = || Rig::new([], [profile(NAME, vec![matching_spec("demo-c1r1")])]);
    for spelling in [NAME, "some-profile"] {
        let ran = run_both_with(seed, &["profile", "create", spelling], Rig::run);
        assert_failure(&ran.both, "profile-exists", 3);
        assert_message_contains(&ran.both, r#""Some Profile" (slug some-profile)"#);
        for rig in ran.rigs() {
            let kept = stored(rig, NAME).expect("still stored");
            assert_eq!(kept.generation, 1, "{spelling}: untouched");
            assert_eq!(kept.panes, vec![matching_spec("demo-c1r1")]);
            assert_eq!(
                profile_writes(rig),
                0,
                "{spelling}: checked before any write"
            );
        }
    }
}

#[test]
fn create_reports_a_create_race_as_profile_exists() {
    // Another writer created the name between the verb's `get` and its `cas_put`: the
    // real store answers `generation-conflict` (`next_generation(1, 0)`), which is a
    // taken name to the operator.
    let seed = || {
        let rig = Rig::new([], []);
        rig.profiles
            .faults()
            .fail_next(ProfileStoreOp::CasPut, PaneError::Conflict);
        rig
    };
    let both = run_both(seed, &["profile", "create", "Demo"]);
    assert_failure(&both, "profile-exists", 3);
    assert_message_contains(&both, r#""Demo" (slug demo)"#);
}

#[test]
fn create_from_makes_a_detached_copy() {
    // `Alpha` names a pane of its own and a pane with no record; the copy keeps both.
    let seed = || {
        Rig::new(
            [member("demo-c1r1", "Alpha"), pane("demo-c2r1")],
            [profile(
                "Alpha",
                vec![matching_spec("demo-c1r1"), matching_spec("demo-c9r9")],
            )],
        )
    };
    // The source is named by another spelling once: the text still uses its stored name.
    for source in ["Alpha", "ALPHA"] {
        let ran = run_both_with(
            seed,
            &["profile", "create", "Beta", "--from", source],
            Rig::run,
        );
        let both = &ran.both;
        assert_eq!(both.text.code, 0, "{:?}", both.text);
        let lines: Vec<&str> = both.text.out.lines().collect();
        assert_eq!(
            lines,
            vec![
                "created profile Beta (beta): 2 specs, generation 1",
                "copied from Alpha; no pane joined",
            ]
        );
        assert_eq!(members(&both.envelope.data), Vec::<&str>::new());
        for rig in ran.rigs() {
            let alpha = stored(rig, "Alpha").unwrap();
            let beta = stored(rig, "Beta").expect("Beta is stored");
            assert_eq!(beta.panes, alpha.panes, "the specs verbatim");
            assert_eq!(alpha.generation, 1, "the source is not written");
            assert_eq!(pane_writes(rig), 0, "no pane is written");
            assert_eq!(
                rig.panes.list().unwrap(),
                seeded_records(vec![member("demo-c1r1", "Alpha"), pane("demo-c2r1")]),
                "every pane record unchanged; demo-c1r1 still in Alpha"
            );
        }
    }
}

#[test]
fn create_from_a_missing_profile_is_profile_not_found() {
    let seed = || Rig::new([pane("demo-c1r1")], [profile("Other", vec![])]);
    let ran = run_both_with(
        seed,
        &["profile", "create", "Beta", "--from", "Alpha"],
        Rig::run,
    );
    assert_failure(&ran.both, "profile-not-found", 3);
    assert_message_contains(&ran.both, r#""Alpha""#);
    for rig in ran.rigs() {
        assert_eq!(stored(rig, "Beta"), None);
        assert_eq!(profile_writes(rig), 0);
    }
}

#[test]
fn create_from_current_snapshots_every_pane_and_joins_it() {
    let ran = run_both_with(
        || Rig::new(three_panes(), []),
        &["profile", "create", NAME, "--from-current"],
        Rig::run,
    );
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);

    let before = seeded_records(three_panes());
    let order = ["demo-c1r1", "demo-c1r2", "demo-c2r1"];
    assert_eq!(
        before.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        order,
        "list() order is by name"
    );
    let want = profile_from_panes(&name(NAME), &before).panes;
    for rig in ran.rigs() {
        let profile = stored(rig, NAME).expect("the profile is stored");
        assert_eq!(profile.panes, want, "one spec per pane, from its record");
        let after = rig.panes.list().unwrap();
        for (was, now) in before.iter().zip(&after) {
            let expected = Pane {
                profile: Some(name(NAME)),
                generation: was.generation + 1,
                ..was.clone()
            };
            assert_eq!(now, &expected, "only profile and generation change");
        }
    }

    let data = &both.envelope.data;
    assert_eq!(members(data), order);
    // r2c1 (demo-c1r2) is row 2, column 1, row first.
    assert_eq!(
        data["profile"]["panes"][1]["herdr"]["grid"],
        json!({"row": 2, "col": 1, "pos": "r2c1"})
    );
    assert_eq!(
        data["profile"],
        serde_json::to_value(stored(&ran.json_rig, NAME).unwrap()).unwrap()
    );

    let lines: Vec<&str> = both.text.out.lines().collect();
    assert_eq!(
        lines.first(),
        Some(&"created profile Some Profile (some-profile): 3 specs, generation 1")
    );
    let member_lines: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with("members: "))
        .collect();
    assert_eq!(
        member_lines,
        vec![&"members: demo-c1r1, demo-c1r2, demo-c2r1"]
    );
}

#[test]
fn create_from_current_of_no_panes_is_an_empty_profile_with_no_members() {
    let both = run_both(
        || Rig::new([], []),
        &["profile", "create", "Demo", "--from-current"],
    );
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(members(&both.envelope.data), Vec::<&str>::new());
    assert!(
        both.text.out.lines().any(|l| l == "members: none"),
        "{:?}",
        both.text
    );
}

#[test]
fn create_from_current_joins_a_pane_that_already_names_the_profile() {
    // A pane whose profile already has NAME's slug is not "in another profile".
    let seed = || Rig::new([member("demo-c1r1", "SOME-PROFILE"), pane("demo-c2r1")], []);
    let ran = run_both_with(
        seed,
        &["profile", "create", NAME, "--from-current"],
        Rig::run,
    );
    assert_eq!(ran.both.text.code, 0, "{:?}", ran.both.text);
    assert_eq!(
        members(&ran.both.envelope.data),
        vec!["demo-c1r1", "demo-c2r1"]
    );
    for rig in ran.rigs() {
        for p in rig.panes.list().unwrap() {
            let slug = p.profile.as_ref().map(ProfileName::slug);
            assert_eq!(
                slug.as_deref(),
                Some("some-profile"),
                "{} is a member",
                p.name
            );
        }
    }
}

#[test]
fn create_from_current_refuses_a_pane_in_another_profile_and_writes_nothing() {
    let seed = || {
        Rig::new(
            [
                member("demo-c1r1", "Other"),
                pane("demo-c2r1"),
                member("demo-c3r1", "Alpha"),
            ],
            [profile("Other", vec![]), profile("Alpha", vec![])],
        )
    };
    let ran = run_both_with(
        seed,
        &["profile", "create", NAME, "--from-current"],
        Rig::run,
    );
    assert_failure(&ran.both, "pane-in-other-profile", 3);
    // Every such pane, together, in list() order.
    assert_message_contains(
        &ran.both,
        r#"demo-c1r1 is in profile "Other"; demo-c3r1 is in profile "Alpha""#,
    );
    for rig in ran.rigs() {
        assert_eq!(stored(rig, NAME), None);
        assert_eq!(profile_writes(rig), 0, "no profile write");
        assert_eq!(pane_writes(rig), 0, "no pane write");
    }
}

#[test]
fn create_from_current_undoes_everything_when_a_join_fails() {
    // The join of the second pane loses a race; the first had joined.
    let ran = run_both_seamed(
        || Rig::new(three_panes(), []),
        &[(2, NthPut::Fail(PaneError::Conflict))],
        &["profile", "create", NAME, "--from-current"],
    );
    assert_failure(&ran.both, "generation-conflict", 1);
    assert_message_contains(&ran.both, "nothing was kept");
    for rig in ran.rigs() {
        assert_eq!(stored(rig, NAME), None, "the profile was deleted");
        for p in rig.panes.list().unwrap() {
            assert_eq!(p.profile, None, "{} left the profile", p.name);
        }
    }
}

#[test]
fn create_from_current_reports_profile_conflict_when_the_undo_fails() {
    // The 2nd cas_put (the join of pane 2) fails and does not land, so its re-read needs no
    // write; the 3rd (the undo of pane 1) fails too, and the undo stops there.
    let plan = [
        (2, NthPut::Fail(PaneError::Conflict)),
        (
            3,
            NthPut::Fail(PaneError::Unavailable {
                what: "pane store".to_owned(),
            }),
        ),
    ];
    let ran = run_both_seamed(
        || Rig::new(three_panes(), []),
        &plan,
        &["profile", "create", NAME, "--from-current"],
    );
    assert_failure(&ran.both, "profile-conflict", 1);
    assert_message_contains(
        &ran.both,
        "holler profile delete 'Some Profile' --keep-panes",
    );
    assert_message_contains(&ran.both, "holler profile show 'Some Profile'");
    assert_eq!(ran.both.text.err.lines().count(), 1, "{:?}", ran.both.text);
    for rig in ran.rigs() {
        assert!(
            stored(rig, NAME).is_some(),
            "the undo stopped before the delete, so the remedy still names a profile"
        );
        let first = rig
            .panes
            .get(&PaneName::parse("demo-c1r1").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(first.profile, Some(name(NAME)), "its undo failed");
    }
}

#[test]
fn create_from_current_undoes_a_join_that_landed_but_timed_out() {
    // B2: the second join is applied, then answers `timeout`; the undo re-reads it and
    // clears it too.
    let ran = run_both_seamed(
        || Rig::new([pane("demo-c1r1"), pane("demo-c2r1")], []),
        &[(
            2,
            NthPut::ApplyThenFail(PaneError::Timeout {
                op: "pane/cas_put".into(),
            }),
        )],
        &["profile", "create", NAME, "--from-current"],
    );
    assert_failure(&ran.both, "timeout", 1);
    assert_message_contains(&ran.both, "nothing was kept");
    for rig in ran.rigs() {
        assert_eq!(stored(rig, NAME), None, "the profile was deleted");
        for p in rig.panes.list().unwrap() {
            assert_eq!(p.profile, None, "{} left the profile", p.name);
        }
    }
}

#[test]
fn create_holds_env_names_never_values() {
    // I7: the snapshot copies env NAMES; a spec cannot even decode a value.
    let seed = || {
        let mut p = pane("demo-c1r1");
        p.env = vec![
            EnvVarName::parse("ALPHA_TOKEN").unwrap(),
            EnvVarName::parse("BETA_URL").unwrap(),
        ];
        Rig::new([p], [])
    };
    let ran = run_both_with(
        seed,
        &["profile", "create", NAME, "--from-current"],
        Rig::run,
    );
    assert_eq!(ran.both.text.code, 0, "{:?}", ran.both.text);
    let data = &ran.both.envelope.data;
    let stored_json = serde_json::to_value(stored(&ran.json_rig, NAME).unwrap()).unwrap();
    for panes in [&data["profile"]["panes"], &stored_json["panes"]] {
        let specs = panes.as_array().expect("panes is an array");
        assert_eq!(specs.len(), 1);
        for spec in specs {
            assert_eq!(spec["env"], json!(["ALPHA_TOKEN", "BETA_URL"]));
            for entry in spec["env"].as_array().unwrap() {
                assert!(!entry.as_str().unwrap().contains('='), "{entry}");
            }
        }
    }

    let mut spec = data["profile"]["panes"][0].clone();
    spec["env"] = json!(["TOKEN=s3cr3t"]);
    let error = serde_json::from_value::<ProfileSpec>(spec)
        .expect_err("a spec holding a value does not decode")
        .to_string();
    assert!(error.contains("profile-secret-refused"), "{error}");
    assert!(
        !error.contains("s3cr3t"),
        "the value is never echoed: {error}"
    );
}

#[test]
fn create_flags_conflict() {
    let error = try_parse(&["profile", "create", "X", "--from-current", "--from", "Y"])
        .expect_err("--from-current and --from conflict");
    assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn create_with_a_bad_name_is_usage_in_both_formats() {
    let seed = || Rig::new([pane("demo-c1r1")], [profile("Demo", vec![])]);
    for args in [
        &["profile", "create", "   "][..],
        &["profile", "create", "Fresh", "--from", "   "],
        &["profile", "create", "   ", "--from-current"],
    ] {
        let ran = run_both_with(seed, args, Rig::run);
        assert_failure(&ran.both, "usage", 2);
        for rig in ran.rigs() {
            assert_eq!(profile_writes(rig), 0, "{args:?}: nothing written");
            assert_eq!(pane_writes(rig), 0, "{args:?}: nothing written");
        }
    }
}
