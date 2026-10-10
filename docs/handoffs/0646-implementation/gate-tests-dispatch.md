# The dispatch.rs gate tests — blocked from T's write scope, delivered here

**What happened:** the t-red instructions tell T to add the pane-state gate unit tests
inside `crates/holler-hub/src/circuit/dispatch.rs`'s existing `#[cfg(test)] mod
hold_tests`. T's write scope in this session is `crates/*/tests/**` and
`docs/handoffs/**`; the edit to `crates/holler-hub/src/**` was **denied by the
permission layer**, and a denial is policy, not a puzzle — T did not route around it.

**What this file is:** the complete gate-test code, authored by T (the contract is
T's), for O — or F under O's instruction — to paste verbatim. It is the third piece of
the RED contract: without it, the hub-gate piece of story #646 (binding 1, 2 and 6,
the `send_prompt` AC) has no red test, and F implements it untested.

## Where it goes

`crates/holler-hub/src/circuit/dispatch.rs`, inside `mod hold_tests` (the existing
`#[cfg(test)]` module, after the two existing tests).

## Edit 1 — the three existing `HoldGate` constructions gain `panes: None`

Binding 1 (HoldGate drops `Copy`, gains `Option<Arc<PaneState>>`). In
`a_held_session_is_refused_in_every_variant_and_nothing_is_sent` (one construction,
inside the `for` loop) and in `an_unheld_session_and_a_released_one_are_sent` (two
constructions), change each:

```rust
// from:
let gate = HoldGate { holds: &holds, key: "io/alpha", grant: None };
// to:
let gate = HoldGate { holds: &holds, key: "io/alpha", grant: None, panes: None };
```

(same for the `"io/beta"` construction and the `"io/alpha"` one further down).

## Edit 2 — append this block at the end of `mod hold_tests`

```rust
    // --- the pane-state gate (story #646 part 3) -----------------------------------
    //
    // The same HoldGate now carries the pane registry (`panes`, `Registry::with_panes`):
    // a prompt to any session that is a pane's session of record is refused while that
    // pane is parked, unhealthy, or has SHOWN != DRIVEN (both observed, differing) —
    // the hub's one enforcement point, closing the bare `say <session-of-record>`
    // bypass. The pane-state predicate's CLI-side twin is `say_cmd::resolve_pane_target`
    // (`holler-cli`); it exists twice only because `holler-pane/**` is frozen (binding 6).
    // Seeded names are synthetic (`demo-c*`, `ses-demo-*`): no name here can name a real
    // pane or session.

    use std::sync::Arc as PaneArc;

    use holler_pane::pane::{Health as PaneHealth, Hold as PaneHold, LastObserved};
    use holler_pane::{Pane, PaneStore as _};

    /// A parked hold for the gate's seeded panes.
    fn parked_hold() -> PaneHold {
        PaneHold::Parked {
            reason: "disk full".to_owned(),
            release_when: "later".to_owned(),
            since: 5,
        }
    }

    /// A demo pane record for the gate: synthetic name, the session of record, hold,
    /// health and last observation the case varies.
    fn gate_pane(
        name: &str,
        session_of_record: Option<&str>,
        hold: PaneHold,
        health: PaneHealth,
        shown: Option<&str>,
        driven: Option<&str>,
    ) -> Pane {
        let mut pane = holler_pane_testkit::fixture::sample_pane(name).unwrap();
        pane.session_of_record = session_of_record.map(str::to_owned);
        pane.hold = hold;
        pane.harness.health = health;
        pane.last_observed = LastObserved {
            shown: shown.map(str::to_owned),
            driven: driven.map(str::to_owned),
            at: 1,
        };
        pane
    }

    /// A pane registry seeded with `panes` in a throwaway state dir. The dir is
    /// returned beside the registry and must outlive the test (the registry persists
    /// each write to it).
    fn pane_registry(panes: &[Pane]) -> (PaneArc<crate::panes::PaneState>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::state::HubState::from_root(dir.path().to_path_buf());
        std::fs::create_dir_all(&state.hub_dir).unwrap();
        let registry = PaneArc::new(crate::panes::PaneState::load(&state));
        for pane in panes {
            registry.cas_put(pane, 0).unwrap();
        }
        (registry, dir)
    }

    /// `send_prompt` to `session` over `holds` and `panes`; the wire error of a
    /// refusal, or a panic that names what was sent.
    async fn prompt(
        holds: &crate::holds::Holds,
        panes: Option<PaneArc<crate::panes::PaneState>>,
        session: &str,
    ) -> holler_proto::WireError {
        let key = format!("io/{session}");
        let gate = HoldGate {
            holds,
            key: &key,
            grant: None,
            panes,
        };
        let mut sink = Recording::default();
        let id = "h-01HTESTPANE00000000000000";
        match send_prompt(&mut sink, gate, id, session, message(), false, false).await {
            Err(SendPromptError::Refused(e)) => {
                assert!(sink.0.is_empty(), "a refused prompt puts nothing on the socket");
                e
            }
            other => panic!("{session} was sent: {other:?}, {:?}", sink.0),
        }
    }

    /// The refusal of a pane-state gate check: `-32011`, `hold_kind == "pane"`, the
    /// pane kebab code as `data.reason`, no `data.since`, and a one-line message
    /// naming the pane and the code.
    fn assert_pane_refusal(error: &holler_proto::WireError, pane: &str, code: &str) {
        assert_eq!(error.code, -32011, "{error:?}");
        let data = error.data.as_deref().expect("the refusal carries data");
        assert_eq!(data.hold_kind.as_deref(), Some("pane"), "{error:?}");
        assert_eq!(data.reason.as_deref(), Some(code), "{error:?}");
        assert!(
            data.since.is_none(),
            "no data.since: the park's epoch-ms since has no RFC 3339 form (decision 2b)"
        );
        assert!(!error.message.contains('\n'), "one line: {}", error.message);
        assert!(
            error.message.contains(pane) && error.message.contains(code),
            "the message names the pane and the code: {}",
            error.message
        );
    }

    /// AC (the hub gate): a prompt to a parked pane's session of record is refused in
    /// every variant — plain, queued and replace alike (the gate is the one
    /// enforcement point; `say --queue` passes through it too).
    #[tokio::test]
    async fn a_prompt_to_a_parked_pane_s_session_of_record_is_refused_in_every_variant() {
        let holds = crate::holds::Holds::in_memory();
        let (panes, _dir) = pane_registry(&[gate_pane(
            "demo-c1r1",
            Some("ses-demo-a"),
            parked_hold(),
            PaneHealth::Healthy,
            Some("ses-demo-a"),
            Some("ses-demo-a"),
        )]);
        for (queue, replace) in [(false, false), (true, false), (false, true), (true, true)] {
            let key = "io/ses-demo-a".to_owned();
            let mut sink = Recording::default();
            let gate = HoldGate {
                holds: &holds,
                key: &key,
                grant: None,
                panes: Some(PaneArc::clone(&panes)),
            };
            let res =
                send_prompt(&mut sink, gate, "h-01HTESTPANE00000000000000", "ses-demo-a", message(), queue, replace)
                    .await;
            assert!(
                matches!(res, Err(SendPromptError::Refused(ref e)) if e.code == -32011),
                "queue={queue} replace={replace}"
            );
            assert!(sink.0.is_empty(), "a refused prompt puts nothing on the socket");
            let Err(SendPromptError::Refused(error)) = res else { unreachable!() };
            assert_pane_refusal(&error, "demo-c1r1", "pane-parked");
        }
    }

    /// AC: an unhealthy pane, and a pane whose SHOWN and DRIVEN both exist and differ,
    /// refuse the same way.
    #[tokio::test]
    async fn unhealthy_and_shown_driven_mismatched_panes_refuse_the_same_way() {
        let holds = crate::holds::Holds::in_memory();
        let cases = [
            (
                "demo-c1r1",
                "ses-demo-a",
                gate_pane(
                    "demo-c1r1",
                    Some("ses-demo-a"),
                    PaneHold::None,
                    PaneHealth::Unhealthy("server wedged".to_owned()),
                    Some("ses-demo-a"),
                    Some("ses-demo-a"),
                ),
                "pane-unhealthy",
            ),
            (
                "demo-c2r1",
                "ses-demo-b",
                gate_pane(
                    "demo-c2r1",
                    Some("ses-demo-b"),
                    PaneHold::None,
                    PaneHealth::Healthy,
                    Some("ses-shown"),
                    Some("ses-driven"),
                ),
                "pane-shown-driven-mismatch",
            ),
        ];
        for (pane, session, record, code) in cases {
            let (panes, _dir) = pane_registry(&[record]);
            let error = prompt(&holds, Some(panes), session).await;
            assert_pane_refusal(&error, pane, code);
        }
    }

    /// AC: a healthy, unparked matching pane goes through (one observation side
    /// absent included), and a session no pane names is never touched by the gate.
    #[tokio::test]
    async fn a_healthy_matching_pane_and_an_unmatched_session_go_through() {
        let holds = crate::holds::Holds::in_memory();
        let (panes, _dir) = pane_registry(&[
            gate_pane(
                "demo-c1r1",
                Some("ses-demo-a"),
                PaneHold::None,
                PaneHealth::Healthy,
                Some("ses-demo-a"),
                None, // driven absent: not a mismatch
            ),
        ]);
        let mut sink = Recording::default();
        for session in ["ses-other", "ses-demo-a"] {
            let key = format!("io/{session}");
            let gate = HoldGate {
                holds: &holds,
                key: &key,
                grant: None,
                panes: Some(PaneArc::clone(&panes)),
            };
            assert!(
                send_prompt(&mut sink, gate, "h-01HTESTPANE00000000000000", session, message(), false, false)
                    .await
                    .is_ok(),
                "{session} is sent"
            );
        }
        assert_eq!(sink.0.len(), 2);
    }

    /// Binding 6 (fail-closed across same-named sessions): two panes share a session
    /// of record and any bad one refuses; the first bad pane in name order is the one
    /// the refusal names.
    #[tokio::test]
    async fn any_bad_pane_of_the_same_session_refuses_fail_closed() {
        let holds = crate::holds::Holds::in_memory();
        let healthy = |name: &str| {
            gate_pane(
                name,
                Some("ses-demo-a"),
                PaneHold::None,
                PaneHealth::Healthy,
                Some("ses-demo-a"),
                Some("ses-demo-a"),
            )
        };
        let bad = [
            ("demo-c2r1", parked_hold(), PaneHealth::Healthy, "pane-parked"),
            (
                "demo-c2r1",
                PaneHold::None,
                PaneHealth::Unhealthy("server wedged".to_owned()),
                "pane-unhealthy",
            ),
            (
                "demo-c2r1",
                PaneHold::None,
                PaneHealth::Healthy,
                "pane-shown-driven-mismatch",
            ),
        ];
        // The bad pane sorts after the healthy one...
        for (name, hold, health, code) in bad {
            let mismatched = code == "pane-shown-driven-mismatch";
            let record = if mismatched {
                gate_pane(name, Some("ses-demo-a"), hold, health, Some("ses-shown"), Some("ses-driven"))
            } else {
                gate_pane(name, Some("ses-demo-a"), hold, health, Some("ses-demo-a"), Some("ses-demo-a"))
            };
            let (panes, _dir) = pane_registry(&[healthy("demo-c1r1"), record]);
            let error = prompt(&holds, Some(panes), "ses-demo-a").await;
            assert_pane_refusal(&error, name, code);
        }
        // ...and before it: the refusal still comes.
        let (panes, _dir) = pane_registry(&[
            gate_pane(
                "demo-c1r1",
                Some("ses-demo-a"),
                parked_hold(),
                PaneHealth::Healthy,
                Some("ses-demo-a"),
                Some("ses-demo-a"),
            ),
            healthy("demo-c2r1"),
        ]);
        let error = prompt(&holds, Some(panes), "ses-demo-a").await;
        assert_pane_refusal(&error, "demo-c1r1", "pane-parked");
    }

    /// Binding 6 (the check runs after `holds.admit`): an operator hold still refuses
    /// first, with its own kind — the pane gate never shadows the hold registry.
    #[tokio::test]
    async fn an_operator_hold_still_refuses_first_with_its_own_kind() {
        let holds = crate::holds::Holds::in_memory();
        holds.hold("io/ses-demo-a", Some("freeze"));
        let (panes, _dir) = pane_registry(&[gate_pane(
            "demo-c1r1",
            Some("ses-demo-a"),
            parked_hold(),
            PaneHealth::Healthy,
            Some("ses-demo-a"),
            Some("ses-demo-a"),
        )]);
        let error = prompt(&holds, Some(panes), "ses-demo-a").await;
        assert_eq!(error.code, -32011);
        let data = error.data.as_deref().expect("data");
        assert_eq!(data.hold_kind.as_deref(), Some("operator"), "the hold's own kind");
        assert_ne!(data.reason.as_deref(), Some("pane-parked"));
        assert!(error.message.contains("freeze"), "{}", error.message);
    }

    /// Binding 1 (the wiring pin): the Registry carries the pane-state handle through
    /// `with_panes` — the builder `serve.rs`'s `build_shared_state` appends, and
    /// `circuit.rs`'s one `HoldGate` construction reads back out.
    #[test]
    fn the_registry_carries_the_pane_state_through_with_panes() {
        let (panes, _dir) = pane_registry(&[]);
        let registry = crate::live::Registry::new().with_panes(PaneArc::clone(&panes));
        assert!(registry.panes().is_some(), "the handle is carried");
    }
```

## The production shape these tests pin (F's contract)

- `HoldGate` (`dispatch.rs`): keeps `holds`, `key`, `grant`; gains
  `panes: Option<Arc<crate::panes::PaneState>>`; `#[derive(Clone)]` only (`Copy` goes).
- `Registry` (`live.rs`): gains `with_panes(self, Arc<PaneState>) -> Self` and a
  `panes()` getter returning `Option<Arc<PaneState>>` (the test only calls `.is_some()`).
- `send_prompt`: after `gate.holds.admit(...)` (which keeps priority), one synchronous
  `PaneState::list()` read when `panes` is `Some` — match the prompted **bare session
  name** against each pane's `session_of_record`; the first matching pane that is
  parked / unhealthy / SHOWN≠DRIVEN (name order) refuses with
  `WireError::new(Code::SessionHeld, message-naming-pane-and-code,
  Some(<pane kebab code>)).with_hold_kind("pane")` — no `data.since`.
- The gate is silent when `panes` is `None` (the three hold-only constructions) and
  when no pane names the session.
