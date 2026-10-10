//! `say`, `interrupt` and `answer` with `--pane NAME` (story #670, decision 5; the
//! routing engine and the queued wait of story #646, part 3).
//!
//! SESSION, TEXT and CHOICE are one optional tail, resolved in code behind
//! `resolve()` (the `Query::resolve` pattern), so `say --pane NAME TEXT` parses while
//! every existing form keeps resolving to the same session and text. The accessor
//! returns the existing `holler_cli::Usage` for a missing or surplus positional.
//!
//! These assert the resolved target and text **by value**, and the pane routing
//! **by engine**: `say_cmd::resolve_pane_target` is the pure resolver the three verbs
//! call (its wire wait is untestable in-process, as with the rest of `say`), over the
//! fakes of #638 (`crate::list::Rig`). The refusal the routed forms had until #646
//! (exit 1, `not implemented`) is replaced by this engine; the process-level shape of
//! the routed forms is in `process/legacy_verbs.rs`.

use clap::Parser;
use holler_cli::prompt_target::{PromptArgs, PromptTarget};
use holler_cli::say_cmd::{pane_hold_refusal, queue_outcome, resolve_pane_target, QueueWait};
use holler_cli::{Cli, Command, Usage};
use holler_pane::error::{class_of, ErrorClass};
use holler_pane::pane::{Health, Hold};
use holler_pane::{Pane, ProfileName};
use holler_pane_testkit::fixture::sample_profile;
use holler_proto::Code;
use serde_json::json;

fn parse(args: &[&str]) -> Result<Cli, String> {
    let mut argv = vec!["holler"];
    argv.extend_from_slice(args);
    Cli::try_parse_from(&argv).map_err(|e| e.to_string())
}

/// `holler <args>` resolved through the verb's accessor. A clap refusal and the
/// accessor's `Usage` both come back as `Err(message)`: either is exit 2.
fn resolve(args: &[&str]) -> Result<PromptArgs, String> {
    match parse(args)?.command {
        Command::Say(say) => say.resolve().map_err(|u| u.to_string()),
        Command::Interrupt(interrupt) => interrupt.resolve().map_err(|u| u.to_string()),
        Command::Answer(answer) => answer.resolve().map_err(|u| u.to_string()),
        other => panic!("not a prompt verb: {other:?}"),
    }
}

fn session(name: &str) -> PromptTarget {
    PromptTarget::Session(name.to_string())
}

fn pane(name: &str) -> PromptTarget {
    PromptTarget::Pane(name.to_string())
}

fn expect(target: PromptTarget, arg: Option<&str>) -> PromptArgs {
    PromptArgs {
        target,
        arg: arg.map(str::to_string),
    }
}

// --- the existing SESSION forms resolve as before -----------------------------

#[test]
fn say_session_text_resolves_to_the_session_and_text() {
    assert_eq!(
        resolve(&["say", "io/alpha", "hello"]),
        Ok(expect(session("io/alpha"), Some("hello")))
    );
}

#[test]
fn say_session_with_parts_file_has_no_text() {
    assert_eq!(
        resolve(&["say", "io/alpha", "--parts-file", "msg.json"]),
        Ok(expect(session("io/alpha"), None))
    );
    let Command::Say(say) = parse(&["say", "io/alpha", "--parts-file", "msg.json"])
        .unwrap()
        .command
    else {
        panic!("say")
    };
    assert_eq!(say.parts_file.as_deref(), Some("msg.json"));
}

#[test]
fn say_flags_after_the_tail_still_parse() {
    let Command::Say(say) = parse(&["say", "io/alpha", "hello", "--queue", "--timeout", "30s"])
        .unwrap()
        .command
    else {
        panic!("say")
    };
    assert!(say.queue);
    assert_eq!(say.timeout, "30s");
    assert_eq!(
        say.resolve().unwrap(),
        expect(session("io/alpha"), Some("hello"))
    );
}

#[test]
fn say_with_a_third_positional_is_still_refused_by_clap() {
    assert!(parse(&["say", "io/alpha", "hello", "extra"]).is_err());
}

