# Handoff-S: #839 - DRIVEN follows each pane's session of record  (spec audit)

**Date:** 2026-10-12 (10:09 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`), head `e83ea9c`. The code audited is
`4e691d9..8d3e7fa`, the tree T-green verified; `8d3e7fa..e83ea9c` adds only `handoff-A-dup.md` and its journal entry.
**Issue:** #839 (absorbs #828)
**Brief:** `docs/handoffs/839-brief.md` (decisions, ADR text, placement, Reuse, Acceptance 1 to 22, Blast radius)
**Handoffs reviewed:** `handoff-A.md` (PASS, 7 warns), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`,
`handoff-A-dup.md` (PASS, 6 warns), `evidence.md`, `decisions.md`

**Verdict: REWORK** (production and test). There are two required changes:

1. Acceptance 18 lists pins that no test writes. This item is test-only.
2. One new message breaks the brief's rule that advertised text is untrusted. This item is production.

Everything else matches the brief, including ADR section 11 word for word. Advisory note 1 is not a rework item, but it
must be done before merge: the branch now conflicts with `origin/main` in ADR-0021.

## A precondition

**Met.** The fourth up-front pass returned PASS with 0 `block` and 7 `warn` (`handoff-A.md`), and the
anti-duplication gate returned PASS with 0 `block` and 6 `warn` (`handoff-A-dup.md`).

## T precondition

**Met.**
- **RED:** valid (`handoff-T-red.md`). It is compile-level for the contract and assertion-level for `check_only`, the
  offline reason, the catalog count, the three scans and the docs.
- **GREEN:** no blocking issue (`handoff-T-green.md`). The run was `cargo test --workspace --no-fail-fast -- --skip
  roster_stays_accurate_under_concurrent_body_load`: 2721 passed, 0 failed.
- **Other Tier 1 checks:** clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete` and `test-hooks.sh`
  all pass.
- **Mutations:** eight production mutations each failed their tests.
- **Test repairs:** T-1 to T-6 changed test code only. `git diff --stat d480424 8d3e7fa` touches no `src/` file.

## Acceptance criteria

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | I2 extended to DRIVEN | `pane_verbs/driven/order.rs`: `a_switch_re_points_the_body_between_the_observation_and_the_record`, `a_reset_re_points_the_body_to_the_session_it_created` (`assert_all_agree`: the record, SHOWN, the TUI, the body's id and DRIVEN are one id; generation +1; one `cas_put`) | PASS |
| 2 | The step order | The same two tests: the hub log is `[Advertised, Rebind]`, the rebind is not yet of record, and select comes before shown. `a_failure_before_the_rebind_leaves_the_body_where_it_was`. `first.rs::reset_first_on_a_healthy_pane_is_sent_through_the_hub` (`[Advertised, Prompt, Rebind, Prompt]`). `a_driven_switch_asks_the_harness_for_no_activity` (no `send_prompt`). T-green's order mutation fails them. | PASS |
| 3 | A message reaches the real session of record | `driven_e2e_test.rs`: `say_pane_reaches_the_pane_s_real_session_of_record` (the id's `prompt_async`, the reply, `say --pane Q` exits 3 `pane-not-driven`, `answer --pane` resolves) and `interrupt_pane_resolves_the_same_id` | PASS |
| 4 | The gate refuses a real-shaped pane | `holler-pane/tests/gate_test.rs` (8); `circuit/dispatch.rs` `hold_tests` (every variant, the recorded-DRIVEN key and its window, the name is no key, spawn is never gated, fail closed, the hold first); `panes/gate/tests.rs` (5) | PASS |
| 5 | Resolution | `driven_resolve_test.rs`: the three resolve tests and `say_interrupt_and_answer_accept_an_id` | PASS |
| 6 | `doctor` | `pane_verbs/driven/doctor.rs`: the reverted body, the relaunch with its window, the agreeing body, the failed `advertised`, unproven, and another pane's body. `driven_test.rs`'s table. | PASS |
| 7 | `reset --first` obeys the gate | `pane_verbs/driven/first.rs`: the three gate codes, operator and default holds, no body and a body that is behind, healthy (sent with `Send`, no harness prompt), and the race. Its last sentence (the `--pane` verbs keep exit 4) is pinned under 18. | PASS |
| 8 | Rebind failures | `order.rs`: `a_refused_rebind_is_a_failure_after_the_act_that_records_nothing`, `a_busy_or_questioning_driver_is_refused_before_anything_moves`, `the_activity_table_is_the_same_through_the_harness_and_the_body`, `a_driven_switch_asks_the_harness_for_no_activity` (E85) | PASS |
| 9 | The body | `holler-body/tests/rebind_test.rs` (8: the happy path, the refusals, busy, the 5 s bound with no late swap, one source after `session.error`, `query/support`, restart, the first moment); `task/rebind_tests.rs` (2); `driven_resolve_test.rs` `control_rebind_*` (3, including the other row's state left as it was) | PASS |
| 10 | `check_only` | `holler-hub/tests/check_only_test.rs` (4: admitted with nothing moved, no grant consumed, the seven-case table through `say` and the check, the redirect from A's warn 3) | PASS |
| 11 | The protocol | The catalog count test (23 rows). `golden_test.rs` covers three goldens, all new (`git diff --name-status` shows A three times and no M). The `codec_test` arm. `scripts/` is untouched. F's `lint.sh` output lists three `(new file)` lines. `v2.md` has the row and the `-32004` sentence. | PASS |
| 12 | Holds across a reset | `first.rs::a_hold_carries_across_a_reset_and_stops_only_the_prompt`; `driven_e2e_test.rs::the_hold_key_survives_a_rebind_and_release_lets_the_first_message_through`; `driven_resolve_test.rs` (`holds.keys()` is empty after a rebind) | PASS |
| 13 | No bypass | `driven_e2e_test.rs::the_generic_forward_cannot_carry_a_rebind`; `driven_resolve_test.rs::the_generic_forward_refuses_session_rebind`; `hold_single_path_test.rs` (three new scans; the two old ones unchanged). `admin/query` is not exercised (advisory 8). | PASS |
| 14 | One predicate | `pane_verbs/driven.rs::the_routed_verbs_refuse_by_the_one_predicate`; `panes/gate/tests.rs::the_frame_is_the_one_predicate_s_code_for_the_pane_on_the_id`; `hold_single_path_test.rs::the_pane_codes_and_the_pair_rule_are_defined_once` | PASS |
| 15 | Fixtures | `pane_verbs/driven.rs::the_session_id_fixture_is_never_a_name` | PASS |
| 16 | Scenario | `pane_integration/scenario.rs::reset_with_a_first_message`, rewritten as D9 says (compiled, opt-in) | PASS |
| 17 | Docs | `docs_pane_control_test.rs::the_sentences_driven_replaced_are_gone_and_the_entry_is_written`. ADR-0021 section 11 matches the brief's text word for word (checked with whitespace normalised). The section 8 replacement matches apart from "(the next paragraph of the ADR)", which reads "(the next paragraph)" inside the ADR. | PASS |
| 18 | A bare id is not a SESSION; P7; P5 | **Pinned:**<br>- the bare id: `a_typed_bare_id_is_unknown_with_no_hub`, `a_typed_bare_id_is_unknown_and_a_session_is_no_pane`;<br>- an id sent straight to `control/say`: `say_interrupt_and_answer_accept_an_id`;<br>- the `classify` table: two tests in `pane_verbs/driven.rs`;<br>- (i) and (ii): `a_turn_that_outlives_its_wait_is_not_pane_not_driven`;<br>- (iii): `a_send_returns_once_accepted_while_the_turn_runs_on` and `the_send_call_carries_the_turn_budget_and_waits_only_for_acceptance`;<br>- the wire reason: `offline_reason_test.rs` (3).<br>**Not pinned:**<br>- P5's "the routed verbs keep their holds": no test runs a `--pane` verb against an operator or default hold (exit 4), or prints `PaneHeld` from the hub's gate under `--json`;<br>- `pane-not-driven` "in all three verbs": only `say --pane` is pinned, and only for an unknown id. Nothing pins `interrupt --pane` or `answer --pane`, the `NotDriven` arm ahead of the `is_ambiguous` exit 2, or an offline body at the verb. | **PARTIAL** (REWORK 1) |
| 19 | The gate sees a body that is behind | Both halves are pinned:<br>- on the fakes, with the real `doctor` and `switch`: `doctor.rs::the_gate_sees_a_behind_body_once_doctor_records_it`;<br>- on a real hub: `driven_e2e_test.rs::the_gate_refuses_a_body_on_a_pane_s_recorded_driven`.<br>The conformance suite runs over `FakeHub` (`fake_hub_test.rs`) and over the real client (`the_real_client_passes_the_hub_conformance_suite`). | PASS |
| 20 | Drivers need evidence | `driven_test.rs` (the table, rows (1) to (4) of (d) included); `pane_verbs/driven/evidence.rs` (7); the `doctor.rs` cases. Row (1) of (d) through the verbs is `session-of-other-pane` (`a_switch_onto_another_pane_s_record_is_refused_by_the_existing_rule_first`); see the ruling under Spec compliance. | PASS |
| 21 | One request at a time | `driven_resolve_test.rs`: `concurrent_forwards_on_one_body_each_get_their_own_reply`, `a_caller_waiting_for_the_slot_times_out_at_its_own_bound`, `the_hub_s_rebind_bound_is_below_the_cli_s`. `driven_e2e_test.rs`: `a_rebind_and_queries_at_once_each_get_their_own_reply`, `the_rebind_bounds_nest` (5, 6, 8 and 10 s). | PASS |
| 22 | A single-process pane asks the hub nothing | `pane_claude/driven.rs` (2); `pane_claude/lifecycle.rs::reset_first_on_a_claude_pane_is_not_driven_and_stops_nothing` (E91 rewritten). Advisory 9 covers the doctor case. | PASS |

## Spec compliance

**Implemented as decided.**

| Decision | How the code matches it |
|---|---|
| P1 | `LiveHandle::query`'s allowlist and its comment are byte-identical to `main`. `rebind` rides the private `forward` with the literal method. `circuit.rs` takes +0 lines. There is no `admin/rebind`. |
| P2 | `Inner.config.session_id` is the one source. `mirror_session_id` writes it in the first attach, the re-attach and the rebind, and `start` initialises it. `live_configs` feeds `query/support`. A restart reverts. |
| P3 | ADR 0014 (point 5 and Consequences), ADR 0005 section 3 and ADR 0003's Addressing line and header are amended. `route_target_by` refuses a non-name `SESSION` before any hub. |
| P4 and P8 | `driver_of` applies the exclusion rule and its exception, then rules (i) to (iii), then the fallback, which is `Unproven` on a shared listing. This is checked line by line against D3. |
| P5 | `classify` decides no exit. The `--pane` arms act on `NotDriven` alone, first. |
| P6 | `elsewhere` and `refuse_elsewhere` are in the plan. |
| P7 | `Error::offline` sits beside `invalid_grant`. It is used on the resolve-time offline arms of `say` (`control_say::refusal`), `interrupt` and `answer` alone. `Timeout`, `Cancelled`, `AckTimeout` and `PromptFailed` stay bare. |
| D1 | `resolve_prompt_target` sits in `live/target.rs`. The gate keys on `harness_session` through `send_turn`, the one `SayOpts` builder. `first_refusal` serves both the hub and `FakeHub`. |
| D2 | The body refuses on its deadline first, then on a spawn session, then on busy. It then attaches under `timeout_at` and swaps only on success. The bounds are 5, 6, 8 and 10 s. `control/rebind` takes `token_id` and fills one roster row (A's warn 1). |
| D3 | `HubPort` has three required methods. The order is plan, act, observe, drive, record, first message. A single process takes `Driver::None` and asks the hub nothing. Reconcile reads `hub.advertised` once per pass, sets the switch remedy after `with_fix` and uses `without_remedy` for ambiguous or unproven drivers. |
| D4 | The hold key is unchanged, and a rebind sets no hold. |
| D5 | `talk::admit` is `say`'s prelude, moved and not copied. `check_only` answers `{admitted: true}`. `Send` is built as `say_cmd::run` builds the call (600 s budget, 2 s accept wait). |
| D6 | DRIVEN is `None`, and `--first` is refused before anything moves. |
| D7 | The three codes are declared in the files that own them. |
| D8 | The fixtures are `ses_` ids, built by `session_id(n)`. |
| D9 | The scenario step is rewritten as decided. |
| D10 | `FakeHub` runs over `FaultSwitch<HubOp>` and `first_refusal`. |
| D11 | No file is at 900. |
| D12 | `cli-surface.txt` is untouched. |

**Explained deviations, accepted.**
- A's warns 1, 3, 5 and 7 were taken: the one-row roster fill, the pane gate before an interrupt's cancel,
  `parse_session_id` moved to `findings.rs`, and `control_say.rs` kept whole.
- `tx_switch/plan.rs`: the brief's own split rule at 650 lines.
- `panes/gate/tests.rs` in place of `panes/gate_tests.rs`: a child module needs no `#[path]`.
- `HubRefusal::Held` has no `key`: the `-32011` data carries none.
- `control/rebind` checks `parse_session_id` first: an extra fail-closed check.
- `FakePaneStore::records()`, which T's contract requires.
- The README's debug-table row for the new `rebind` event.
- The `--pane` mismatch wording "its body drives".

**Ruling on Acceptance 20(d) row (1)**, held open by T-red, F, T-green and A-dup. The brief says two things:
- the acceptance text says a switch of P1 onto the id of a body on P2's **session of record** is `session-driven-elsewhere`;
- D3 step 1 runs "the existing refusals; then" the driver.

The contract text F copied into the ADR says the same as D3 (section 11 point 3: "plan (the refusals of section 8, then
the driver, ...)"). P6 itself keeps `session-of-other-pane` for "the target is another pane's session of record"
(E153). So `check_unclaimed` refuses first, and row (1) is `session-of-other-pane`, as built and tested. Row (2) (P2's
recorded DRIVEN) is `session-driven-elsewhere`.

Both answers refuse with nothing moved, so the property the sentence protects holds: the `Unproven` remedy never pairs
a body another pane's evidence proves. **No change.** The acceptance clause and D3's "one consequence" sentence
overstate P6 for that one row. The contract governs. Under the standing rule to decide and report, this is decided
here, not held.

**One silent deviation (REWORK 2):** `HubClient::held` breaks D3's untrusted-text rule (see Quality audit).

## Quality audit

**Correctness and failure handling: sound.**
- Every new refusal fails closed and moves nothing: the body's five rebind refusals, `control/rebind`'s params, id,
  token and cached session, and the plan's refusals.
- A late attach is never swapped in. The swap is after `timeout_at`, and the old driver's `shutdown` is a send and an
  abort, so it cannot delay the reply.
- A record is one compare-and-swap at the plan's generation. Reconcile writes DRIVEN in the same write as SHOWN.
- `control/rebind` fills one roster row, so the whole-set race of A's warn 1 is gone.
- `query_slot` sits inside each caller's own `timeout`, and replies match by correlation id.
- The grant spent before the pane gate refuses (T-green's advisory 3) is `main`'s order, not new (advisory 5).

**Build guards: met.**
- No production `unwrap`, `expect` or `panic`. Every hit is in `rebind_tests.rs`, `panes/gate/tests.rs` or
  `hold_tests`, under a linked allow.
- Every new `#[allow]` carries `// #839`.
- The largest touched files are `circuit.rs` 898 (+0), `pane_verbs/switch.rs` 895 and `roster.rs` 893. None is at 900.
- No new `unsafe`, and no dead code (clippy `-D warnings`).

**Protocol: additive only.**
- One new request, `session/rebind`; the catalog has 23 rows.
- An optional `data.reason: "offline"` on `-32004`, and an optional `check_only` on `control/say` and `admin/say`.
- `v2.md` updates the `-32004` and `-32009` rows, the `data` sentence, the presence sentence, the pane hold paragraph,
  the "Re-pointing" paragraph and the `pane/*` note.
- Three new goldens, and no existing golden changed.

**Tests: meet the bar.**
- Cross-process behaviour runs on a real hub and body against the fake OpenCode server (`driven_e2e_test.rs`, ephemeral
  port, throwaway state).
- Waits are bounded poll loops. The fake server's stall mode polls a flag. There are no fixed sleeps used as
  synchronisation.
- RED-first evidence is in `handoff-T-red.md`.
- Gaps: REWORK 1.

**Documentation: complete.**
- `CHANGELOG.md` has an `## [Unreleased]` entry linking #839 and #828, and rewrites the #649 clause.
- The README's `session` row names the new `rebind` event.
- `docs/pane-control.md` has the `reset`, `park`, DRIVEN, cutover, "Later" and section 9 edits.
- The edits listed for ADR-0021, 0014, 0005 and 0003 were checked against the brief's table.

**Public-repository privacy: clean.** I grepped the added lines for IPs, `ts.net`, tailnet, host and account names,
home paths, emails and keys. The only hits are a test message id (`m1`) and evidence excerpts quoting the existing
placeholder `remote-a.example.ts.net`.

**Message hygiene: one finding.** `crates/holler-cli/src/pane/wiring/hub.rs:173-192` (`HubClient::held`, new) puts
`body.name` into the `session-held` message unquoted, twice. That name is the roster name of the body session, and
its session part is the body's advertised `SessionAd.name`, an unvalidated wire `String` that the hub's roster does
not check. The message also spells the command `holler release {name}` from it.

D3: "every advertised name and id a message prints goes through `findings::quoted` ... and the command in these
messages is built, never spelled". F applied this rule everywhere else:
- `drive`;
- `find_driver`;
- `check_first`;
- `refuse_elsewhere`;
- `driven::named`;
- reconcile's `check_driven`.

Pane errors print raw in text mode (`output::emit_text`), so a body that advertises a crafted name reaches the
operator's terminal through the one message that hands them a command to run.

**Commit and PR hygiene: no PR yet.**
- Subjects are Conventional Commits.
- The script's trailers are `Co-Authored-By: Claude <noreply@anthropic.com>`, with no session link. `main`'s
  history shows the same for script commits, so this is the pipeline's behaviour, not this story's.
- The PR is not open yet. The AI disclosure goes in after the script opens it (`CLAUDE.md`).

## Scope check

**What the brief asked for is delivered.**

**Edits outside the Blast radius, each explained by F:**
- `holler-hub/src/roster.rs` (+1) and `roster/rebind.rs` (new), for A's warn 1;
- `holler-pane/src/tx_switch/plan.rs`, the brief's split rule;
- `holler-pane-testkit/src/pane_store.rs`, for T's contract;
- `README.md`, for the new log event.

**Test files split for size:**
- `pane_verbs/driven/{order,first,evidence,doctor}.rs` and `pane_claude/driven.rs`;
- `holler-hub/tests/check_only_test.rs` and `driven_support/`.

**Sizes above the brief's estimates, none at 900:**
- the test kit's `hub.rs` is 317 lines (the brief said "under 250");
- `say_cmd.rs` is 588 ("under 560");
- `pane_verbs/switch.rs` is +2 ("no growth");
- `roster.rs` is +1 ("no growth").

**No unrelated refactors.**

**Under-delivered:** Acceptance 18's pins (REWORK 1).

## Verdict

**REWORK** (`reworkKind: production`: one production change plus test-only additions).

1. **TEST-ONLY (no `src/` change; T writes it): Acceptance 18's unpinned clauses** (and Acceptance 7's last sentence).
   Add them to `crates/holler-cli/tests/driven_e2e_test.rs`, which already has a real hub, a real body and `--join-held`
   support.
   - **(a) P5, holds keep exit 4.** Hold the body that drives P, first with an operator hold (`holler hold kiwi/oc`),
     then with a default hold (a hub started with `--join-held kiwi/*`). Under each hold:
     - `holler say --pane P TEXT` and `holler interrupt --pane P TEXT` exit 4 (`HELD_EXIT_CODE`) with the held refusal;
     - stderr contains neither `pane-not-driven` nor `session-held`.

     `answer` has no held arm (E125), so it is not in this pin.
   - **(b) P5, a pane hold prints `PaneHeld`.** Seed P healthy, and a second pane that is parked and whose recorded
     DRIVEN is P's id. `holler say --pane P --json TEXT` prints the `PaneHeld` object and exits 3, because the hub's
     gate refuses for the parked pane.
   - **(c) `pane-not-driven` in all three verbs, first.**
     - `interrupt --pane Q` and `answer --pane Q 0` exit 3 with `pane-not-driven` for the pane no body drives, as `say
       --pane Q` already does at `:244`.
     - A pane whose id two live attach sessions advertise (`-32003` `ambiguous`; for example `Rig::start(&[("oa",
       &id), ("ob", &id)])`) exits 3 `pane-not-driven` for `say --pane` and `interrupt --pane`, not the
       `is_ambiguous` exit 2 (`interrupt_cmd.rs:200`, `say_cmd.rs:212`).
     - A pane whose body is offline (stop the rig's body; the hub answers `-32004` with `data.reason: "offline"`)
       exits 3 `pane-not-driven`.
2. **PRODUCTION (F): `crates/holler-cli/src/pane/wiring/hub.rs:173-192`, `HubClient::held`.**
   - Name the session with `findings::quoted`.
   - Give the command `holler release <name>` only when the name parses as a `holler_proto::RoutableName`, whose
     grammar has no space or shell metacharacter. Otherwise keep the existing fallback sentence, which spells no name.
   - Keep D4's content: the kind, the reason and the release.
   - `first.rs::reset_first_on_a_held_pane_is_session_held` (`contains("holler release kiwi/demo-c1r1")`) must stay
     green. T-green re-runs the `session-held` tests.

## Advisory notes (non-blocking)

1. **Before the PR can merge (O or the run's agent): the branch conflicts with `origin/main`.** `origin/main` moved
   to `6eff9cc` (#853) after this branch's base. `git merge-tree` reports one conflict, in `docs/adr/ADR-0021.md`,
   "Decisions taken": #853 took items 32 and 33. By the brief's clash rule (2), whichever story merges second
   renumbers, so #839's items 32 to 45 become 34 to 47, after #853's. Nothing elsewhere cites those numbers.
   `CHANGELOG.md`, `ADR-0003.md`, `pane-control.md` and `profile_verbs/apply/rig.rs` merge cleanly.
2. **A's Phase 3 warn 2 (P8, row (4)):** both panes get `One` for one body. This is built and pinned as P8 decided
   (`driven_test.rs`, row 4). It is the operator's to revisit, and it is not reopened here.
3. **A-dup's warns 1 to 6 are follow-ups.** None changes behaviour:
   - the resolve scaffolding written twice;
   - `FakeHub`'s copied words;
   - the fast-gate order spelled in two places;
   - `failed()`'s composition in two files;
   - re-typed fixtures and `driven_support::id`;
   - the test kit's `hub.rs` at 317 lines.
4. **T-green's advisory 2: a slow failure.** A gate regression makes `driven_e2e_test.rs` take about ten minutes to
   fail, because the conformance suite's real-client `Send` has a 600 s budget. A shorter budget there is a
   follow-up.
5. **T-green's advisory 3 predates this story.** `send_prompt` running `holds.admit` before the pane gate (a grant
   spent on a prompt the gate then refuses, in a race only) is `main`'s order, unchanged by this story.
6. **The body's rebind reason is broader than D2 names.** It reports every attach failure as `-32602` `"session not
   found on the endpoint"`, a refused connection or an HTTP error included. The message carries the driver's own text,
   and the CLI treats every rebind refusal as `unavailable`, so nothing is mis-mapped. A distinct reason is a possible
   follow-up.
7. **Two other pane messages pass hub or body text through raw.**
   - `HubClient::refused` passes the hub's text for `Busy`. For `input-required` that text is the harness's question
     (`control_say::refusal` sets `err.message = q`). The existing `say` busy hint deliberately does not print it.
   - `reason_of` passes the body's refusal text.

   D5 says these carry the hub's text, so they are not rework. Escaping them (`quoted`, or an `embedded`-style escape)
   is a cheap follow-up. Both are reachable only in a race after the plan's own refusals.
8. **Acceptance 13's `admin/query` form is not exercised.** It reaches the same `hub_query_remote` (E99), whose
   allowlist is pinned directly (`driven_resolve_test.rs` calls `LiveHandle::query("session/rebind")`). T-red named
   this gap.
9. **Acceptance 22, the doctor case.** `doctor_leaves_a_claude_pane_s_driven_as_stored` pins DRIVEN as stored. One
   more line (`run_logged`, then `calls.hub.is_empty()`) would also pin "makes no `hub.advertised` observation".
   Optional.
10. **`profile_rename_faults_test` (#665)** is an inode-reuse flake that predates this story. It needs its own issue.
