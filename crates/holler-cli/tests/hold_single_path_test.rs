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

// --- #839: the forward keeps its shape, one prompt path from the pane verbs, one predicate ----

fn crate_src(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(name).join("src")
}

/// Every non-test `.rs` file of a crate's `src`, each with its code (`code_of`), and its path
/// relative to that `src`.
fn sources(name: &str) -> Vec<(String, String)> {
    let root = crate_src(name);
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    // An out-of-line test module (`holds/tests.rs`, `panes/gate/tests.rs`, `rebind_tests.rs`)
    // has no `#[cfg(test)]` line of its own for `code_of` to cut at: it is test code, not a copy.
    files.retain(|f| {
        f.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n != "tests.rs" && !n.ends_with("_tests.rs"))
    });
    files.sort();
    files
        .iter()
        .map(|f| (f.strip_prefix(&root).unwrap().display().to_string(), code_of(f)))
        .collect()
}

/// The name of the `fn` whose declaration is the last one before `at` in `code`.
fn enclosing_fn(code: &str, at: usize) -> &str {
    let decl = code[..at].rfind("fn ").expect("a call inside a function");
    let rest = &code[decl + 3..];
    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
    &rest[..end]
}

/// D2 and P1 (Acceptance 13): one private helper, `LiveHandle::forward`, builds every
/// `LiveCommand::Query` (the circuit's match arm destructures it and builds none). It lies after
/// `pub async fn query(` in `live.rs`, so the scan above still finds the allowlist check first,
/// and it is called exactly twice: by `query`, after its allowlist check, and by `rebind` in
/// `live/target.rs`, with the literal method `"session/rebind"`.
#[test]
fn the_query_forward_is_built_once_and_called_by_query_and_rebind_only() {
    let hub = sources("holler-hub");
    let built: Vec<String> = hub
        .iter()
        .flat_map(|(path, code)| {
            code.lines()
                .enumerate()
                .filter(|(_, l)| l.contains("LiveCommand::Query {") && !l.contains("=>"))
                .map(move |(i, _)| format!("{path}:{}", i + 1))
        })
        .collect();
    assert_eq!(built.len(), 1, "one place builds a LiveCommand::Query: {built:?}");
    assert!(built[0].starts_with("live.rs:"), "{built:?}");
    let live = code_of(&hub_src().join("live.rs"));
    let at = live.find("LiveCommand::Query {").unwrap();
    assert_eq!(enclosing_fn(&live, at), "forward", "the builder is the forward helper");
    let query = live.find("pub async fn query(").unwrap();
    assert!(query < live.find("fn forward(").unwrap(), "forward lies after query");

    let target = code_of(&hub_src().join("live/target.rs"));
    let calls: Vec<(String, &str)> = [("live.rs", live.as_str()), ("live/target.rs", target.as_str())]
        .into_iter()
        .flat_map(|(path, code)| {
            code.match_indices("self.forward(")
                .map(move |(at, _)| (path.to_owned(), enclosing_fn(code, at)))
        })
        .collect();
    let names: Vec<(&str, &str)> = calls.iter().map(|(p, f)| (p.as_str(), *f)).collect();
    assert_eq!(names, [("live.rs", "query"), ("live/target.rs", "rebind")], "{calls:?}");
    let in_query = &live[query..];
    let allowlist = in_query.find("method.starts_with(\"query/\")").unwrap();
    assert!(allowlist < in_query.find("self.forward(").unwrap(), "query checks first");
    let rebind = &target[target.find("fn rebind(").unwrap()..];
    let forward = rebind.find("self.forward(").unwrap();
    assert!(rebind[forward..].lines().next().unwrap().contains("\"session/rebind\""), "{rebind}");
    let others: Vec<&String> = hub
        .iter()
        .filter(|(path, code)| code.contains("self.forward(") && !path.starts_with("live"))
        .map(|(path, _)| path)
        .collect();
    assert!(others.is_empty(), "no other caller of the forward: {others:?}");
}

/// D5 (Acceptance 13): no verb and no engine calls `HarnessPort::send_prompt`. In `holler-pane`
/// and `holler-cli`, a `.send_prompt(` call is allowed only in the body of a `fn send_prompt`
/// that forwards it, and there are exactly two: the wired harness (`pane/wiring/bind.rs`) and the
/// router (`ports/router.rs`), one each. A first message goes through the hub's prompt path.
#[test]
fn send_prompt_is_called_only_by_the_two_pass_throughs() {
    let mut calls = Vec::new();
    for krate in ["holler-pane", "holler-cli"] {
        for (path, code) in sources(krate) {
            for (at, _) in code.match_indices(".send_prompt(") {
                calls.push((format!("{krate}/{path}"), enclosing_fn(&code, at).to_owned()));
            }
        }
    }
    let want = [
        ("holler-pane/ports/router.rs".to_owned(), "send_prompt".to_owned()),
        ("holler-cli/pane/wiring/bind.rs".to_owned(), "send_prompt".to_owned()),
    ];
    assert_eq!(calls, want, "only the two pass-throughs call send_prompt");
}

/// #850 (plan review W-2): the body is the third place a `HarnessPort::send_prompt` can be called
/// from, outside the scan above (which reads `holler-pane` and `holler-cli`). The Claude Code attach
/// driver's delivery is that one call: it sits behind the hub's `send_prompt` (hold and pane gate
/// first), so a second caller in the body would be a way around both. Exactly one, in
/// `claude_attach_driver.rs`.
#[test]
fn the_body_calls_send_prompt_from_exactly_one_place() {
    let calls: Vec<String> = sources("holler-body")
        .into_iter()
        .flat_map(|(path, code)| {
            code.match_indices(".send_prompt(")
                .map(|(at, _)| format!("{path}:{}", enclosing_fn(&code, at)))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(calls.len(), 1, "exactly one HarnessPort::send_prompt call in the body: {calls:?}");
    assert!(
        calls[0].starts_with("claude_attach_driver"),
        "and it is the Claude Code attach driver's delivery: {calls:?}"
    );
}

/// D1 (Acceptance 14): the three pane codes and the pair rule exist once, in `holler_pane::gate`.
/// No hub or CLI source spells a code (the hub's gate and `say_cmd` take them from there), and
/// `holler roster`'s mismatch is a call of `shown_driven_pair_differs`, not a third copy.
#[test]
fn the_pane_codes_and_the_pair_rule_are_defined_once() {
    let mut spelled = Vec::new();
    for krate in ["holler-hub", "holler-cli"] {
        for (path, code) in sources(krate) {
            for literal in ["\"pane-parked\"", "\"pane-unhealthy\"", "\"pane-shown-driven-mismatch\""] {
                if code.contains(literal) {
                    spelled.push(format!("{krate}/{path}: {literal}"));
                }
            }
        }
    }
    assert!(spelled.is_empty(), "a pane code spelled outside holler_pane::gate: {spelled:?}");
    let roster = code_of(&crate_src("holler-cli").join("roster_cmd.rs"));
    let at = roster.find("fn shown_driven_mismatch(").expect("the roster's mismatch exists");
    let body = &roster[at..at + roster[at..].find("\n}").unwrap()];
    assert!(body.contains("shown_driven_pair_differs"), "a call of the one rule: {body}");
    assert!(!body.contains("shown != driven"), "no third copy of the rule: {body}");
}
