//! Regression guard for story #144's acceptance criterion: every
//! `holler-hub` module reports through the shared `holler_proto::log` core
//! rather than a raw `eprintln!`.
//!
//! `holler-hub` was still an empty stub crate when #144 landed (only
//! `holler-cli`'s `main.rs` and `holler-hub::token` existed), so the
//! criterion was unenforceable then. `hub serve` (#143) and the token store
//! (#163) have since given the crate real modules with real error paths, so
//! this test makes the criterion durable: a future patch that reaches for
//! `eprintln!` in a hub module fails CI instead of drifting back to
//! unstructured output.
//!
//! One line is deliberately exempt: `serve.rs`'s `{"event":"listening",…}`
//! readiness line is a fixed wire contract the test harness parses
//! regardless of `--log-format`/`--debug` (see the comment at its call
//! site), not a log event.

use std::path::Path;

#[test]
fn no_raw_eprintln_outside_the_listening_wire_contract() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    collect_rs_files(&src_dir, &mut offenders);

    let mut violations = Vec::new();
    for path in &offenders {
        // A source file we just found via `read_dir` is expected to be
        // readable; if it somehow is not, skip it rather than `.expect`
        // (this crate denies `clippy::expect_used`, even in tests).
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        for (lineno, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            // Skip comments/doc-comments — only a real macro invocation counts.
            if trimmed.starts_with("//") {
                continue;
            }
            if line.contains("eprintln!(") && !line.contains(r#""event":"listening""#) {
                violations.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    lineno + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "holler-hub must log through holler_proto::log, not a raw eprintln! \
         (story #144); offending lines:\n{}",
        violations.join("\n")
    );
}

/// Recursively collect every `.rs` file under `dir`.
fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
