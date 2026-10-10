//! `holler pane get` (story #643): one pane in full, and the terminal-safety case of the
//! three read verbs' text output. The rig and the shared helpers are `crate::list`'s.

use clap::error::ErrorKind;
use clap::Parser;
use holler_cli::output::Format;
use holler_cli::Cli;
use holler_pane::pane::{ContextCeilings, Health, Hold, ModelSpec, PaneProbe};
use holler_pane::{Argv, EnvVarName, Pane, ProbeResult};
use holler_pane_testkit::envelope::{check_envelope, check_ndjson};
use holler_pane_testkit::fixture::sample_profile;
use serde_json::{json, Value};

use crate::list::{
    assert_fails, field, member, ok_envelope, ok_text, pane, scoped_rig, sync_rig, Rig,
};

/// The rendered `--help` of `holler <argv...>` (clap's `DisplayHelp` error).
pub(crate) fn help(argv: &[&str]) -> String {
    let mut full = vec!["holler"];
    full.extend_from_slice(argv);
    full.push("--help");
    match Cli::try_parse_from(&full) {
        Err(e) if e.kind() == ErrorKind::DisplayHelp => e.to_string(),
        Err(e) => panic!("{full:?}: want DisplayHelp, got {:?}: {e}", e.kind()),
        Ok(_) => panic!("{full:?}: want DisplayHelp, but it parsed"),
    }
}

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

/// The pane of AC 7: every optional field set, in the profile `demo`.
fn full_pane() -> Pane {
    let mut full = member("demo-c1r1", "demo");
    full.model = ModelSpec {
        provider: "demo-provider".into(),
        model_id: "demo-model".into(),
        effort: "high".into(),
    };
    full.env = ["ALPHA_TOKEN", "BETA_URL"]
        .map(|n| EnvVarName::parse(n).unwrap())
        .to_vec();
    full.context = ContextCeilings {
        soft: 64_000,
        hard: 96_000,
    };
    full.command = Some(argv(&["opencode", "serve", "--port", "48100"]));
    full.probe = PaneProbe {
        check: Some(argv(&["curl", "-s", "http://127.0.0.1:48100/health"])),
        expect: vec!["ok".into()],
        last: Some(ProbeResult::Failed {
            missing: vec!["ok".into()],
        }),
    };
    full.session_of_record = Some("ses-a".into());
    full.hold = Hold::Parked {
        reason: "maintenance".into(),
        release_when: "after the deploy".into(),
        since: 0,
    };
    full
}

#[test]
fn get_shows_every_field_of_the_record() {
    let profile = sample_profile("demo", &["demo-c1r1"]).unwrap();
    let rig = Rig::new([full_pane()], [profile.clone()]).unwrap();
    let run = rig.run(&["pane", "get", "demo-c1r1"], Format::Text);
    let out = ok_text(&run);
    for line in [
        "pane: demo-c1r1",
        "profile: demo",
        "model: demo-provider/demo-model",
        "effort: high",
        "env: ALPHA_TOKEN BETA_URL",
        "context: soft=64000 hard=96000",
        r#"command: ["opencode","serve","--port","48100"]"#,
        r#"probe-check: ["curl","-s","http://127.0.0.1:48100/health"]"#,
        r#"probe-expect: ["ok"]"#,
        r#"probe-last: failed missing=["ok"]"#,
        "session-of-record: ses-a",
        r#"hold: parked reason=maintenance until="after the deploy" since=never"#,
    ] {
        assert!(
            out.lines().any(|l| l == line),
            "a whole line {line:?} in:\n{out}"
        );
    }

    let stored = Pane {
        generation: 1,
        ..full_pane()
    };
    let spec = serde_json::to_value(&profile.panes[0]).unwrap();
    for argv in [
        &["pane", "get", "demo-c1r1"][..],
        &["pane", "get", "demo-c1r1", "--profile", "demo"],
    ] {
        let data = ok_envelope(&rig.run(argv, Format::Json)).data;
        assert_eq!(
            data["pane"],
            serde_json::to_value(&stored).unwrap(),
            "{argv:?}: the record verbatim"
        );
        assert_eq!(data["profile"], "demo", "{argv:?}: {data}");
        assert_eq!(
            data["spec"], spec,
            "{argv:?}: the profile's spec for the pane"
        );
        assert!(data["spec"]["context"]["soft"].is_u64(), "{data}");
        assert!(data["spec"]["context"]["hard"].is_u64(), "{data}");
        assert!(data.get("sync").is_some(), "{argv:?}: data.sync: {data}");
    }
}

#[test]
fn get_pane_without_a_profile_shows_its_ceilings() {
    let rig = Rig::new([pane("demo-c1r1")], []).unwrap();
    let run = rig.run(&["pane", "get", "demo-c1r1"], Format::Text);
    let out = ok_text(&run);
    assert_eq!(field(out, "profile"), Some("-"), "{out}");
    assert_eq!(field(out, "spec"), Some("-"), "{out}");
    assert_eq!(
        field(out, "context"),
        Some("soft=100000 hard=150000"),
        "{out}"
    );

    let data = ok_envelope(&rig.run(&["pane", "get", "demo-c1r1"], Format::Json)).data;
    assert_eq!(data["profile"], Value::Null, "{data}");
    assert_eq!(data["spec"], Value::Null, "{data}");
    assert!(
        data.get("profile").is_some() && data.get("spec").is_some(),
        "always present: {data}"
    );
    assert_eq!(
        data["pane"]["context"],
        json!({"soft": 100_000, "hard": 150_000}),
        "{data}"
    );
}

