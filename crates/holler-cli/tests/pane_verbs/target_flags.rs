//! `say`, `interrupt` and `answer` with `--pane NAME` (story #670, decision 5).
//!
//! SESSION, TEXT and CHOICE are one optional tail, resolved in code behind
//! `resolve()` (the `Query::resolve` pattern), so `say --pane NAME TEXT` parses while
//! every existing form keeps resolving to the same session and text. The accessor
//! returns the existing `holler_cli::Usage` for a missing or surplus positional.
//!
//! These assert the resolved target and text **by value**. The refusal of `--pane` and
//! `--profile` (exit 1, story #646) is tested on the real binary in `pane_cli_process`.

use clap::Parser;
use holler_cli::prompt_target::{PromptArgs, PromptTarget};
use holler_cli::{Cli, Command, Usage};

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
