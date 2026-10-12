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

## F, implement (Phase 6): 2026-10-12, 09:12 MDT

- **Decided:**
  - **The contract is implemented under the names T pinned**, in the brief's order: the contract docs (ADR-0021, ADR-0014, ADR-0005, ADR-0003, `v2.md`, the guide, the CHANGELOG), then proto, body, hub, pane, the test kit and the CLI. The handoff is `docs/handoffs/839/handoff-F.md`.
  - **A's warns that F owned are all taken:**
    - Warn 1: `control/rebind` fills one roster row (`Roster::set_harness_session_id`, in a new child module `roster/rebind.rs`) after `Registry::replace_session`. It does not call a whole-set `Roster::advertise`, so the body's other rows and their state are never put back stale.
    - Warn 3: `interrupt` with text checks the pane gate before the cancel, beside `would_refuse`.
    - Warn 5: `parse_session_id` and `SESSION_ID_MAX` move to `findings.rs`, and `tx_switch` re-exports them, so `switch_command` causes no module cycle.
    - Warn 7: the whole `control/say` handler is in `control_say.rs`.
  - **Splits the brief's own rules call for:**
    - `tx_switch.rs` would have been 683 lines, over the brief's 650 trigger, so the plan's helpers moved to `tx_switch/plan.rs`.
    - `live.rs`'s inline tests moved whole to `live/tests.rs`, as the brief's "do first" says. Only the indentation changed (checked with `diff`).
  - **Choices the brief left open:**
    - `HubRefusal::Held` is `{ kind, reason }`, with no `key`. The `-32011` data carries no key, so `session-held` names the body session from one roster read, on that path only.
    - `control/rebind` checks the id with `parse_session_id` before any body is asked.
    - `FakePaneStore::records()` is a crate-private read that is not logged. It keeps `FakeHub`'s reads out of the store's call log, as T's contract requires.
    - The README's debug table names the body's new `rebind` event under `session`.
  - **Five documented commands** in F's doc text did not parse (`docs_cli_test`). They now use single-token placeholders.
- **Assumed:**
  - **The six failing tests are T's to fix** (T-1 to T-6 in the handoff). Each was confirmed by a temporary local edit of the test alone, run and then restored with `git checkout --`. With the edits, every affected target was green.
  - **The one failure of `profile_rename_faults_test` is a flake that existed before this story**, not caused by it:
    - it is a test from #665 that compares inodes;
    - run alone, it fails 20 times in 20 on the tree before #839 (`9ecfc34`, built in the scratchpad) and 40 in 40 on this branch;
    - this diff touches neither the rename nor the store.
- **Hedged:**
  - **`rebind_test` passed 8 of 8 in the full run, but it is flaky.** Run alone it fails 11 times in 12 (`-p` build) and 4 in 6 (workspace build). With `until_attached` temporarily made to wait for the first attach, it failed 0 times in 12. The production code cannot avoid the race: T's own guard pins the row as visible before the task runs. So this is T-5.
  - **The end-to-end tests were run green only with an ephemeral `--listen`.** The rig's hubs bind the default port, which on this machine is the operator's live hub. That hub was never contacted: the bind failed and the tests stopped at the start.
  - **Acceptance 20(d) row (1)** follows D3's order (the existing refusals come first), as T's tests do. T flagged it for O.
- **Evidence:**
  - Commands:
    - `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 2703 passed and 18 failed. All 18 are accounted for:
      - 15 are T-1 to T-6;
      - 1 is the five doc commands, since fixed;
      - 1 is the #665 flake.
    - `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
    - `bash scripts/lint.sh`: exit 0, with the three new goldens shown as `(new file)`.
    - `bash scripts/changelog-check.sh origin/main`: ok.
    - `rustfmt --check --edition 2021` on the nine new leaf files: clean.
    - `cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test`: 3 and 20 passed.
    - The temporary-edit runs: `check_only_test` 4, `driven_resolve_test` 11, `offline_reason_test` 3, `cli_invocation_test` 14, `driven_e2e_test` 11 and `hold_single_path_test` 5, all passed. The edits were restored, and `git status` on `crates/*/tests` is clean.
  - Files read: the brief (its decisions, where the code goes, Reuse, Acceptance and Blast radius), `handoff-A.md`, `handoff-T-red.md`, and every test target named in it.
  - Source facts F relies on are in `docs/handoffs/839/evidence.md`.

