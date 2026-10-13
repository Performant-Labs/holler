#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #650
//! The finding vocabulary of `holler pane import` (#650 acceptance 12) and its one refusal
//! code, `import-blocked`.
//!
//! `ImportFinding` copies `FindingKind`'s pattern: `code()` is the one source of each code,
//! the JSON form is the code, and `ALL` lists every kind. A condition reconcile already names
//! keeps reconcile's spelling, taken from its one source (`FindingKind::code`, or the closed
//! `PaneError` code), never retyped. A code is stable once merged (ADR-0021 section 9), so
//! the set is pinned here. The declaration order is the implementation's.
//!
//! Decided in Phase 4 (A's plan review, finding 1, applying the brief's own rule "a condition
//! that reconcile already names keeps reconcile's spelling"): a missing tmux session is
//! reconcile's `tmux-session-missing`, not a second name, and `session-not-found` is the
//! closed `PaneError` code. So the import adds ten new codes, not twelve.
//!
//! #835 adds two more (brief, round 2, R2-4): `harness-claude-code`, an `info` finding for a
//! pane whose tmux session runs Claude Code and that the import leaves alone, and
//! `harness-unreadable`, a `problem` for a pane whose harness could not be read.
//!
//! #831 adds five more (brief, "Final finding codes"; twenty-three in all): `claude-code-adopted`
//! (`info`, each adopted pane), `claude-process-unmatched` and `claude-login-env` (`problem`),
//! `claude-model-not-anthropic` and `spec-kind-differs` (`warning`).

use std::collections::{BTreeMap, BTreeSet};

use holler_pane::error::{class_of, is_valid_code, ErrorClass};
use holler_pane::findings::FindingKind;
use holler_pane::import::findings::ImportFinding;
use holler_pane::import::IMPORT_BLOCKED;
use holler_pane::PaneError;

/// The codes only the import has.
const NEW_CODES: [&str; 17] = [
    "pane-unlabelled",
    "pane-label-ambiguous",
    "sources-disagree",
    "source-unreadable",
    "source-missing",
    "no-session-recorded",
    "harness-not-opencode",
    "name-cell-differs",
    "already-imported",
    "ignored-entry",
    "harness-claude-code",
    "harness-unreadable",
    "claude-code-adopted",
    "claude-process-unmatched",
    "claude-login-env",
    "claude-model-not-anthropic",
    "spec-kind-differs",
];

/// The codes the import shares with reconcile and the closed error codes, each from its
/// one source.
fn reused_codes() -> Vec<String> {
    let closed = |error: PaneError| error.code().to_owned();
    vec![
        FindingKind::ObserveFailed.code().to_owned(),
        FindingKind::ServerWedged.code().to_owned(),
        FindingKind::ServerDown.code().to_owned(),
        FindingKind::TmuxSessionMissing.code().to_owned(),
        closed(PaneError::SessionNotFound {
            what: String::new(),
        }),
        closed(PaneError::CommandNotArgv),
    ]
}

/// Each code's level (brief "Exit codes and what blocks"): only a `problem` blocks.
fn levels() -> BTreeMap<String, &'static str> {
    let mut levels = BTreeMap::new();
    for code in [
        "pane-unlabelled",
        "pane-label-ambiguous",
        "sources-disagree",
        "source-unreadable",
        "source-missing",
        "harness-unreadable",
        "claude-process-unmatched",
        "claude-login-env",
        FindingKind::TmuxSessionMissing.code(),
    ] {
        levels.insert(code.to_owned(), "problem");
    }
    for code in [
        FindingKind::ObserveFailed.code(),
        FindingKind::ServerWedged.code(),
        FindingKind::ServerDown.code(),
        "session-not-found",
        "no-session-recorded",
        "command-not-argv",
        "claude-model-not-anthropic",
        "spec-kind-differs",
    ] {
        levels.insert(code.to_owned(), "warning");
    }
    levels.insert("harness-not-opencode".to_owned(), "skipped");
    // `harness-claude-code` is `info`, as the issue says, not `skipped` like
    // `harness-not-opencode` (brief section 3: a deliberate, recorded difference).
    for code in [
        "ignored-entry",
        "already-imported",
        "name-cell-differs",
        "harness-claude-code",
        "claude-code-adopted",
    ] {
        levels.insert(code.to_owned(), "info");
    }
    levels
}

