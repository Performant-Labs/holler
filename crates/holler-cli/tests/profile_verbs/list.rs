//! `holler profile list` (story #662, AC 4): name, slug, spec count, live count and
//! generation of every profile, sorted by slug. Also declares the shared rig.

#[path = "rig.rs"]
pub(crate) mod rig;

use serde_json::json;

use rig::{matching_spec, member, pane, profile, run_both, Rig};

/// `Some Profile` with two specs and one member, and the empty `Alpha`, seeded in the
/// reverse of slug order; plus a pane in no profile.
fn two_profiles() -> Rig {
    Rig::new(
        [member("demo-c1r1", "Some Profile"), pane("demo-c3r1")],
        [
            profile(
                "Some Profile",
                vec![matching_spec("demo-c1r1"), matching_spec("demo-c2r1")],
            ),
            profile("Alpha", vec![]),
        ],
    )
}

#[test]
fn list_reports_name_slug_panes_live_generation_in_both_formats() {
    let both = run_both(two_profiles, &["profile", "list"]);

    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert!(both.text.err.is_empty(), "{:?}", both.text);
    assert_eq!(
        both.text.out,
        "Alpha (alpha): 0 panes, 0 live, generation 1\n\
         Some Profile (some-profile): 2 panes, 1 live, generation 1\n"
    );

    assert_eq!(
        both.envelope.data,
        json!([
            {"name": "Alpha", "slug": "alpha", "panes": 0, "live": 0, "generation": 1},
            {"name": "Some Profile", "slug": "some-profile", "panes": 2, "live": 1, "generation": 1},
        ])
    );
    // A derived struct: the key order is the struct's in every build.
    assert!(
        both.json
            .out
            .contains(r#"{"name":"Alpha","slug":"alpha","panes":0,"live":0,"generation":1}"#),
        "{}",
        both.json.out
    );
}

#[test]
fn list_counts_members_by_slug() {
    let seed = || {
        Rig::new(
            [
                member("demo-c1r1", "SOME-PROFILE"),
                member("demo-c2r1", "Other"),
                pane("demo-c3r1"),
            ],
            [
                profile("Some Profile", vec![matching_spec("demo-c1r1")]),
                profile("Other", vec![]),
            ],
        )
    };
    let both = run_both(seed, &["profile", "list"]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    let live: Vec<(String, u64)> = both
        .envelope
        .data
        .as_array()
        .expect("data is an array")
        .iter()
        .map(|row| {
            (
                row["slug"].as_str().unwrap().to_owned(),
                row["live"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        live,
        vec![("other".to_owned(), 1), ("some-profile".to_owned(), 1)]
    );
    let line = both
        .text
        .out
        .lines()
        .find(|l| l.starts_with("Some Profile (some-profile): "))
        .unwrap_or_else(|| panic!("a line for Some Profile: {:?}", both.text));
    assert!(line.contains(", 1 live, "), "{line:?}");
}

#[test]
fn list_of_no_profiles() {
    let both = run_both(|| Rig::new([pane("demo-c1r1")], []), &["profile", "list"]);
    assert_eq!(both.text.code, 0, "{:?}", both.text);
    assert_eq!(both.text.out, "no profiles\n");
    assert_eq!(both.envelope.data, json!([]));
}
