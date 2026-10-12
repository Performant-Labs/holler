//! `holler profile export NAME [--out FILE]` (story #665, Scope B, D7 and D9): a profile as
//! one JSON document of its specs, `herdr.grid` written `rRcC`, with no slug, generation,
//! timestamp, log or environment value. Without `--out` the document is the result; with it,
//! the file is written atomically at mode 0600, replacing one that exists. Export writes
//! nothing to the stores.
//!
//! The document's text is pinned byte for byte (D7: two-space pretty print, the spec's own
//! key order, a trailing newline). [`full_spec`] and [`FULL_DOCUMENT`] are shared with
//! `import.rs` (the round trip).

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use holler_pane::pane::{ContextCeilings, HarnessKind, ModelSpec, PaneRole};
use holler_pane::profile::{SpecHarness, SpecHerdr, SpecHost};
use holler_pane::{AgentKey, Argv, EnvVarName, GridPos, ProfileSpec};
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::{json, Value};

use crate::list::rig::{assert_failure, profile, run_both, run_both_with, Rig};

/// A made-up value of the environment variable the full spec names: it must never be read
/// into a document or printed.
pub(crate) const CANARY_VAR: &str = "HOLLER_T665_EXPORT_CANARY";
pub(crate) const CANARY_VALUE: &str = "t665-canary-value-not-a-secret-8d1e";

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

/// A spec with every field set, at `r<row>c<col>`, naming the env variable [`CANARY_VAR`].
pub(crate) fn full_spec(pane: &str, row: u16, col: u16) -> ProfileSpec {
    ProfileSpec {
        pane: pane.to_owned(),
        herdr: SpecHerdr {
            workspace: "main".to_owned(),
            grid: GridPos { row, col },
        },
        host: SpecHost {
            cwd: "/srv/demo".to_owned(),
        },
        harness: SpecHarness {
            kind: HarnessKind::Opencode,
            port_policy: "fixed:8095".to_owned(),
        },
        model: ModelSpec {
            provider: "demo-provider".to_owned(),
            model_id: "demo-model".to_owned(),
            effort: "medium".to_owned(),
        },
        opencode_agent: Some(AgentKey::parse("build").unwrap()),
        role: PaneRole::Agent,
        env: vec![
            EnvVarName::parse("DEMO_API_KEY").unwrap(),
            EnvVarName::parse(CANARY_VAR).unwrap(),
        ],
        context: ContextCeilings {
            soft: 100_000,
            hard: 150_000,
        },
        command: Some(argv(&["opencode", "serve"])),
        check: Some(argv(&["curl", "-s", "http://127.0.0.1:8095/v1/models"])),
        expect: vec!["demo-model".to_owned()],
    }
}

/// [`full_spec`] without the three optional members (`opencode_agent`, `command`, `check`)
/// and with no env and no expect.
pub(crate) fn bare_spec(pane: &str) -> ProfileSpec {
    ProfileSpec {
        opencode_agent: None,
        env: Vec::new(),
        command: None,
        check: None,
        expect: Vec::new(),
        ..full_spec(pane, 1, 1)
    }
}

/// The profile `Fleet` with one [`full_spec`] at `r2c3`.
pub(crate) fn fleet() -> Rig {
    Rig::new([], [profile("Fleet", vec![full_spec("demo-c3r2", 2, 3)])])
}

/// The export of [`fleet`], byte for byte.
pub(crate) const FULL_DOCUMENT: &str = r#"{
  "format": "holler-profile",
  "version": 1,
  "name": "Fleet",
  "panes": [
    {
      "pane": "demo-c3r2",
      "herdr": {
        "workspace": "main",
        "grid": "r2c3"
      },
      "host": {
        "cwd": "/srv/demo"
      },
      "harness": {
        "kind": "opencode",
        "port_policy": "fixed:8095"
      },
      "model": {
        "provider": "demo-provider",
        "model_id": "demo-model",
        "effort": "medium"
      },
      "opencode_agent": "build",
      "role": "agent",
      "env": [
        "DEMO_API_KEY",
        "HOLLER_T665_EXPORT_CANARY"
      ],
      "context": {
        "soft": 100000,
        "hard": 150000
      },
      "command": [
        "opencode",
        "serve"
      ],
      "check": [
        "curl",
        "-s",
        "http://127.0.0.1:8095/v1/models"
      ],
      "expect": [
        "demo-model"
      ]
    }
  ]
}
"#;

/// The export of a profile `Bare` with one [`bare_spec`] at `r1c1`: the optional members
/// are left out, `env` and `expect` are always present.
const BARE_DOCUMENT: &str = r#"{
  "format": "holler-profile",
  "version": 1,
  "name": "Bare",
  "panes": [
    {
      "pane": "demo-c1r1",
      "herdr": {
        "workspace": "main",
        "grid": "r1c1"
      },
      "host": {
        "cwd": "/srv/demo"
      },
      "harness": {
        "kind": "opencode",
        "port_policy": "fixed:8095"
      },
      "model": {
        "provider": "demo-provider",
        "model_id": "demo-model",
        "effort": "medium"
      },
      "role": "agent",
      "env": [],
      "context": {
        "soft": 100000,
        "hard": 150000
      },
      "expect": []
    }
  ]
}
"#;