/// `num_args` bounds the values of one occurrence only, so a flag between positionals
/// starts another and clap accepts a third positional there. The accessor must refuse it
/// (the issue: "missing or extra positionals return the existing `cli::Usage`"), as
/// `origin/main`'s fixed `session` + `text` positionals did through clap. Parsing is
/// asserted first: that clap accepts the argv is the premise of the test.
#[test]
fn an_extra_positional_split_off_the_session_form_by_a_flag_is_refused() {
    for args in [
        &["say", "io/alpha", "hello", "--queue", "extra"][..],
        &["say", "io/alpha", "--timeout", "5m", "fix", "it"],
        &[
            "interrupt",
            "io/alpha",
            "--server",
            "ws://127.0.0.1:1",
            "stop",
            "now",
        ],
        &[
            "answer",
            "io/alpha",
            "--server",
            "ws://127.0.0.1:1",
            "1",
            "2",
        ],
    ] {
        assert!(
            parse(args).is_ok(),
            "{args:?}: clap accepts the split form (the premise of this test)"
        );
        let err = resolve(args).expect_err(&format!("{args:?}: a third positional is refused"));
        assert!(
            err.contains("positionals"),
            "{args:?}: refused by the accessor's Usage, not by a parse error: {err}"
        );
    }
}

/// The valid split form is unchanged: a flag between SESSION and TEXT still resolves to
/// both, as it does on `origin/main`.
#[test]
fn a_flag_between_session_and_text_still_resolves_to_both() {
    assert_eq!(
        resolve(&["say", "io/alpha", "--queue", "hello"]),
        Ok(expect(session("io/alpha"), Some("hello")))
    );
    assert_eq!(
        resolve(&[
            "interrupt",
            "io/alpha",
            "--server",
            "ws://127.0.0.1:1",
            "stop"
        ]),
        Ok(expect(session("io/alpha"), Some("stop")))
    );
    assert_eq!(
        resolve(&["answer", "io/alpha", "--server", "ws://127.0.0.1:1", "1"]),
        Ok(expect(session("io/alpha"), Some("1")))
    );
}

#[test]
fn interrupt_session_resolves_with_and_without_redirect_text() {
    assert_eq!(
        resolve(&["interrupt", "io/alpha"]),
        Ok(expect(session("io/alpha"), None))
    );
    assert_eq!(
        resolve(&["interrupt", "io/alpha", "stop"]),
        Ok(expect(session("io/alpha"), Some("stop")))
    );
}

#[test]
fn answer_session_choice_resolves() {
    assert_eq!(
        resolve(&["answer", "io/alpha", "yes"]),
        Ok(expect(session("io/alpha"), Some("yes")))
    );
}

// --- the --pane forms ------------------------------------------------------------

#[test]
fn say_pane_text_resolves_to_the_pane_and_text() {
    assert_eq!(
        resolve(&["say", "--pane", "demo-c1r1", "hello"]),
        Ok(expect(pane("demo-c1r1"), Some("hello")))
    );
}

#[test]
fn say_pane_with_parts_file_has_no_text() {
    assert_eq!(
        resolve(&["say", "--pane", "demo-c1r1", "--parts-file", "msg.json"]),
        Ok(expect(pane("demo-c1r1"), None))
    );
}

#[test]
fn say_pane_and_profile_resolve_to_the_pane() {
    assert_eq!(
        resolve(&["say", "--pane", "demo-c1r1", "--profile", "demo", "hello"]),
        Ok(expect(pane("demo-c1r1"), Some("hello")))
    );
}

#[test]
fn interrupt_pane_resolves_with_and_without_redirect_text() {
    assert_eq!(
        resolve(&["interrupt", "--pane", "demo-c1r1"]),
        Ok(expect(pane("demo-c1r1"), None))
    );
    assert_eq!(
        resolve(&["interrupt", "--pane", "demo-c1r1", "stop"]),
        Ok(expect(pane("demo-c1r1"), Some("stop")))
    );
}

#[test]
fn answer_pane_choice_resolves() {
    assert_eq!(
        resolve(&["answer", "--pane", "demo-c1r1", "yes"]),
        Ok(expect(pane("demo-c1r1"), Some("yes")))
    );
}

#[test]
fn roster_takes_a_profile() {
    assert!(parse(&["roster", "--profile", "demo"]).is_ok());
    assert!(parse(&["roster", "--prefix", "io", "--all"]).is_ok());
}

// --- usage errors (exit 2), by the accessor -----------------------------------

/// The accessor's error is the existing `holler_cli::Usage` (no second usage-error type).
#[test]
fn the_accessor_error_is_the_existing_usage_type() {
    let Command::Say(say) = parse(&["say"])
        .expect("an empty tail parses; the accessor refuses it")
        .command
    else {
        panic!("say")
    };
    let usage: Usage = say.resolve().unwrap_err();
    assert!(!usage.to_string().is_empty());
}

