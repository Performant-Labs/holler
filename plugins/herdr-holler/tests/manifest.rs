#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)] // #651
//! Acceptance: the manifest is valid (id/name/version/description present,
//! `min_herdr_version` "0.9.1" — Herdr 0.9.1 protocol 22 is the supported target),
//! every command is an argv array (never a shell string), every `[[actions]]`
//! command is a `holler pane` verb of the merged set (display only: the plugin
//! mutates nothing itself), and the `[[events]]` hooks on `pane.created` and
//! `layout.updated` re-run the plugin's refresh entry (metadata is lost on a
//! Herdr restart, so the hooks are the re-report triggers).

use std::fs;

use toml::Value;

/// The merged pane verbs the actions may run (#643 list/get/watch, #645
/// switch/reset, #646 park/unpark, #647 doctor; `close` and routed `say` are
/// not merged and not offered).
const PANE_VERBS: &[&str] = &[
    "list", "get", "watch", "switch", "reset", "park", "unpark", "doctor",
];

fn manifest() -> Value {
    let path = format!("{}/herdr-plugin.toml", env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(path).unwrap();
    text.parse().unwrap()
}

/// The `[[actions]]` tables, as arrays-of-tables values.
fn actions(doc: &Value) -> Vec<&Value> {
    doc.get("actions")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .collect()
}

/// The `[[events]]` tables, as array-of-tables values.
fn events(doc: &Value) -> Vec<&Value> {
    doc.get("events")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .collect()
}

/// The command of a table, asserted to be an argv array of non-empty strings
/// (never a shell string) and returned as a slice of str.
fn argv<'a>(table: &'a Value, what: &str) -> Vec<&'a str> {
    let command = table
        .get("command")
        .unwrap_or_else(|| panic!("{what} has no command"));
    let array = command
        .as_array()
        .unwrap_or_else(|| panic!("{what} command is not an argv array: {command}"));
    let words: Vec<&str> = array
        .iter()
        .map(|word| {
            word.as_str()
                .unwrap_or_else(|| panic!("{what} command holds a non-string"))
        })
        .collect();
    for word in &words {
        assert!(!word.is_empty(), "{what} command holds an empty word");
    }
    words
}

#[test]
fn manifest_is_valid_and_every_command_is_argv() {
    let doc = manifest();
    for key in ["id", "name", "version", "description", "min_herdr_version"] {
        let value = doc
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{key} is missing or not a string"));
        assert!(!value.is_empty(), "{key} is empty");
    }
    assert_eq!(
        doc.get("min_herdr_version").and_then(Value::as_str),
        Some("0.9.1"),
        "the supported target is Herdr 0.9.1 (protocol 22)"
    );
    for action in actions(&doc) {
        argv(action, "an action");
    }
    for event in events(&doc) {
        argv(event, "an event");
    }
}

#[test]
fn every_action_is_a_holler_pane_verb() {
    let doc = manifest();
    assert!(!actions(&doc).is_empty(), "the manifest offers no actions");
    for action in actions(&doc) {
        let id = action
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>");
        let command = argv(action, &format!("action {id}"));
        assert!(
            command.len() >= 3,
            "action {id} is `{command:?}`; a pane verb needs program, `pane` and the verb"
        );
        assert_eq!(
            command[0], "holler",
            "action {id} does not run the holler binary"
        );
        assert_eq!(
            command[1], "pane",
            "action {id} is not a pane verb: `{command:?}`"
        );
        assert!(
            PANE_VERBS.contains(&command[2]),
            "action {id} runs `{}`, which is not one of the merged pane verbs {:?}",
            command[2],
            PANE_VERBS
        );
    }
}

#[test]
fn event_hooks_rerun_the_refresh_on_the_two_triggers() {
    let doc = manifest();
    let hooks = events(&doc);
    assert!(!hooks.is_empty(), "the manifest wires no event hooks");
    for trigger in ["pane.created", "layout.updated"] {
        let wired = hooks
            .iter()
            .any(|hook| hook.get("on").and_then(Value::as_str) == Some(trigger));
        assert!(wired, "no event hook is wired on `{trigger}`");
    }
    for hook in &hooks {
        let on = hook
            .get("on")
            .and_then(Value::as_str)
            .unwrap_or("<unnamed>");
        let command = argv(hook, &format!("the `{on}` hook"));
        assert_eq!(
            command.first(),
            Some(&"herdr-holler"),
            "the `{on}` hook does not run the plugin binary: `{command:?}`"
        );
        assert!(
            command.contains(&"refresh"),
            "the `{on}` hook does not re-run the refresh entry: `{command:?}`"
        );
    }
}
