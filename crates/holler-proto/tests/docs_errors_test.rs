#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Docs conformance for the error table (issue #441): `docs/protocol/v2.md` §8
//! must name every code the closed table has, with its JSON-RPC number and
//! its Holler string, and must document the `session_held` data fields and
//! the hold's roster fields. A code added to `Code::ALL` without a docs row
//! fails here.

use std::path::Path;

use holler_proto::error::Code;

fn v2_md() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol/v2.md")).unwrap()
}

/// The text of §8 (from its heading up to §9's).
fn errors_section(doc: &str) -> &str {
    let start = doc.find("\n## 8. Errors").expect("§8 heading");
    let rest = &doc[start..];
    let end = rest.find("\n## 9.").expect("§9 heading");
    &rest[..end]
}

#[test]
fn every_error_code_has_a_row_in_section_8() {
    let doc = v2_md();
    let section = errors_section(&doc);
    for c in Code::ALL {
        // The JSON-RPC-standard codes (-327xx / -326xx) are prose-listed by
        // number in the table too; every code has one row starting `| `-NNNNN`` .
        let needle = format!("| `{}` | `{}` |", c.jsonrpc(), c.data_code());
        assert!(section.contains(&needle), "docs §8 has no row for {} ({})", c.data_code(), c.jsonrpc());
    }
}

#[test]
fn session_held_data_and_roster_fields_are_documented() {
    let doc = v2_md();
    let section = errors_section(&doc);
    // The refusal's structured data.
    assert!(section.contains("`since`"), "§8 must document session_held's `since`");
    assert!(section.contains("`reason`"), "§8 must document session_held's `reason`");
    // The roster fields (§7) and their non-presence.
    for field in ["hold", "hold_reason", "held_since"] {
        assert!(doc.contains(&format!("`{field}`")), "docs must name the roster field `{field}`");
    }
}
