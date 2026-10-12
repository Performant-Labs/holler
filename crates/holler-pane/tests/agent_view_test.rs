#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
//! `AgentView` and `TurnAgent` (#647 part 2, ADR-0021 section 12): what
//! `HarnessPort::agent_view` answers. The empty default is "cannot tell"; the builders build
//! the turns as given (oldest first) and the dispatchers sorted by key without duplicates, so
//! that rule is kept in one place whatever an implementation reads. A struct literal outside
//! the crate does not compile: that is the `compile_fail,E0639` doc test on `HarnessPort`.

use holler_pane::{AgentKey, AgentView, TurnAgent};

fn key(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

#[test]
fn the_default_view_is_empty() {
    let view = AgentView::default();
    assert_eq!(view.turns, Vec::<TurnAgent>::new());
    assert_eq!(view.dispatchers, Vec::<AgentKey>::new());
}

#[test]
fn a_turn_holds_its_agent_and_whether_it_can_dispatch() {
    let turn = TurnAgent::new(Some(key("build")), Some(false));
    assert_eq!(turn.agent, Some(key("build")));
    assert_eq!(turn.can_dispatch, Some(false));
    let unknown = TurnAgent::new(None, None);
    assert_eq!(
        unknown.agent, None,
        "no agent recorded, or a name that is not a key"
    );
    assert_eq!(unknown.can_dispatch, None, "the server does not say");
}

#[test]
fn the_turns_builder_keeps_the_order_given() {
    let turns = vec![
        TurnAgent::new(Some(key("plan")), Some(true)),
        TurnAgent::new(Some(key("build")), Some(false)),
        TurnAgent::new(Some(key("build")), Some(false)),
    ];
    let view = AgentView::default().with_turns(turns.clone());
    assert_eq!(view.turns, turns, "oldest first, a repeated turn kept");
    assert_eq!(view.dispatchers, vec![]);
}

#[test]
fn the_dispatchers_builder_sorts_and_drops_duplicates() {
    let view = AgentView::default().with_dispatchers(vec![
        key("reviewer"),
        key("orchestrator"),
        key("reviewer"),
    ]);
    assert_eq!(view.dispatchers, vec![key("orchestrator"), key("reviewer")]);
    assert_eq!(view.turns, vec![]);
}

#[test]
fn the_builders_compose() {
    let view = AgentView::default()
        .with_dispatchers(vec![key("orchestrator")])
        .with_turns(vec![TurnAgent::new(None, Some(true))]);
    assert_eq!(view.turns.len(), 1);
    assert_eq!(view.dispatchers, vec![key("orchestrator")]);
    assert_ne!(view, AgentView::default());
}