#[test]
fn a_missing_argument_is_refused_and_named() {
    let cases: [(&[&str], &str); 8] = [
        (&["say"], "session"),
        (&["say", "io/alpha"], "text"),
        (&["say", "--pane", "demo-c1r1"], "text"),
        (&["interrupt"], "session"),
        (&["answer"], "session"),
        (&["answer", "io/alpha"], "choice"),
        (&["answer", "--pane", "demo-c1r1"], "choice"),
        (&["say", "--parts-file", "msg.json"], "session"),
    ];
    for (args, missing) in cases {
        let err = resolve(args).expect_err(&format!("{args:?} is missing {missing}"));
        assert!(
            err.to_lowercase().contains(missing),
            "{args:?}: the message names the missing {missing}: {err}"
        );
    }
}

#[test]
fn a_pane_together_with_a_session_is_refused() {
    for args in [
        &["say", "--pane", "demo-c1r1", "io/alpha", "hello"][..],
        &["interrupt", "--pane", "demo-c1r1", "io/alpha", "stop"],
        &["answer", "--pane", "demo-c1r1", "io/alpha", "yes"],
    ] {
        assert!(
            resolve(args).is_err(),
            "{args:?}: a pane and a session name two targets"
        );
    }
}

#[test]
fn an_extra_positional_after_pane_and_text_is_refused() {
    for args in [
        &["say", "--pane", "demo-c1r1", "a", "b"][..],
        &["answer", "--pane", "demo-c1r1", "a", "b"],
        &["interrupt", "--pane", "demo-c1r1", "a", "b"],
    ] {
        assert!(resolve(args).is_err(), "{args:?}");
    }
}

// --- the pane routing engine (story #646, part 3) -------------------------------
//
// `say_cmd::resolve_pane_target` is the pure resolver every `--pane`/`--profile` form
// of the three verbs calls: it reads the pane's record and answers its session of
// record, refusing every pane-state condition the record itself names (parked,
// unhealthy, SHOWN != DRIVEN — never a live probe). Its twin predicate sits at the
// hub's `send_prompt` gate (`holler-hub/src/circuit/dispatch.rs`, binding 6).

use crate::list::{observed, Rig};

