//! `holler profile rename`, `export` and `import` on the real binary against a real hub
//! (#665): the verbs reach the hub's real `profile/rename` handler and registry over the
//! control socket, not a stub. What each verb prints and refuses is pinned in-process
//! (`profile_verbs/{rename,export,import}.rs`); this file pins the wiring end to end.

use serde_json::Value;

use crate::LiveHub;

/// A profile document of D7 (#665), one spec, every field set.
const DOCUMENT: &str = r#"{
  "format": "holler-profile",
  "version": 1,
  "name": "Wired Fleet",
  "panes": [
    {
      "pane": "hlr665-c1r1",
      "herdr": {"workspace": "main", "grid": "c2r1"},
      "host": {"cwd": "/srv/demo"},
      "harness": {"kind": "opencode", "port_policy": "fixed:48195"},
      "model": {"provider": "demo-provider", "model_id": "demo-model", "effort": "medium"},
      "role": "agent",
      "env": ["DEMO_API_KEY"],
      "context": {"soft": 100000, "hard": 150000},
      "command": ["opencode", "serve"],
      "check": ["curl", "-s", "http://127.0.0.1:48195/v1/models"],
      "expect": ["demo-model"]
    }
  ]
}
"#;

#[test]
fn rename_on_an_empty_hub_reaches_the_real_handler() {
    let hub = LiveHub::start();
    let text = hub.holler(&["profile", "rename", "hlr665-a", "hlr665-b"]);
    assert_eq!(text.code, 3, "{text:?}");
    assert!(text.stdout.is_empty(), "{text:?}");
    assert!(
        text.stderr.lines().any(|l| l.starts_with("error: ")),
        "{text:?}"
    );
    let json = hub.holler(&["profile", "rename", "hlr665-a", "hlr665-b", "--format=json"]);
    assert_eq!(json.code, 3, "{json:?}");
    assert_eq!(
        json.envelope()["error"]["code"],
        "profile-not-found",
        "{json:?}"
    );
}

#[test]
fn import_export_and_rename_round_trip_through_the_hub() {
    let hub = LiveHub::start();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("in.json");
    std::fs::write(&file, DOCUMENT).unwrap();
    let out = dir.path().join("out.json");
    let (file, out) = (file.to_str().unwrap(), out.to_str().unwrap());

    let imported = hub.holler(&["profile", "import", file]);
    assert_eq!(imported.code, 0, "{imported:?}");
    let exported = hub.holler(&["profile", "export", "Wired Fleet", "--out", out]);
    assert_eq!(exported.code, 0, "{exported:?}");
    let parsed = |path: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    let mut want = parsed(file);
    want["panes"][0]["herdr"]["grid"] = Value::from("r1c2");
    assert_eq!(parsed(out), want, "export gives back the imported document");

    let renamed = hub.holler(&["profile", "rename", "Wired Fleet", "Wired Other"]);
    assert_eq!(renamed.code, 0, "{renamed:?}");
    let shown = hub.holler(&["profile", "show", "Wired Other", "--format=json"]);
    assert_eq!(shown.code, 0, "{shown:?}");
    assert_eq!(
        shown.envelope()["data"]["profile"]["name"],
        "Wired Other",
        "{shown:?}"
    );
    let gone = hub.holler(&["profile", "show", "Wired Fleet", "--format=json"]);
    assert_eq!(gone.envelope()["error"]["code"], "profile-not-found");
}
