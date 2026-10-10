//! Every stub verb of `holler pane` and `holler profile`, on the real binary
//! (`stub_verb_not_implemented`), plus the bare-namespace, `--help` and policy-refusal
//! behaviour that `main.rs` keeps.

use serde_json::Value;

use crate::{assert_no_failures, holler, PANE_VERBS, PROFILE_VERBS};

/// Every verb that is still a stub, with the story that owns it. The only place in the
/// shared process tests that names a stub's owning story.
///
/// Grouped by story, each group under its own `// #NNN` comment line. A verb story
/// deletes its own entries when its verb stops being a stub and **keeps its `// #NNN`
/// line**: two stories that delete whole groups, header included, delete adjacent lines,
/// and git reports that as a conflict. (`PANE_VERBS` and `PROFILE_VERBS` keep the verb
/// itself.)
pub const STUBS: &[(&str, &str, u32)] = &[
    // #643
    // #644
    ("pane", "launch", 644),
    ("pane", "relaunch", 644),
    // #645
    ("pane", "switch", 645),
    ("pane", "reset", 645),
    // #646
    ("pane", "park", 646),
    ("pane", "unpark", 646),
    ("pane", "close", 646),
    // #647
    // #650
    ("pane", "import", 650),
    // #662
    ("profile", "create", 662),
    ("profile", "delete", 662),
    // #664
    ("profile", "apply", 664),
    // #665 (proposed: the operator confirms it)
    ("profile", "rename", 665),
    ("profile", "export", 665),
    ("profile", "import", 665),
];

/// The refusal of `say`/`interrupt`/`answer` with `--pane` or `--profile`, until story #646.
/// That story deletes the constant and keeps its `// #646` line and the blank line below,
/// so the two stories' deletions do not touch adjacent lines.
// #646
pub const PANE_FORM_REFUSAL: &str = "error: not implemented (story #646)";

/// The refusal of `roster --profile`, until story #648.
// #648
pub const ROSTER_PROFILE_REFUSAL: &str = "error: not implemented (story #648)";

/// The table and the permanent verb lists agree: every stub is a known verb, and a verb
/// missing from the table is simply no longer a stub (so this checks only the first half).
#[test]
fn every_stub_names_a_known_verb() {
    for &(namespace, verb, _) in STUBS {
        let verbs = if namespace == "pane" {
            PANE_VERBS
        } else {
            PROFILE_VERBS
        };
        assert!(
            verbs.contains(&verb),
            "`{namespace} {verb}` is in STUBS but not in the verb list"
        );
    }
}

/// Text mode: `error: not implemented (story #NNN)` on stderr, nothing on stdout, exit 1.
#[test]
fn stub_verb_not_implemented() {
    let mut failures = Vec::new();
    for &(namespace, verb, story) in STUBS {
        let out = holler(&[namespace, verb]);
        let line = format!("error: not implemented (story #{story})");
        if out.code != 1 || !out.stdout.is_empty() || !out.stderr_has_line(&line) {
            failures.push(format!("holler {namespace} {verb}: want exit 1, empty stdout, stderr line {line:?}; got {out:?}"));
        }
    }
    assert_no_failures(failures);
}