/// No write reached either store.
fn assert_no_store_write(rig: &Rig) {
    let writes = [
        ProfileStoreOp::CasPut,
        ProfileStoreOp::Delete,
        ProfileStoreOp::Rename,
    ];
    let calls = rig.profiles.faults().calls();
    assert!(
        !calls.iter().any(|op| writes.contains(op)),
        "export writes no profile: {calls:?}"
    );
    let calls = rig.panes.faults().calls();
    assert!(
        calls
            .iter()
            .all(|op| [PaneStoreOp::List, PaneStoreOp::Get].contains(op)),
        "export writes no pane: {calls:?}"
    );
}

fn dir_listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn export_without_out_prints_the_document_in_both_formats() {
    let ran = run_both_with(fleet, &["profile", "export", "Fleet"], Rig::run);
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert!(both.text.err.is_empty(), "{:?}", both.text);
    assert_eq!(
        both.text.out, FULL_DOCUMENT,
        "the document is the whole stdout"
    );
    let data = &both.envelope.data;
    let document: Value = serde_json::from_str(FULL_DOCUMENT).unwrap();
    assert_eq!(data["document"], document);
    assert_eq!(data["out"], Value::Null);
    assert_eq!(
        (&data["name"], &data["slug"], &data["specs"]),
        (&json!("Fleet"), &json!("fleet"), &json!(1))
    );
    for rig in ran.rigs() {
        assert_no_store_write(rig);
    }
}

#[test]
fn export_leaves_out_absent_optional_members() {
    let seed = || Rig::new([], [profile("Bare", vec![bare_spec("demo-c1r1")])]);
    let both = run_both(seed, &["profile", "export", "Bare"]);
    assert_eq!(both.text.out, BARE_DOCUMENT);
}

#[test]
fn export_writes_every_position_as_rowcol_and_no_record_fields() {
    let cells = [(1, 1), (2, 1), (1, 2), (3, 12)];
    let specs: Vec<ProfileSpec> = cells
        .iter()
        .enumerate()
        .map(|(i, &(row, col))| full_spec(&format!("demo-p{i}"), row, col))
        .collect();
    let seed = move || Rig::new([], [profile("Grid", specs.clone())]);
    let both = run_both(seed, &["profile", "export", "Grid"]);
    let document: Value = serde_json::from_str(&both.text.out).unwrap();
    let grids: Vec<&Value> = document["panes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| &p["herdr"]["grid"])
        .collect();
    assert_eq!(
        grids,
        [
            &json!("r1c1"),
            &json!("r2c1"),
            &json!("r1c2"),
            &json!("r3c12")
        ]
    );
    let top: Vec<&str> = document
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(top, ["format", "version", "name", "panes"], "specs only");
    for field in ["slug", "generation", "created", "updated", "log"] {
        assert!(!both.text.out.contains(&format!("\"{field}\"")), "{field}");
    }
}

#[test]
fn export_never_reads_an_environment_value() {
    // The profile names CANARY_VAR, and this process has it set: a verb that resolved the
    // environment would leak the value into the document or a message.
    std::env::set_var(CANARY_VAR, CANARY_VALUE);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fleet.json");
    let out_text = out.to_str().unwrap();
    for args in [
        &["profile", "export", "Fleet"][..],
        &["profile", "export", "Fleet", "--out", out_text],
    ] {
        let both = run_both(fleet, args);
        for text in [
            &both.text.out,
            &both.text.err,
            &both.json.out,
            &both.json.err,
        ] {
            assert!(
                !text.contains(CANARY_VALUE),
                "{args:?} leaked the value: {text}"
            );
        }
    }
    let written = std::fs::read_to_string(&out).unwrap();
    assert!(!written.contains(CANARY_VALUE), "the file holds names only");
    assert!(written.contains(CANARY_VAR), "the name is kept");
}

#[test]
fn export_out_writes_the_file_at_0600_and_replaces_one_that_exists() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fleet.json");
    std::fs::write(&out, "an older export\n").unwrap();
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o644)).unwrap();
    let out_text = out.to_str().unwrap().to_owned();
    let ran = run_both_with(
        fleet,
        &["profile", "export", "Fleet", "--out", &out_text],
        Rig::run,
    );
    let both = &ran.both;
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(
        both.text.out,
        format!("exported profile Fleet (fleet) to {out_text}: 1 spec\n")
    );
    let data = &both.envelope.data;
    assert_eq!(data["out"], json!(out_text));
    assert_eq!(data["document"], Value::Null, "the document is in the file");
    assert_eq!(data["specs"], json!(1));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), FULL_DOCUMENT);
    let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the file is private");
    assert_eq!(
        dir_listing(dir.path()),
        ["fleet.json"],
        "no temporary file is left"
    );
    for rig in ran.rigs() {
        assert_no_store_write(rig);
    }
}

#[test]
fn export_to_a_missing_directory_is_usage_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("missing").join("fleet.json");
    let out_text = out.to_str().unwrap();
    let both = run_both(fleet, &["profile", "export", "Fleet", "--out", out_text]);
    assert_failure(&both, "usage", 2);
    assert!(!dir.path().join("missing").exists());
    assert_eq!(dir_listing(dir.path()), Vec::<String>::new());
}

#[test]
fn export_of_a_missing_profile_is_profile_not_found() {
    let both = run_both(fleet, &["profile", "export", "Missing"]);
    assert_failure(&both, "profile-not-found", 3);
}

#[test]
fn export_prints_a_control_character_in_a_path_escaped() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fleet\u{1b}[2J.json");
    let out_text = out.to_str().unwrap();
    let both = run_both(fleet, &["profile", "export", "Fleet", "--out", out_text]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert!(!both.text.out.contains('\u{1b}'), "{:?}", both.text.out);
    assert!(out.exists(), "the file is written under its real name");
}