#[test]
fn get_profile_member_is_shown() {
    let rig = scoped_rig();
    let argv = ["pane", "get", "demo-c1r1", "--profile", "demo"];
    let run = rig.run(&argv, Format::Text);
    assert_eq!(field(ok_text(&run), "pane"), Some("demo-c1r1"), "{run:?}");
    let data = ok_envelope(&rig.run(&argv, Format::Json)).data;
    assert_eq!(data["pane"]["name"], "demo-c1r1", "{data}");
    assert_eq!(data["profile"], "demo", "{data}");
}

#[test]
fn get_flags_a_mismatch() {
    let rig = sync_rig();
    for (name, text, json) in [
        ("demo-c1r1", "MISMATCH", "mismatch"),
        ("demo-c2r1", "ok", "ok"),
        ("demo-c3r1", "-", "unobserved"),
    ] {
        let run = rig.run(&["pane", "get", name], Format::Text);
        assert_eq!(field(ok_text(&run), "sync"), Some(text), "{name}: {run:?}");
        let data = ok_envelope(&rig.run(&["pane", "get", name], Format::Json)).data;
        assert_eq!(data["sync"], json, "{name}: {data}");
    }
}

#[test]
fn get_missing_pane_is_pane_not_found_in_both_formats() {
    let rig = Rig::new([], []).unwrap();
    assert_fails(
        &rig,
        &["pane", "get", "demo-c9r9"],
        3,
        "pane-not-found",
        false,
    );
    let text = rig.run(&["pane", "get", "demo-c9r9"], Format::Text);
    assert_eq!(text.err, "error: pane not found: demo-c9r9\n", "{text:?}");
}

#[test]
fn get_requires_a_pane_name() {
    let parsed = Cli::try_parse_from(["holler", "pane", "get"]).map(|_| ());
    let kind = parsed.as_ref().map_err(clap::Error::kind);
    assert_eq!(kind, Err(ErrorKind::MissingRequiredArgument), "{parsed:?}");
}

#[test]
fn get_help_documents_the_json_shape() {
    let help = help(&["pane", "get"]);
    for needle in ["\"pane\"", "\"profile\"", "\"spec\"", "\"sync\"", "context"] {
        assert!(
            help.contains(needle),
            "`pane get --help` names {needle:?}:\n{help}"
        );
    }
}

/// A pane whose stored strings hold an escape sequence, a newline and a carriage return.
fn hostile(cwd: &str, reason: &str) -> Pane {
    let mut bad = pane("demo-c1r1");
    bad.host.cwd = cwd.into();
    bad.harness.health = Health::Unhealthy(reason.into());
    bad
}

#[test]
fn text_output_escapes_control_characters() {
    let rig = Rig::new([hostile("/srv/demo\u{1b}[31mred\nfake", "bad\rline")], []).unwrap();
    let clean = Rig::new([hostile("/srv/demo", "bad")], []).unwrap();
    let escaped = r#""/srv/demo\u{1b}[31mred\nfake""#;
    let get = ["pane", "get", "demo-c1r1"];
    let get_lines = ok_text(&clean.run(&get, Format::Text))
        .matches('\n')
        .count();
    let cases: [(&[&str], usize); 3] = [
        (&["pane", "list"], 2),
        (&get, get_lines),
        (&["pane", "watch", "--until-idle"], 1),
    ];
    for (argv, newlines) in cases {
        let run = rig.run(argv, Format::Text);
        let out = ok_text(&run);
        assert!(
            !out.contains('\u{1b}'),
            "{argv:?}: no ESC reaches the terminal: {out:?}"
        );
        assert!(!out.contains('\r'), "{argv:?}: no carriage return: {out:?}");
        assert_eq!(
            out.matches('\n').count(),
            newlines,
            "{argv:?}: no forged line: {out:?}"
        );
        assert!(
            out.contains(escaped),
            "{argv:?}: the project escaped as {escaped}: {out:?}"
        );

        let json = rig.run(argv, Format::Json);
        assert_eq!(json.code, 0, "{argv:?}: {json:?}");
        let checked = if argv[1] == "watch" {
            check_ndjson(&json.out, 0).map(|_| ())
        } else {
            check_envelope(&json.out, 0).map(|_| ())
        };
        assert_eq!(checked, Ok(()), "{argv:?}: {json:?}");
    }
}

/// Beyond C0: U+009B is a one-character CSI on some terminals, and U+202E reorders the
/// line. Neither reaches the terminal raw, in a stored string or in an argv printed as JSON.
#[test]
fn text_output_escapes_c1_and_bidi_characters() {
    let mut bad = pane("demo-c1r1");
    bad.host.cwd = "/srv/demo\u{202e}x".into();
    bad.command = Some(argv(&["opencode", "a\u{9b}b", "x\u{202e}y"]));
    let rig = Rig::new([bad], []).unwrap();
    let run = rig.run(&["pane", "get", "demo-c1r1"], Format::Text);
    let out = ok_text(&run);
    for c in ['\u{9b}', '\u{202e}'] {
        assert!(
            !out.contains(c),
            "no raw {c:?} reaches the terminal: {out:?}"
        );
    }
    assert_eq!(
        field(out, "project"),
        Some(r#""/srv/demo\u{202e}x""#),
        "{out:?}"
    );
    assert_eq!(
        field(out, "command"),
        Some(r#"["opencode","a\u009bb","x\u202ey"]"#),
        "still JSON, the same argv: {out:?}"
    );
}