#[test]
fn import_finding_codes_are_the_pinned_set() {
    let codes: Vec<&str> = ImportFinding::ALL.iter().map(|k| k.code()).collect();
    let unique: BTreeSet<&str> = codes.iter().copied().collect();
    assert_eq!(unique.len(), codes.len(), "a duplicate code: {codes:?}");

    let mut expected: BTreeSet<String> = NEW_CODES.iter().map(|c| (*c).to_owned()).collect();
    expected.extend(reused_codes());
    let got: BTreeSet<String> = unique.iter().map(|c| (*c).to_owned()).collect();
    assert_eq!(got, expected, "the import's finding codes");

    for kind in ImportFinding::ALL.iter() {
        assert!(
            is_valid_code(kind.code()),
            "{:?} is not kebab-case",
            kind.code()
        );
        let json = serde_json::to_value(kind).unwrap();
        assert_eq!(json, kind.code(), "{kind:?} serializes as its code");
    }
}

/// The reused codes are spelled as reconcile and the closed codes spell them, and no
/// new code takes a spelling reconcile already owns.
#[test]
fn import_finding_reuses_reconcile_spellings_and_adds_no_synonym() {
    let reconcile: BTreeSet<&str> = FindingKind::ALL.iter().map(|k| k.code()).collect();
    for code in NEW_CODES {
        assert!(!reconcile.contains(code), "{code} is already reconcile's");
    }
    let codes: BTreeSet<&str> = ImportFinding::ALL.iter().map(|k| k.code()).collect();
    assert!(
        !codes.contains("no-tmux-session"),
        "a missing tmux session is reconcile's tmux-session-missing: {codes:?}"
    );
}

/// `herdr-version-unsupported` is an error that ends the verb, never a finding of the
/// import (brief "The finding vocabulary", acceptance 14).
#[test]
fn herdr_version_unsupported_is_not_an_import_finding() {
    let unsupported = FindingKind::HerdrVersionUnsupported.code();
    assert!(
        ImportFinding::ALL.iter().all(|k| k.code() != unsupported),
        "no ImportFinding carries {unsupported}"
    );
}

#[test]
fn each_import_finding_has_its_level() {
    let levels = levels();
    for kind in ImportFinding::ALL.iter() {
        let level = serde_json::to_value(kind.level()).unwrap();
        let want = levels
            .get(kind.code())
            .unwrap_or_else(|| panic!("no level pinned for {}", kind.code()));
        assert_eq!(level, *want, "the level of {}", kind.code());
    }
}

/// #831: each adoption code is in `ALL` with the level the brief fixes, so a code that is
/// missing, or present at another level, fails here by name (the set test above says only that
/// the set differs).
#[test]
fn the_adoption_codes_are_listed_at_their_levels() {
    let want = [
        ("claude-code-adopted", "info"),
        ("claude-process-unmatched", "problem"),
        ("claude-login-env", "problem"),
        ("claude-model-not-anthropic", "warning"),
        ("spec-kind-differs", "warning"),
    ];
    for (code, level) in want {
        let kind = ImportFinding::ALL
            .iter()
            .find(|k| k.code() == code)
            .unwrap_or_else(|| panic!("ImportFinding::ALL lacks {code}"));
        assert_eq!(serde_json::to_value(kind.level()).unwrap(), level, "{code}");
    }
    assert_eq!(
        ImportFinding::ALL.len(),
        23,
        "eighteen kinds and five adoption kinds"
    );
}

/// `import-blocked` is the verb's own open code: a refusal, exit 3.
#[test]
fn import_blocked_is_an_open_refusal_code() {
    assert_eq!(IMPORT_BLOCKED.as_str(), "import-blocked");
    let error = PaneError::Refused {
        code: IMPORT_BLOCKED,
        message: "import blocked".to_owned(),
    };
    assert_eq!(error.code(), "import-blocked");
    assert_eq!(class_of(error.code()), ErrorClass::Refusal);
    assert_eq!(class_of(error.code()).exit_code(), 3);
}
