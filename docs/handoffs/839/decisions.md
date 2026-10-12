# Decision journal: #839, DRIVEN follows each pane's session of record

Run handoff directory `docs/handoffs/839` · branch `issue-839-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry. The three earlier plan checks are journalled in
`docs/handoffs/839-plan-checks/decisions.md`.

## A, up-front plan review, fourth pass: 2026-10-12T07:14-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 7 `warn` findings (`docs/handoffs/839/handoff-A.md`).
  - **The third pass's eight findings are all resolved**, each checked against the code: P7 (the offline
    `data.reason`), P8 (the exclusion rule stated once), the `.send_prompt(` scan, the placement of `hold_tests` and
    of the `mod` lines, `first_refusal`, the rename scan, the clashes table, and the placement, `token_id` and
    quoting points.
  - **The warns:**
    1. `control/rebind` refreshes the roster with a whole-set `Roster::advertise` from a control-socket task. It
       races the presence handler, which writes the roster first and the cache two awaits later. It can put stale
       state on the body's other rows for up to the 15 s heartbeat (`wait --until idle` can match early), and it
       reverts `talk::say`'s forward-fills. The fix is a one-row forward-fill of `harness_session_id`.
    2. P8's exception lets a pane's recorded DRIVEN outrank another pane's session of record. In Acceptance 20(d)
       row (4), both panes get `One` for one body, and P1's remedy re-points the body that P2's record proves. This
       is the operator's decision.
    3. `interrupt` with text checks the hold before it cancels, but not the pane gate.
    4. Existing pins this story changes are not named: `switch_moves_the_tui_and_the_record_together` and
       `assert_recorded` in `pane_verbs/switch.rs` (not in the Blast radius), and the op lists of
       `reset_creates_a_fresh_session_and_switches_to_it`.
    5. `switch_command` in `findings.rs` would import `tx_switch::parse_session_id`, a module cycle.
    6. Acceptance 13's `forward(` scan collides with `pane_dispatch::forward` and `send_query_forward`.
    7. `pane/mod.rs` is missing from the Blast radius, and `control/say` would be split across two files.
- **Confirmed against the code, and left standing:**
  - `SayOpts` is built only in `talk::send_turn`, so the gate key reaches every delivery variant.
  - The query forward is method-agnostic, replies route by correlation id, and the confirmation probes use their
    own map.
  - The session task attaches from `&inner.config` in `attempt_initial_attach` and in `start_turn`.
  - `HttpAttachDriver::attach` has one await (`check_exists`) before it spawns and returns, so the 5 s deadline
    either drops it with nothing spawned or lets it finish.
  - `holler-hub` already depends on `holler-pane`.
  - `admin/say`'s params are untyped, so `check_only` changes no golden file.
  - `Error::new` already carries an optional reason.
  - A Claude Code UUID (36 characters) is never a `RoutableName` (segments are 32 at most), so it resolves as an id.
  - The `Ports` literals are 20 in 17 files.
  - The cited tests (E86, E91, E100, E150) exist where the brief says.
  - The hub tests the brief places in `holler-hub/tests` can drive fake bodies through the registry's
    `LiveCommand` channels.
  - `main` is still `4e691d9`.
- **Assumed:**
  - Phase 3 warns reach F and T only if relayed: their role docs do not read `handoff-A.md`. S reads it.
  - #803 lands as its brief's D13 says (one body for the fleet), which is what makes warn 1 likely to be hit.
  - The import of a TUI that shows a session another pane's stale DRIVEN names is the main way into warn 2's state.
    I did not trace every path into it.
- **Hedged:**
  - Warn 1 could be argued a `block`, since it is a concurrency defect in the plan's hub design. I kept it a `warn`
    because it is transient (it heals at the next presence), it touches no enforcement point (`send_prompt` and the
    body stay authoritative), and its fix is small and local.
  - Warn 2 is a corner of a rule the operator decided ("final, not to be re-opened"), so it is offered for the
    operator, with two alternatives.
  - I ran no build or test: no code exists yet. The rename scan was reproduced with `git ls-files` and `grep`.
- **Evidence:**
  - Read the brief's decision text in full (lines 1-522) and the third pass's handoff and journal.
  - **Hub:** `circuit.rs` (`Query` and `Say` arms, `static_wire_method`), `circuit/dispatch.rs` (`send_prompt`,
    `pane_gate`, `send_query_forward`, `handle_presence_notification`, `handle_response`,
    `handle_error_response`, the confirmation probes, `hold_tests`), `live.rs` (`LiveCommand`, `SayOpts`,
    `LiveHandle`, `query`, `insert`, `find_by_token`, `find_target`, `replace_session`, `resolve_session`, the inline
    tests), `talk.rs` (`answer`, `say`, `send_turn`), `interrupt.rs`, `control_server.rs`, `roster.rs` (`advertise`,
    `apply_ad`, `build_row`, the forward-fills, `qualified_name`), `panes/mod.rs`, `lib.rs`, `Cargo.toml`.
  - **Body:** `session_manager.rs`, `session_manager/task.rs`, `http_attach_driver.rs`, `connection.rs`,
    `connection/session_dispatch.rs`.
  - **Pane crate and kit:** `tx_switch.rs` (in full), `reconcile.rs`, `findings.rs`, `error.rs`, `ports.rs`; the
    test kit's `lib.rs` and `fault.rs`.
  - **CLI:** `say_cmd.rs`, `interrupt_cmd.rs`, `pane/wiring.rs`, `pane/wiring/store.rs`, `pane/mod.rs`; the tests
    `hold_single_path_test.rs`, `wizard/skilltext_w5_rename.rs`, `pane_verbs/switch.rs`, `pane_verbs/reset.rs`,
    `pane_verbs/relaunch.rs`.
  - **Proto:** `methods.rs`, `error.rs`, `vocab.rs`, `lib.rs`, `tests/golden_test.rs`, `tests/codec_test.rs`.
  - **Docs:** ADR-0021 sections 4, 5, 6 and 11; `docs/protocol/v2.md`; `scripts/lint.sh`.
  - **Workflow:** `coding-pipeline.workflow.mjs` (the arch gate), `pipeline-conventions.md` section 1, the generated
    `feature-implementor.md` and `tester.md`.
  - Commands: `grep -n`, `sed -n`, `wc -l`, `git log`, `git ls-files`, `git ls-remote`, `git merge-base`.

## T, author tests (RED): 2026-10-12, 07:58 MDT

- **Decided:**
  - **RED is valid.** It is compile-level for the contract and assertion-level wherever the code compiles today (`docs/handoffs/839/handoff-T-red.md`). The suite has about 140 tests, new or changed, in 19 files and 6 crates. It covers Acceptance 1 to 22, at the cheapest tier each criterion allows: pure functions in `holler-pane`, the fake in the test kit, the hub in process over its real control socket with fake bodies, the body over a real `SessionManager` and the fake OpenCode server, the verbs over the fakes, and one end-to-end file (`driven_e2e_test`, a new `[[test]]`).
  - **The contract's names and shapes** are the ones the brief gives. T fixed the ones it leaves open, listed in the handoff:
    - `FakeHub`'s API, with `fail_send` for the race;
    - `HubCall` and `SentPrompt`, with `of_record`;
    - `HubFixture`, and the 7 conformance case ids;
    - `handle_rebind`'s signature, and `REBIND_DEADLINE` and `REBIND_REPLY_WAIT`;
    - `SessionCommand::Rebind`'s fields;
    - `{admitted: true}`;
    - `send_call` and `HubClient::new`.
  - **T migrated every test `Ports` literal**, with `&Unwired` where nothing reaches the hub and a `FakeHub` in the doctor and Claude rigs. It also rewrote the pins this story changes: E83, E84, E91, `switch_moves_the_tui_and_the_record_together`, `assert_recorded` and D9's scenario step.
  - **`pane_gate`'s tests are `panes/gate/tests.rs`**, the hub's `holds/tests.rs` precedent. The brief's `panes/gate_tests.rs` is not a child module of `gate.rs` without a `#[path]`.
- **Assumed:**
  - **A's warn 4.** A reset with no advertised body reads no `list_sessions` or `pane_store.list` (the existing op pins are kept).
  - **A's warn 3.** `interrupt` with text pre-checks the pane gate (one new test, `check_only_test.rs`); O may drop it.
  - **Acceptance 20(d) row (1)** is `session-of-other-pane`, by D3 step 1's order ("the existing refusals; then" the driver). The acceptance text says `session-driven-elsewhere`, so this is flagged for O.
- **Hedged:**
  - **Compile-level RED** shows the contract is missing. It cannot show that each behavioural assertion fails for the right reason, which is why T-green spot-checks by removing behaviour.
  - **To keep F from meeting T's own type errors**, T type-checked the whole suite against a throwaway stub of the contract and ran `cargo clippy --workspace --all-targets -- -D warnings` (clean). It then removed the stubs and restored `src/`. `git status` matches the T-only change set.
  - **The contract amendment** (ADR and docs, scope 1) is not on the branch, so the docs test is RED on today's text until it lands.
- **Evidence:**
  - **Read:** the brief's decision text, Acceptance and Blast radius, and the Evidence the tests touch (E1 to E24, E30 to E38, E42, E46, E47, E77, E81 to E86, E91, E95 to E97, E101 to E105, E116, E122 to E127, E137 to E142, E147 to E150, E158 to E170); `handoff-A.md`.
  - **The code:** `tx_switch.rs`, `findings.rs`, `reconcile.rs`, `ports.rs`, `circuit/dispatch.rs`, `live.rs`, `talk.rs`, `interrupt.rs`, `control_server.rs`, `control.rs`, `holds.rs`, `roster.rs`, `session_manager.rs`, `session_manager/task.rs`, `prompt_dispatch.rs`, `connection/session_dispatch.rs`, `http_attach_driver.rs`, `query.rs`, `say_cmd.rs`, `prompt_target.rs`, `wiring.rs`, the test kit's `fault.rs`, `fixture.rs` and `pane_store.rs`, and the rigs and harnesses named in the handoff.
  - **Commands:**
    - `cargo test -p <crate> --test <target> --no-run`, for every target touched;
    - `cargo test` on `check_only_test`, `offline_reason_test`, `hold_single_path_test`, `docs_pane_control_test`, the proto lib, `codec_test` and `http_attach_driver_test`;
    - `cargo clippy --workspace --all-targets -- -D warnings`, against the stubs;
    - `bash scripts/lint.sh` (exit 0);
    - `rustfmt --edition 2021` on the new files only, with the `#[path]` includes kept out.
