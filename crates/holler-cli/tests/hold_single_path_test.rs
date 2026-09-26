#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #442
//! The session hold has one enforcement point (issue #442, umbrella decision
//! 3): every prompt reaches a body through `send_prompt` in
//! `crates/holler-hub/src/circuit/dispatch.rs`, and the hold is checked there.
//! A second way to put a `session/prompt` on a body's socket would be a way
//! around every hold, so this test fails when one appears.
//!
//! It is a source-level guard, deliberately: it reads the hub's non-test
//! source and asserts that the `session/prompt` method name is *built* in
//! exactly one place, and that place checks the hold first. The behavioral
//! half (the generic request forward cannot carry a prompt either) is
//! `hold_hub_test::a_prompt_cannot_be_forwarded_around_the_hold`.

use std::path::{Path, PathBuf};

fn hub_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../holler-hub/src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The file's code with `//` comments and everything from the first
/// `#[cfg(test)]` on removed (test modules are last in every hub file).
fn code_of(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    let text = text.split("#[cfg(test)]").next().unwrap();
    text.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn session_prompt_is_built_in_exactly_one_place_and_the_hold_is_checked_there() {
    let mut files = Vec::new();
    rust_files(&hub_src(), &mut files);
    let mut builders = Vec::new();
    for f in &files {
        let code = code_of(f);
        // Constructing the request: the method name as a request's method.
        // (`static_wire_method` in circuit.rs only maps an inbound name to a
        // log label and never sends.)
        for (i, line) in code.lines().enumerate() {
            if line.contains("\"session/prompt\"") && line.contains("Envelope::request") {
                builders.push(format!("{}:{}", f.strip_prefix(hub_src()).unwrap().display(), i + 1));
            }
        }
    }
    assert_eq!(builders.len(), 1, "exactly one place may build a session/prompt request: {builders:?}");
    assert!(builders[0].starts_with("circuit/dispatch.rs"), "the one builder must be send_prompt: {builders:?}");

    // And the hold is checked in that function before the request is built.
    let dispatch = code_of(&hub_src().join("circuit/dispatch.rs"));
    let start = dispatch.find("async fn send_prompt").expect("send_prompt exists");
    let body = &dispatch[start..];
    let check = body.find("gate.holds.admit(").expect("send_prompt checks the hold");
    let build = body.find("Envelope::request").expect("send_prompt builds the request");
    assert!(check < build, "the hold must be checked before the prompt is built or sent");
}

#[test]
fn the_generic_request_forward_allows_only_query_and_answer() {
    // `LiveHandle::query` is the hub's generic "forward this method to the
    // body" path (`hub query TARGET ...`, `answer`). It must not carry a prompt.
    let live = code_of(&hub_src().join("live.rs"));
    let start = live.find("pub async fn query(").expect("LiveHandle::query exists");
    let body = &live[start..];
    assert!(
        body.find("method.starts_with(\"query/\")").is_some_and(|i| i < body.find("LiveCommand::Query").unwrap()),
        "LiveHandle::query must reject anything but query/* and session/answer before forwarding"
    );
}