/// The world the resolver is asked over: `demo-c1r1` the healthy member of
/// `Demo Alpha`; `demo-c2r1` a member with no session of record; `demo-c3r1` parked,
/// in no profile; `demo-c4r1` unhealthy; `demo-c5r1` with SHOWN != DRIVEN;
/// `demo-c6r1`/`demo-c7r1` healthy with one observation side absent (not a
/// mismatch); `demo-c8r1` parked with no session of record (ordering).
fn routing_world() -> Rig {
    let parked = || Hold::Parked {
        reason: "disk full".into(),
        release_when: "later".into(),
        since: 5,
    };
    let panes = [
        routed(
            "demo-c1r1",
            Some("Demo Alpha"),
            Hold::None,
            Health::Healthy,
            Some("ses-demo-c1r1"),
        ),
        routed(
            "demo-c2r1",
            Some("Demo Alpha"),
            Hold::None,
            Health::Healthy,
            None,
        ),
        routed(
            "demo-c3r1",
            None,
            parked(),
            Health::Healthy,
            Some("ses-demo-c3r1"),
        ),
        routed(
            "demo-c4r1",
            None,
            Hold::None,
            Health::Unhealthy("server wedged".into()),
            Some("ses-demo-c4r1"),
        ),
        routed_mismatched("demo-c5r1", None),
        routed(
            "demo-c6r1",
            None,
            Hold::None,
            Health::Healthy,
            Some("ses-demo-c6r1"),
        ),
        routed(
            "demo-c7r1",
            None,
            Hold::None,
            Health::Healthy,
            Some("ses-demo-c7r1"),
        ),
        routed("demo-c8r1", None, parked(), Health::Healthy, None),
    ];
    Rig::new(
        panes,
        [sample_profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"]).unwrap()],
    )
    .unwrap()
}

/// The sample pane `name` with `profile`, `hold`, `health` and — when `session` is
/// given — a session of record that it shows and drives (an equal pair, the state a
/// healthy pane is left in).
fn routed(
    name: &str,
    profile: Option<&str>,
    hold: Hold,
    health: Health,
    session: Option<&str>,
) -> Pane {
    let shown = session; // equal shown and driven; the mismatch case is its own builder
    let mut pane = observed(name, session, shown, session, 1);
    pane.profile = profile.map(|named| ProfileName::parse(named).unwrap());
    pane.hold = hold;
    pane.harness.health = health;
    pane
}

/// The sample pane `name` whose SHOWN and DRIVEN both exist and differ.
fn routed_mismatched(name: &str, profile: Option<&str>) -> Pane {
    let mut pane = observed(
        name,
        Some("ses-shown"),
        Some("ses-shown"),
        Some("ses-driven"),
        1,
    );
    pane.profile = profile.map(|named| ProfileName::parse(named).unwrap());
    pane
}

/// AC: `--pane` on a healthy pane answers its session of record, and `--profile P`
/// beside it resolves the same session when the pane is of P.
#[test]
fn the_engine_answers_the_session_of_record_of_a_healthy_pane() {
    let rig = routing_world();
    for (pane, session) in [
        ("demo-c1r1", "ses-demo-c1r1"),
        ("demo-c6r1", "ses-demo-c6r1"),
        ("demo-c7r1", "ses-demo-c7r1"),
    ] {
        let answer =
            resolve_pane_target(rig.ports(), pane, None).unwrap_or_else(|e| panic!("{pane}: {e}"));
        assert_eq!(answer, session, "{pane}");
    }
    // One observation side absent (c6's driven, c7's shown) is not a mismatch: the
    // refusal needs both sides present and differing.
    assert!(
        resolve_pane_target(rig.ports(), "demo-c1r1", Some("Demo Alpha")).is_ok(),
        "--profile beside --pane resolves the member pane"
    );
}

/// AC + binding 3: every routed refusal is a refusal (exit 3, `class_of`'s table)
/// whose message carries the stable code; the pane (and, where it applies, the
/// profile or the health reason) is named.
#[test]
fn the_engine_refuses_every_routed_case_with_exit_3() {
    let cases: &[(&str, Option<&str>, &str, &[&str])] = &[
        ("demo-c9r9", None, "pane-not-found", &["demo-c9r9"]),
        (
            "demo-c1r1",
            Some("No Such Profile"),
            "profile-not-found",
            &["No Such Profile"],
        ),
        (
            "demo-c3r1",
            Some("Demo Alpha"),
            "pane-not-in-profile",
            &["demo-c3r1", "Demo Alpha"],
        ),
        ("demo-c2r1", None, "session-not-found", &["demo-c2r1"]),
        (
            "demo-c3r1",
            None,
            "pane-parked",
            &["demo-c3r1", "pane-parked"],
        ),
        (
            "demo-c4r1",
            None,
            "pane-unhealthy",
            &["demo-c4r1", "pane-unhealthy", "server wedged"],
        ),
        (
            "demo-c5r1",
            None,
            "pane-shown-driven-mismatch",
            &["demo-c5r1", "pane-shown-driven-mismatch"],
        ),
    ];
    for (pane, profile, code, named) in cases {
        let rig = routing_world();
        let error = resolve_pane_target(rig.ports(), pane, *profile)
            .expect_err(&format!("{pane}: refused with {code}"));
        assert_eq!(error.code(), *code, "{pane}: {error}");
        assert_eq!(
            class_of(error.code()),
            ErrorClass::Refusal,
            "{pane}: exit 3, the stable code in the message"
        );
        for needle in *named {
            assert!(
                error.to_string().contains(needle),
                "{pane}: the message names {needle}: {error}"
            );
        }
    }
}

/// The chain's order (the brief's design decision): `PaneName::parse`, then the
/// profile scope (so its refusals come from the helper, exactly as park), then the
/// record, then the session, then the pane-state checks.
#[test]
fn the_engine_checks_the_name_the_scope_the_record_then_the_session() {
    let rig = routing_world();
    // A bad pane name is usage (exit 2), before anything else.
    let usage = resolve_pane_target(rig.ports(), "a/b", None).unwrap_err();
    assert_eq!(usage.code(), "usage");
    assert_eq!(class_of(usage.code()), ErrorClass::Usage);

    // The scope refuses before the record is read: a pane with no record at all is
    // `pane-not-in-profile`, not `pane-not-found`.
    let fresh = Rig::new([], [sample_profile("Demo Alpha", &["demo-c1r1"]).unwrap()]).unwrap();
    assert_eq!(
        resolve_pane_target(fresh.ports(), "demo-c9r9", Some("Demo Alpha"))
            .unwrap_err()
            .code(),
        "pane-not-in-profile"
    );

    // The session check precedes the pane-state checks: a parked pane with no session
    // of record is `session-not-found`, not `pane-parked`.
    assert_eq!(
        resolve_pane_target(rig.ports(), "demo-c8r1", None)
            .unwrap_err()
            .code(),
        "session-not-found"
    );
}

// --- the routed `say --queue` wait (the brief's § Queue, Finding 1's remedy) -------

/// AC (`say --queue` routed returns without waiting for a reply): a deadline expiry
/// after acceptance is a success — `queued <session>` on stdout, exit 0 — never a
/// timeout failure, because the prompt was accepted and the turn keeps running.
#[test]
fn a_deadline_expiry_after_acceptance_is_queued_exit_0() {
    let text = queue_outcome("ses-demo-c1r1", QueueWait::Deadline, false);
    assert_eq!(text.message, "queued ses-demo-c1r1");
    assert!(!text.to_stderr, "stdout: the prompt was accepted");
    assert_eq!(text.exit_code, 0);

    let json = queue_outcome("ses-demo-c1r1", QueueWait::Deadline, true);
    assert_eq!(json.message, r#"{"queued":true,"session":"ses-demo-c1r1"}"#);
    assert!(!json.to_stderr);
    assert_eq!(json.exit_code, 0);
}

/// A queued turn that happens to finish inside the wait prints its reply exactly as
/// the bare form does (text: the `text` member; JSON: the whole document).
#[test]
fn a_reply_inside_the_wait_prints_as_the_bare_form_does() {
    let doc = json!({"text": "all done", "state": "completed", "stop_reason": "end_turn"});
    let text = queue_outcome("ses-demo-c1r1", QueueWait::Reply(doc.clone()), false);
    assert_eq!(text.message, "all done");
    assert!(!text.to_stderr);
    assert_eq!(text.exit_code, 0);

    let json = queue_outcome("ses-demo-c1r1", QueueWait::Reply(doc.clone()), true);
    assert_eq!(json.message, doc.to_string());
    assert!(!json.to_stderr);
    assert_eq!(json.exit_code, 0);
}

// --- the CLI-side pane arm of a held refusal (binding 2) --------------------------

/// A `-32011 session_held` whose `data.hold_kind` is `"pane"` and whose `data.reason`
/// is the pane kebab code.
fn pane_held(code: &'static str) -> holler_proto::WireError {
    holler_proto::WireError::new(
        Code::SessionHeld,
        format!("demo-c1r1 refuses prompts ({code})"),
        Some(code),
    )
    .with_hold_kind("pane")
}

/// Binding 2: the pane arm renders before the generic `is_held` arm — one line naming
/// the pane code with the right remedy (`unpark` for a park, `doctor` otherwise),
/// exit 3 — never the generic "; ask whoever holds it to release it" tail and never
/// the held exit 4.
#[test]
fn a_pane_hold_renders_one_line_exit_3_with_the_right_remedy() {
    for (code, remedy) in [
        ("pane-parked", "holler pane unpark"),
        ("pane-unhealthy", "holler pane doctor"),
        ("pane-shown-driven-mismatch", "holler pane doctor"),
    ] {
        let text = pane_hold_refusal("ses-demo-c1r1", &pane_held(code), false)
            .unwrap_or_else(|| panic!("{code}: the pane arm renders"));
        assert_eq!(text.exit_code, 3, "{code}: exit 3, not the held 4");
        assert!(text.to_stderr, "{code}: a refusal goes to stderr");
        assert!(!text.message.contains('\n'), "{code}: one line");
        assert!(
            text.message.contains(code) && text.message.contains(remedy),
            "{code}: the message names the code and the remedy: {}",
            text.message
        );
        assert!(
            !text.message.contains("release"),
            "{code}: never the generic held tail: {}",
            text.message
        );
    }

    let json = pane_hold_refusal("ses-demo-c1r1", &pane_held("pane-parked"), true)
        .unwrap_or_else(|| panic!("json: the pane arm renders"));
    assert_eq!(json.exit_code, 3);
    assert!(
        !json.to_stderr,
        "json output goes to stdout, as the held form does"
    );
    assert_eq!(
        json.message,
        r#"{"error":"session_held","session":"ses-demo-c1r1","reason":"pane-parked","hold_kind":"pane"}"#
    );
}

/// Every other `-32011` (an operator hold, a default one, or one with no kind) falls
/// through to the generic held arm: the pane arm claims only `hold_kind == "pane"`.
#[test]
fn every_other_held_error_falls_through_to_the_generic_arm() {
    let mut operator = pane_held("freeze");
    if let Some(data) = operator.data.as_mut() {
        data.hold_kind = Some("operator".into());
    }
    assert!(pane_hold_refusal("ses-demo-c1r1", &operator, false).is_none());

    let mut unkinded = pane_held("freeze");
    if let Some(data) = unkinded.data.as_mut() {
        data.hold_kind = None;
    }
    assert!(pane_hold_refusal("ses-demo-c1r1", &unkinded, false).is_none());
}
