#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! The three name types (#637 AC 6 table tests): `PaneName` (the ADR 0005 session-name
//! grammar, reused not copied), `ProfileName` (a display name with the one `slug()`),
//! and `Actor` (who made a profile write).

use holler_pane::{Actor, PaneName, ProfileName};
use holler_proto::vocab::SessionName;

#[test]
fn pane_name_grammar() {
    let long_ok = "a".repeat(32);
    let too_long = "a".repeat(33);
    let cases = [
        // Valid session names, including the real fleet's.
        "hj-c1r1",
        "hj-c1r2",
        "c2r2",
        "a",
        "a1",
        "a-b-c",
        "a--b",
        "0",
        long_ok.as_str(),
        // Invalid: empty, separators at an end, case, punctuation, whitespace, slash, length, non-ASCII.
        "",
        "-a",
        "a-",
        "-",
        "A",
        "Hj-c1r1",
        "a_b",
        "a.b",
        "a b",
        " a",
        "a ",
        "a/b",
        "/a",
        too_long.as_str(),
        "é",
        "a\n",
    ];

    for s in cases {
        // The grammar is `SessionName`'s: PaneName must agree with it on every input.
        assert_eq!(
            PaneName::parse(s).is_ok(),
            SessionName::parse(s).is_ok(),
            "PaneName and SessionName disagree on {s:?}"
        );
    }

    let name = PaneName::parse("hj-c1r1").unwrap();
    assert_eq!(name.as_str(), "hj-c1r1");
    assert_eq!(name.to_string(), "hj-c1r1");
    assert!(PaneName::parse("Not A Name").is_err());
}

#[test]
fn pane_name_serde_goes_through_the_grammar() {
    let name: PaneName = serde_json::from_str(r#""hj-c1r1""#).unwrap();
    assert_eq!(name.as_str(), "hj-c1r1");
    assert_eq!(serde_json::to_string(&name).unwrap(), r#""hj-c1r1""#);

    for bad in [r#""Hj C1R1""#, r#""""#, r#""a/b""#, "7", "null"] {
        assert!(serde_json::from_str::<PaneName>(bad).is_err(), "{bad}");
    }
}

#[test]
fn pane_name_is_a_usable_map_key() {
    use std::collections::{BTreeSet, HashSet};
    let a = PaneName::parse("hj-c1r1").unwrap();
    let b = PaneName::parse("hj-c1r2").unwrap();
    let hashed: HashSet<PaneName> = [a.clone(), b.clone(), a.clone()].into_iter().collect();
    assert_eq!(hashed.len(), 2);
    let ordered: BTreeSet<PaneName> = [b.clone(), a.clone()].into_iter().collect();
    assert_eq!(ordered.into_iter().next(), Some(a));
}

#[test]
fn profile_name_slug() {
    #[rustfmt::skip]
    let table = [
        ("Some Profile", "Some Profile", "some-profile"),
        ("Fleet", "Fleet", "fleet"),
        ("fleet", "fleet", "fleet"),
        ("a  b", "a  b", "a-b"),
        ("A_B.C", "A_B.C", "a-b-c"),
        ("x1 y2", "x1 y2", "x1-y2"),
        ("My Profile (v2)", "My Profile (v2)", "my-profile-v2"),
        // Leading and trailing separators are trimmed, in the name and in the slug.
        ("  Padded  ", "Padded", "padded"),
        ("--Hello, World!--", "--Hello, World!--", "hello-world"),
        ("!!a!!", "!!a!!", "a"),
        ("a - b", "a - b", "a-b"),
    ];

    for (input, display, slug) in table {
        let name = ProfileName::parse(input).unwrap_or_else(|e| panic!("{input:?}: {e}"));
        assert_eq!(name.as_str(), display, "display form of {input:?}");
        assert_eq!(name.slug(), slug, "slug of {input:?}");
        // The slug is stable: slugging is idempotent through a name parse.
        assert_eq!(ProfileName::parse(&name.slug()).unwrap().slug(), slug);
    }
}

#[test]
fn profile_names_that_differ_but_share_a_slug_have_the_same_slug() {
    // #661 unique-checks the slug, so two spellings of one slug must be detectable.
    let names = [
        "Some Profile",
        "some profile",
        "SOME-PROFILE",
        "some_profile",
        " Some  Profile ",
    ];
    let parsed: Vec<ProfileName> = names
        .iter()
        .map(|n| ProfileName::parse(n).unwrap())
        .collect();
    for p in &parsed {
        assert_eq!(p.slug(), "some-profile");
    }
    assert_ne!(parsed[0], parsed[1], "the display names stay distinct");
    assert_ne!(parsed[0].as_str(), parsed[2].as_str());
}

#[test]
fn profile_name_refusals() {
    let sixty_four = "a".repeat(64);
    let sixty_five = "a".repeat(65);

    for ok in ["a", "Some Profile", sixty_four.as_str()] {
        assert!(
            ProfileName::parse(ok).is_ok(),
            "{ok:?} is a valid profile name"
        );
    }

    for bad in [
        "",
        " ",
        "\t\n",
        // No alphanumerics: the slug would be empty, and the slug is persisted.
        "!!!",
        "---",
        "_ _",
        "  - . -  ",
        // Control characters.
        "a\u{7}b",
        "a\nb",
        "tab\there",
        "nul\u{0}",
        sixty_five.as_str(),
    ] {
        assert!(ProfileName::parse(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn profile_name_serde_is_a_plain_string_through_the_same_checks() {
    let name: ProfileName = serde_json::from_str(r#""Some Profile""#).unwrap();
    assert_eq!(serde_json::to_string(&name).unwrap(), r#""Some Profile""#);
    for bad in [r#""""#, r#""!!!""#, "5", "null"] {
        assert!(serde_json::from_str::<ProfileName>(bad).is_err(), "{bad}");
    }
}

#[test]
fn actor_is_a_non_empty_name_of_at_most_64_chars() {
    let sixty_four = "a".repeat(64);
    let sixty_five = "a".repeat(65);
    for ok in ["pfleet", "holler profile apply", "mo", sixty_four.as_str()] {
        assert_eq!(Actor::parse(ok).unwrap().as_str(), ok);
    }
    for bad in ["", sixty_five.as_str()] {
        assert!(Actor::parse(bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        serde_json::to_string(&Actor::parse("mo").unwrap()).unwrap(),
        r#""mo""#
    );
    assert!(serde_json::from_str::<Actor>(r#""""#).is_err());
}