## T (Phase 7, verify GREEN), 2026-10-12 09:41 MDT

- **Decided:**
  - **All six of F's "tests that look wrong" (T-1 to T-6) were T's errors, repaired in the tests alone.** Each diagnosis was checked against the source (`circuit.rs:306-307`, `support/mod.rs:259-261`, `task.rs:168-172`, `hold_single_path_test.rs:35`) and, for T-6, against the brief's P3 text. No production file was edited.
  - **T-3 skips `tests.rs` and `*_tests.rs` in `sources()`** rather than rewriting the expected values in `panes/gate/tests.rs`: the scan's intent is non-test code, and the filter also covers the other out-of-line test modules.
  - **T-5 waits for the first attach's `GET /session/{id}`, then for the row.** This keeps the guard that the row is visible before the task runs.
  - **Verdict PASS:** the workspace is green (2721 passed, 0 failed) and Tier 2 has no blocking finding.
- **Assumed:**
  - Whether a grant spent on a prompt the pane gate then refuses (a race only) matters is O's to judge; it is advisory.
- **Hedged:**
  - The `rebind_test` race fix is shown by 15 clean runs alone, not a proof; the full-suite run was also clean.
  - `profile_rename_faults_test` was not re-run alone; it passed in the full suite.
- **Evidence:**
  - `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 2721 passed, 0 failed, 36 ignored.
  - Clippy `-D warnings`, `scripts/lint.sh`, `scripts/changelog-check.sh origin/main`, `cargo machete`, `scripts/test-hooks.sh`, `docs_cli_test`, `docs_pane_control_test`, `wire_selftest`: all pass.
  - Eight production mutations, each failing its test (table in `handoff-T-green.md`), each restored.
  - Two evidence entries appended to `evidence.md`.

## A, anti-duplication gate (Phase 7): 2026-10-12T09:55-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 6 `warn` findings (`docs/handoffs/839/handoff-A-dup.md`). The diff is
  `4e691d9..8d3e7fa`; F's change set is `949115e..d480424`.
  - **F extended every object the Reuse map named:**
    - `say`'s prelude moved into `talk::admit`, and `query`'s send-and-wait moved into the one private `forward`;
    - the hub's and the CLI's pane predicates were replaced by `holler_pane::gate`, and `first_refusal` is shared with
      `FakeHub`;
    - `activity_refusal` was extracted from `check_idle`, and `tx_switch/plan.rs` is a move;
    - `control_say.rs` is a whole move with one mapping, and `control_rebind.rs` follows `control_hold.rs`;
    - `wiring/hub.rs` reuses `store.rs`'s four items and the accept rule;
    - `FakeHub` runs over `FaultSwitch`, and `live/tests.rs` is a pure move.
  - **The warns:**
    1. `resolve_prompt_target`'s id branch copies `resolve_session`'s scan-and-classify scaffolding (and
       `offline_advertises` copies `matches_offline`).
    2. `FakeHub::why` and its messages copy the hub's `pane_state_refusal` words and the client's `session-held` text.
    3. The fast-path order (hold, then pane gate) is spelled in both `talk::admit` and `interrupt`.
    4. `wiring/hub.rs::failed` repeats `store.rs::call`'s failed-exchange closure.
    5. `driven_support::id` re-types `fixture::session_id`, and the observed-pane fixture builder is written seven times.
    6. The test kit's `hub.rs` is 317 lines against the brief's "under 250".
- **Assumed:**
  - A justified new object whose internals copy a neighbour's scaffolding is a `warn`, not a `block`. The rule
    blocks a parallel path that the brief did not justify, and the brief justified `resolve_prompt_target` and
    `FakeHub` in writing (D1, D10).
  - Copying small test helpers across test binaries is the repo's own habit (`attach_mode_test::join_fresh` says so).
    So those copies are warns, except where a shared helper was already reachable (`session_id`).
- **Hedged:**
  - I ran no build or test: T-green's full run (2721 passed, 0 failed) is the runtime evidence, and this gate reviews
    structure.
  - Finding 4's fix needs `store.rs` changed beyond visibility, which the brief ruled out for this story. So it is
    offered as a follow-up.
- **Evidence:**
  - **Read:** the brief's decision text, "Where the code goes", "Reuse", Acceptance and Blast radius; `handoff-A.md`,
    `handoff-F.md`, `handoff-T-green.md` and this journal.
  - **The diff:** every production file of `4e691d9..8d3e7fa`, in full or as a diff. The neighbours: `live.rs`,
    `roster.rs`'s forward-fills, `control_hold.rs`, `store.rs`, `say_cmd::run`, the body's `handle_answer` and
    `busy_ages`, the test kit's `fixture.rs`, `pane_store.rs` and `harness.rs`, and the test support of the CLI and
    the hub.
  - **Commands:**
    - `git diff`, `git show --stat` and `git merge-base`;
    - `grep -rn` for the copies above, and `wc -l`;
    - `diff -w` of the moved `live` tests against the removed block;
    - a scan of the added lines for personal infrastructure names (none);
    - the overlap of `origin/main`'s `6eff9cc` with this branch: five files, and no `Ports` literal.

## S, spec audit: 2026-10-12T10:09-06:00 (MDT)

- **Decided:** REWORK (`docs/handoffs/839/handoff-S.md`), production and test-only together, so `reworkKind:
  production`.
  - **Acceptance 1 to 17 and 19 to 22 pass,** each pinned by a named test that asserts behaviour. ADR-0021 section 11
    matches the brief word for word.
  - **Acceptance 18 is partial (REWORK 1, test-only).** No test runs a `--pane` verb against an operator or default
    hold (P5, exit 4), or reaches `PaneHeld` through the hub's gate under `--json`. Of the three verbs, only `say
    --pane` is pinned to answer `pane-not-driven`, and only for an unknown id. Nothing pins `interrupt` or `answer`,
    the arm ahead of the ambiguous exit 2, or an offline body at the verb.
  - **One silent deviation from D3's untrusted-text rule (REWORK 2, production).** `HubClient::held` prints the body's
    advertised roster name unquoted, and spells `holler release <name>` from it.
  - **Acceptance 20(d) row (1) is ruled, not held.** The answer is `session-of-other-pane`, as built. D3 step 1 and
    ADR section 11 point 3 (the contract text) put the refusals of section 8 before the driver, and P6 keeps
    `session-of-other-pane` for "another pane's session of record". The acceptance clause overstates P6 for that row,
    and both answers refuse with nothing moved.
- **Assumed:**
  - T-green's run (2721 passed, 0 failed) and its eight mutations are the runtime evidence. As S's role says, I re-ran
    no build or test. `git diff --stat d480424 8d3e7fa` and `8d3e7fa e83ea9c` confirm that what T-green verified is
    what I audited.
  - `answer` has no held arm (E125; `talk::answer` checks no hold), so P5's exit-4 pin covers `say` and `interrupt`
    only.
- **Hedged:**
  - REWORK 2 is low severity: it needs a body that advertises a crafted session name and holds a valid token. It is
    marked rework because the brief states the rule outright and F applied it in every other new message.
  - The ADR-0021 merge conflict with `origin/main` (#853 took "Decisions taken" 32 and 33) is advisory for O. The
    brief's clash rule (2) says #839 renumbers to 34 to 47 when it merges second.
- **Evidence:**
  - **Read:** the brief (decisions, ADR text, placement, Reuse, Acceptance, Blast radius, clashes, the rules); the
    issue (`gh issue view 839`); every phase handoff, `evidence.md` and this journal; `CONTRIBUTING.md`.
  - **The diff:** `git diff origin/main...HEAD` for every production file; the tests named in the table of
    `handoff-S.md`.
  - **Commands:**
    - `wc -l` on every touched `.rs` file;
    - `grep` of the added lines for `unwrap`, `expect`, `panic`, `#[allow]`, `unsafe` and `set_var`;
    - `grep` for personal infrastructure names and secrets;
    - `git show origin/main:` for the old `send_prompt` and `parse_session_id`;
    - `git diff --name-status` of the goldens;
    - a whitespace-normalised comparison of ADR section 11 and of section 8's replacement with the brief;
    - `git merge-tree --write-tree HEAD origin/main`, for the ADR-0021 conflict;
    - `git log` for the commit trailers.