/// JSON mode, in each spelling (`--format=json`, `--format json`, `--json`, and the
/// flag before the namespace): exactly one envelope on stdout and nothing else, exit 1.
#[test]
fn stub_verb_not_implemented_json_is_one_envelope() {
    let mut failures = Vec::new();
    for &(namespace, verb, story) in STUBS {
        let phrase = format!("not implemented (story #{story})");
        let spellings: [Vec<&str>; 4] = [
            vec![namespace, verb, "--format=json"],
            vec![namespace, verb, "--format", "json"],
            vec![namespace, verb, "--json"],
            vec!["--format=json", namespace, verb],
        ];
        for argv in spellings {
            let out = holler(&argv);
            let lines: Vec<&str> = out.stdout.lines().collect();
            let compact_prefix = r#"{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"#;
            let parsed: Option<Value> = (lines.len() == 1)
                .then(|| serde_json::from_str(lines[0]).ok())
                .flatten();
            let message_ok = parsed
                .as_ref()
                .and_then(|v| v["error"]["message"].as_str())
                .is_some_and(|m| m.contains(&phrase) && !m.contains('\n'));
            let shape_ok = parsed.as_ref().is_some_and(|v| {
                v["schema_version"] == 1
                    && v["ok"] == false
                    && v["data"] == Value::Null
                    && v["error"]["code"] == "not-implemented"
            });
            if out.code != 1
                || !out.stdout.ends_with('\n')
                || !lines.first().is_some_and(|l| l.starts_with(compact_prefix))
                || !message_ok
                || !shape_ok
            {
                failures.push(format!("holler {argv:?}: want exit 1 and one not-implemented envelope naming {phrase:?}; got {out:?}"));
            }
        }
    }
    assert_no_failures(failures);
}

/// A bare namespace is a usage error: clap's message on stderr, exit 2, nothing on stdout.
#[test]
fn a_bare_namespace_is_a_usage_error() {
    for namespace in ["pane", "profile"] {
        let out = holler(&[namespace]);
        assert_eq!(out.code, 2, "{out:?}");
        assert!(out.stdout.is_empty(), "{out:?}");
        assert!(out.stderr.contains("Usage"), "{out:?}");
        // The namespace exists: clap says its subcommand is missing, not that it is unknown,
        // and its usage lists the verbs.
        assert!(
            !out.stderr.contains("unrecognized subcommand"),
            "`{namespace}` must be a known namespace: {out:?}"
        );
        assert!(
            out.stderr.contains("list"),
            "the usage names the verbs: {out:?}"
        );
    }
}

/// `--help` lists every verb and is a success (exit 0), even when `--json` or
/// `--format=json` is also given: help is not an error to turn into an envelope.
#[test]
fn help_lists_every_verb_and_exits_0() {
    for (namespace, verbs) in [("pane", PANE_VERBS), ("profile", PROFILE_VERBS)] {
        for flags in [
            &["--help"][..],
            &["--help", "--json"],
            &["--help", "--format=json"],
        ] {
            let mut argv = vec![namespace];
            argv.extend_from_slice(flags);
            let out = holler(&argv);
            assert_eq!(out.code, 0, "{argv:?}: {out:?}");
            for &verb in verbs {
                assert!(
                    out.stdout.contains(verb),
                    "{argv:?}: help names `{verb}`: {}",
                    out.stdout
                );
            }
        }
    }
    let out = holler(&["pane", "list", "--help", "--format=json"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(out.stdout.contains("Usage"), "{out:?}");
}

/// The top-level help names the two new namespaces; `--version` is unchanged.
#[test]
fn top_level_help_names_pane_and_profile() {
    let out = holler(&["--help"]);
    assert_eq!(out.code, 0);
    for word in ["pane", "profile"] {
        assert!(
            out.stdout.contains(word),
            "--help names `{word}`: {}",
            out.stdout
        );
    }
    let version = holler(&["--version"]);
    assert_eq!(version.code, 0);
    assert_eq!(
        version.stdout,
        format!("holler {}\n", env!("CARGO_PKG_VERSION"))
    );
}

/// Exit 3 (the fail-closed policy refusal) is still decided before dispatch, in plain
/// text under every format: no stub runs and nothing is written to stdout.
#[test]
fn a_bad_debug_value_is_still_a_policy_refusal_before_dispatch() {
    for argv in [
        &["pane", "list", "--debug", "bogus"][..],
        &["pane", "list", "--debug", "bogus", "--format=json"],
        &["profile", "list", "--debug", "bogus", "--json"],
    ] {
        let out = holler(argv);
        assert_eq!(out.code, 3, "{argv:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{argv:?}: {out:?}");
        assert!(out.stderr.contains("error:"), "{argv:?}: {out:?}");
        assert!(
            !out.stderr.contains("not implemented"),
            "{argv:?}: the stub must not have run: {out:?}"
        );
    }
}
