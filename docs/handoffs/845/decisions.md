
## A (up-front plan review) — 2026-10-12
- **Decided:** PASS on the plan for #845 (option B, re-send inside the one `select_session` poll). Eight warnings, no block: how the re-send enters `Tui::watch` (extend it, no second poll, `attach_tui` unchanged); step order (query, then `send_by` re-send, so a dead TUI or 404 ends the call with no further send and the `settle` bound holds); one shared send-and-judge helper (no pasted reply handling); the test mechanism (stub hook plus atomic rename of the display-message reply); `attach_test.rs` size (706 lines, projected near the 900 lint failure, so name the placement); the re-send cadence; sequencing the settle measurement before F; the exact wording of the docs rule.
- **Assumed:** that `Call::send_by` bounds a re-send by the poll's `until` as `settled` relies on (read in lib.rs:513-520, not run); that the stub/fake-tmux race on an in-place `Fake::reply` write is real (read, not reproduced); the line count projection for attach_test.rs is an estimate.
- **Hedged:** whether the observed 8 s warm-up fits B at all is unknown here (the brief's stop condition (a) covers it); the brief's root-cause claim (title never changes versus a hung request) is unestablished, and T's first test settles it.
- **Evidence:** read docs/handoffs/845-brief.md, crates/holler-adapter-opencode/src/{attach,lib,tui}.rs, tests/attach_test.rs, tests/support/{stub,fake_tmux}.rs, tests/fixtures/fake-tmux, crates/holler-cli/tests/pane_integration/scenario.rs, docs/adr/ADR-0021.md (L543-560), docs/pane-control.md (L247-258), scripts/lint.sh (900-line gate).

## T (author / RED) — 2026-10-12
- **Decided:** four new tests in `attach_test.rs` (`ac29f-i`) rather than a new file or a moved rig, 841 lines, under the 900 lint gate; `ac29f/g/h` RED, `ac29i` a valid regression pin of the unchanged `request` bound. The stub got `hang` and `on_nth`; the changed title is a rename onto the fake tmux's data file, so `fake_tmux.rs` and the fixture stay untouched (#844). `ac29f` asserts exactly 3 POSTs (none once the title shows). Positive tests use a wide `settle` so they cannot race a loaded runner.
- **Assumed:** the real-OpenCode probe on 1.18.35 in the repo's scratch rig stands for the 1.19.0 behaviour of the brief only loosely; the race did not reproduce, so neither stop condition (a) nor (b) is shown.
- **Hedged:** the real cause of the 1.19.0 timeouts (unchanged title after `200 true`, versus something else) is not established by any real run; only the hermetic model of it is pinned. The brief's "8 s" is not a measured readiness time. A throwaway implementation (reverted) was used only to prove the suite satisfiable and stable.
- **Evidence:** `cargo test -p holler-adapter-opencode --test attach_test`: 24 pass, 3 fail (ac29f/g/h) with the output in handoff-T-red.md; probe numbers in the same file; `bash scripts/lint.sh` exit 0; clippy clean on the crate.

## F (implementation) — 2026-10-12
- **Decided:** the re-send enters through `Tui::watch`, which gained one per-poll action (`unseen`) that runs only after a query finds the pane live and not showing the session; `attach_tui` passes `|_| Ok(())` (unchanged behaviour), `select_session` passes a re-send. The step order is query, then re-send, so a dead or missing TUI ends the call with no further request (A warnings 1 and 2). The send-and-judge rules (404 `session-not-found`, anything but `true` `unavailable`) are one private fn `select`, used by the first send (bounded by the call's deadline, as `send` was) and every re-send (bounded by the poll's end via `send_by`) (A warning 3). No second poll, no new bound; `settle`/`call` untouched. Removed the #649 scenario's 20 s switch retry (criterion 6, handed to F by T's handoff).
- **Assumed:** that the hermetic model (title unchanged after `200 true`) is the real 1.19.0 failure; T's real probe on 1.18.35 did not reproduce the race. A re-sent request answered non-`true` or HTML ends `unavailable`, as the first one does (A's stricter reading).
- **Hedged:** criterion 6 (opt-in #649 scenario with `HOLLER_TEST_PANE_INTEGRATION=1`) was not run by F (no scratch Herdr run in this phase); its compile and default (ignored) run are green. Whether 5 s `settle` covers the 1.19.0 warm-up stays unmeasured; nothing was added under "Needs operator decision if measured".
- **Evidence:** `cargo test -p holler-adapter-opencode` all green (attach_test 27/27, ac29 x5 repeats green); `cargo test -p holler-pane` 141 passed; `cargo test -p holler-cli --test pane_verbs` 347 passed; `--test pane_integration` 71 passed, 4 ignored; clippy `-D warnings` clean on the adapter (all targets) and on holler-cli's lib, bins, `pane_integration` and `pane_verbs`; `bash scripts/lint.sh` exit 0; facts in evidence.md.

## T (phase 7, verify GREEN)
- Decided: PASS. Suite green, 8 repeat runs stable, three mutations each caught by a named test.
- Assumed: `cargo machete` unavailable locally; CI runs it and the change adds no dependency.
- Hedged: criterion 6 live run and risk 1 stay with the operator.
- Evidence: docs/handoffs/845/handoff-T-green.md.

## A (Phase 7, anti-duplication): PASS
F extended Tui::watch and reused poll/send_by/query/known; one private `select` helper. Two warns (test-only Rig::on_select near-copy of Fake::reply, attach_test.rs at 841 lines). See handoff-A-dup.md.
