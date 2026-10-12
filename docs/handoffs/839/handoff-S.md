# Handoff-S: #839 - DRIVEN follows each pane's session of record  (spec audit, S rework round 1)

**Date:** 2026-10-12 (10:57 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`), head `3f8b0bd`, working tree clean.
This cycle's code is `fde8eb8..ee874c1`:
- F `fde8eb8..9eb55d4`: production only;
- T-green `9eb55d4..ee874c1`: tests only;
- `ee874c1..3f8b0bd`: A-dup's handoff alone.

**Issue:** #839 (absorbs #828)
**Brief:** `docs/handoffs/839-brief.md`
**Reworks:** round 1's REWORK, which audited the whole story (`4e691d9..8d3e7fa`): `git show
fde8eb8:docs/handoffs/839/handoff-S.md`. This file covers the rework and re-checks the whole story's gates. Rows and
rulings that did not change are carried from round 1, which holds their full evidence.
**Handoffs reviewed:**
- `handoff-A.md`: PASS, 7 warns;
- `handoff-T-red.md`: RED valid;
- `handoff-F.md`: this round;
- `handoff-T-green.md`: this round, GREEN;
- `handoff-A-dup.md`: this round, PASS, 6 warns carried;
- `evidence.md` and `decisions.md`.

**Verdict: PASS.** Both REWORK items are closed:
1. **REWORK 1 (test-only).** Three new process-level tests pin every clause of Acceptance 18's P5 and of Acceptance 7's
   last sentence.
2. **REWORK 2 (production).** `session-held` now quotes the advertised name. It builds `holler release NAME` only from a
   routable name, in one builder that both ports call.

There is no new blocking finding. Advisory 1 must still be done before the PR can merge: the branch conflicts with
`origin/main` in ADR-0021.

## A precondition

**Met.**
- **Up-front pass:** PASS, 0 `block`, 7 `warn` (`handoff-A.md`).
- **Anti-duplication, this cycle:** PASS, 0 `block`, 6 `warn` (`handoff-A-dup.md`). All six are carried from the first
  pass, and this cycle half-fixes warn 2.

## T precondition

**Met.**
- **RED:** valid (`handoff-T-red.md`), unchanged.
- **GREEN this round:** no blocking issue (`handoff-T-green.md`).
  - The run was `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`:
    2725 passed, 0 failed, 36 ignored. That is round 1's 2721 plus the four new tests.
  - `driven_e2e_test` ran four times, with 14 passed each time.
- **Other Tier 1 checks:** these pass:
  - clippy `-D warnings`;
  - `lint.sh` and `changelog-check.sh`;
  - `cargo machete` and `test-hooks.sh`;
  - `git diff --check`.
- **Mutations instead of RED.** The new tests were written after the code, so they have no RED. T-green made six
  temporary production mutations instead, and each one failed its test:
  - `classify` stops mapping the offline reason;
  - `classify` stops mapping the ambiguous reason;
  - `interrupt_cmd` loses its `pane_not_driven` arm;
  - a hold is read as `NotDriven`;
  - there is no `PaneHeld` arm;
  - `session_held` spells the raw name.
- **Roles kept apart.** `git diff --stat` per commit shows that F's commit touches only the three production files and
  T-green's touches only the two test files.

## Acceptance criteria

**How rows 1 to 22 were checked.**
- No test was removed or weakened. T-green's diff adds four tests and edits only two module docs and two imports.
- No production file changed outside the three that REWORK 2 touched.
- So round 1's evidence stands for every row it passed, and every row is green in T-green's run.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | I2 extended to DRIVEN | `pane_verbs/driven/order.rs`: `a_switch_re_points_the_body_between_the_observation_and_the_record`, `a_reset_re_points_the_body_to_the_session_it_created` | PASS |
| 2 | The step order | The same two tests, plus:<br>- `a_failure_before_the_rebind_leaves_the_body_where_it_was`;<br>- `first.rs::reset_first_on_a_healthy_pane_is_sent_through_the_hub`;<br>- `a_driven_switch_asks_the_harness_for_no_activity`. | PASS |
| 3 | A message reaches the real session of record | `driven_e2e_test.rs`: `say_pane_reaches_the_pane_s_real_session_of_record`, `interrupt_pane_resolves_the_same_id` | PASS |
| 4 | The gate refuses a real-shaped pane | `holler-pane/tests/gate_test.rs` (8), `circuit/dispatch.rs` `hold_tests`, `panes/gate/tests.rs` (5) | PASS |
| 5 | Resolution | `driven_resolve_test.rs`: the three resolve tests and `say_interrupt_and_answer_accept_an_id` | PASS |
| 6 | `doctor` | `pane_verbs/driven/doctor.rs` (the six cases), `driven_test.rs`'s table | PASS |
| 7 | `reset --first` obeys the gate | `pane_verbs/driven/first.rs` covers:<br>- the three gate codes;<br>- operator and default holds (`session-held`, with `holler release kiwi/demo-c1r1`);<br>- no body, and a body that is behind;<br>- healthy, and the race.<br>**New this round:** its last sentence ("the same operator hold on the `--pane` verbs keeps exit 4") is pinned by `driven_e2e_test.rs::a_hold_on_the_pane_s_body_keeps_exit_4_for_the_routed_verbs` for `say` and `interrupt`. `answer` has no hold behaviour to keep; see the ruling under Spec compliance. | PASS |
| 8 | Rebind failures | `order.rs`:<br>- `a_refused_rebind_is_a_failure_after_the_act_that_records_nothing`;<br>- `a_busy_or_questioning_driver_is_refused_before_anything_moves`;<br>- `the_activity_table_is_the_same_through_the_harness_and_the_body`;<br>- `a_driven_switch_asks_the_harness_for_no_activity`. | PASS |
| 9 | The body | `holler-body/tests/rebind_test.rs` (8), `task/rebind_tests.rs` (2), `driven_resolve_test.rs` `control_rebind_*` (3) | PASS |
| 10 | `check_only` | `holler-hub/tests/check_only_test.rs` (4) | PASS |
| 11 | The protocol | All four are pinned:<br>- the catalog count test (23 rows);<br>- `golden_test.rs` over three new goldens (re-checked: `git diff --name-status` shows A three times, no M);<br>- the `codec_test` arm;<br>- `v2.md`'s row and the `-32004` sentence. | PASS |
| 12 | Holds across a reset | `first.rs::a_hold_carries_across_a_reset_and_stops_only_the_prompt`; `driven_e2e_test.rs::the_hold_key_survives_a_rebind_and_release_lets_the_first_message_through`; `driven_resolve_test.rs` | PASS |
| 13 | No bypass | `driven_e2e_test.rs::the_generic_forward_cannot_carry_a_rebind`; `driven_resolve_test.rs::the_generic_forward_refuses_session_rebind`; `hold_single_path_test.rs` (three new scans) | PASS |
| 14 | One predicate | `pane_verbs/driven.rs::the_routed_verbs_refuse_by_the_one_predicate`; `panes/gate/tests.rs::the_frame_is_the_one_predicate_s_code_for_the_pane_on_the_id`; `hold_single_path_test.rs::the_pane_codes_and_the_pair_rule_are_defined_once` | PASS |
| 15 | Fixtures | `pane_verbs/driven.rs::the_session_id_fixture_is_never_a_name` | PASS |
| 16 | Scenario | `pane_integration/scenario.rs::reset_with_a_first_message`, rewritten as D9 says (compiled, opt-in) | PASS |
| 17 | Docs | `docs_pane_control_test.rs::the_sentences_driven_replaced_are_gone_and_the_entry_is_written`; ADR-0021 section 11 matches the brief word for word (round 1) | PASS |
| 18 | A bare id is not a SESSION; P7; P5 | **Round 1's pins:** the bare id with no hub; `control/say` with an id; the `classify` table; (i), (ii) and (iii); `offline_reason_test.rs` (3).<br><br>**New this round, `driven_e2e_test.rs`:**<br>- **`pane_not_driven_comes_first_in_all_three_verbs`.** `say`, `interrupt` and `answer --pane` each exit 3 with `pane-not-driven` in three cases: an unknown id (`-32003`); an ambiguous id (`-32003` `ambiguous`; the test first confirms through the hub's own `check_only` that the hub sees two sessions, so exit 2 would fail it); and an offline body (`-32004` `offline`, after a bounded wait for the hub's answer). No prompt reaches the session.<br>- **`a_hold_on_the_pane_s_body_keeps_exit_4_for_the_routed_verbs`.** The two holds are an operator hold (`holler hold kiwi/oc`) and a default hold (`--join-held kiwi/*`). Under each, `say --pane P TEXT` and `interrupt --pane P TEXT` exit 4. The text carries that kind's tail and neither `pane-not-driven` nor `session-held`. No prompt reaches the session.<br>- **`a_pane_hold_on_the_id_prints_pane_held_for_say_pane`.** P is healthy, and Q is parked with P's id as its recorded DRIVEN. `--json say --pane P` prints exactly `{"error":"session_held","session":<id>,"reason":"pane-parked","hold_kind":"pane"}` and exits 3. No prompt reaches the session. | **PASS** (was PARTIAL) |
| 19 | The gate sees a body that is behind | `doctor.rs::the_gate_sees_a_behind_body_once_doctor_records_it`; `driven_e2e_test.rs::the_gate_refuses_a_body_on_a_pane_s_recorded_driven`; the conformance suite over `FakeHub` and over the real client | PASS |
| 20 | Drivers need evidence | `driven_test.rs` (the table, rows (1) to (4) of (d)), `pane_verbs/driven/evidence.rs` (7), the `doctor.rs` cases; round 1's ruling on (d) row (1) stands | PASS |
| 21 | One request at a time | `driven_resolve_test.rs` (3), `driven_e2e_test.rs::a_rebind_and_queries_at_once_each_get_their_own_reply`, `the_rebind_bounds_nest` | PASS |
| 22 | A single-process pane asks the hub nothing | `pane_claude/driven.rs` (2), `pane_claude/lifecycle.rs::reset_first_on_a_claude_pane_is_not_driven_and_stops_nothing` | PASS |

**REWORK 2's own pin:** `crates/holler-pane/tests/driven_test.rs::the_release_command_is_built_only_from_a_routable_name`.
- **A routable name:** the code is `session-held`, the subject is quoted (`"kiwi/oc" is held (operator hold, reason
  "night")`), and the text contains `run holler release kiwi/oc to lift it`.
- **Names that are not routable** (a `;` command, a space, a newline with ESC, and uppercase), **and no name:** the text
  contains neither the raw name nor `holler release kiwi`, and it names the id, quoted.

## Spec compliance

**REWORK 2 is implemented as instructed.** `holler_pane::hub::session_held`, `crates/holler-pane/src/hub.rs:36-60`:
- **The name is quoted.** The name is printed with `findings::quoted`, which uses `{:?}`: it escapes control characters
  and adds quotes, cut at 64 characters (`error.rs:691-698`). This is D3's "every advertised name and id a message prints
  goes through `findings::quoted`".
- **The command is built, never spelled.** The command is built only from `RoutableName::parse(name)`, whose segments
  are `[a-z0-9-]` joined by `/`, so it carries no space and no shell metacharacter. Any other name, a failed roster read
  and no live body all give the existing fallback, which names the id quoted and spells no name.
- **D4's content is kept:** the kind, the reason (quoted) and the release.
- **`kind` is printed bare, safely.** Only `operator` or `default` reach it: `hub_refusal.rs`'s `held` maps every other
  kind to `Other`, and `FakeHub` passes `HoldKind::as_str()`.
- **The required pin stays green:** `first.rs::reset_first_on_a_held_pane_is_session_held`.

**F's placement is accepted.**
- **What F did:** one builder in `holler_pane::hub`, beside `SESSION_HELD`, called by both `HubClient::held`
  (`wiring/hub.rs:171-182`) and `FakeHub::judge` (test kit `hub.rs:215`).
- **Why it is right:**
  - D7 puts `session-held` in that file as the port's answer.
  - It follows the precedent of `driven::not_driven`.
  - It adds no dependency (`holler-pane` already depends on `holler-proto`).
  - It removes the copy that A-dup's warn 2 named.
- **Outside the named file, with reason.** Round 1 named only `wiring/hub.rs`. The other two files are this story's own
  new files. Without the move, the fake would have pinned words the real client no longer says.

**No other message prints an advertised field raw.**
- I re-grepped `holler-pane/src`, `holler-cli/src/pane` and the test kit for messages built from advertised fields.
- `driven::named`, `plan.rs`'s activity refusal and the other sites round 1 listed all quote.
- The remaining hits are validated pane names and record names.

**Round 1's rulings stand.** That includes Acceptance 20(d) row (1).

**New ruling: `answer` and holds.**
- **What the brief says.** P5 and Acceptance 18 list `answer --pane` among the verbs that keep "the held refusal and
  exit 4" under an operator or default hold.
- **What the code does:**
  - D5 (E125) says `answer_cmd` has no held arm.
  - The hub's `control/answer` checks no hold (`control_server.rs:351-362`). It calls `talk::answer`, which takes only
    the registry.
  - ADR-0021 already says "`answer` sends no prompt and is not gated" (line 784, on `main` too).
  - So `answer --pane` under a hold was never exit 4, on `main` or here.
- **What is built:** P5's rule is "the three `--pane` verbs keep today's hold behaviour", and that is what is built.
  Round 1's REWORK rightly left `answer` out of the pin.
- **Ruling.** The brief's words overstate the behaviour for that one verb, and no code changes. This is decided here
  under the standing rule, not held. It costs nothing at run time, and the contract's own section 8 sentence is right.
  The new section 9 clause repeats the overstatement (advisory 2).

## Quality audit

**Correctness: sound.**
- **Every path out of the builder is safe.** A failed roster read, no live body, and a name outside the grammar each
  fall back to a sentence that names only the quoted id.
- **Arguments:** both callers pass the prompted id and the body's roster name in the right order.
- **The hold key:** the roster name is `label/session` (`roster.rs` `qualified_name`), which is the hold key, so the
  built command lifts the right hold.

**Build guards: met.**
- **No production panic paths.** No `unwrap`, `expect` or `panic` is added in production code. The one `panic!` in the
  whole story's diff is `circuit/dispatch.rs:517`, inside `#[cfg(test)] mod hold_tests` under
  `#[allow(..)] // #442, #646`.
- **Allows:** no `#[allow]` is added without a `// #NNN` link.
- **Sizes:** the largest touched file is `circuit.rs` at 898 (+0 for the story). This cycle's files are 106, 223, 310,
  705 and 423 lines.
- **Dependencies:** none is new.

**Protocol: no change this cycle.** Across the whole story:
- three goldens are added and none is modified;
- `cli-surface.txt` and `scripts/` are untouched (D12).

**Tests: meet the bar.**
- **Real processes:** each new e2e test runs a real hub and a real body against the fake OpenCode server, on an
  ephemeral port with throwaway state.
- **No fixed sleeps:** the waits are bounded `wait_for` loops on the hub's own answer.
- **The cases cannot pass vacuously:**
  - `Rig::start` waits for each row's `harness_session_id`;
  - the ambiguous case first checks the hub's `ambiguous` reason;
  - the offline case waits for the hub's `offline` reason.
- **Delivery:** each test also asserts that nothing reached the session.

**Documentation: nothing more is needed this cycle.**
- No doc quotes the `session-held` text.
- The guide's sentence describes the normal case (advisory 5).
- `CHANGELOG.md` has an `## [Unreleased]` entry linking #839 and #828 (checked in round 1).

**Public-repository privacy: clean.**
- I grepped every added line of `origin/main...HEAD` for IPs (other than `127.0.0.1`), `ts.net`, tailnet, host and
  account names, home paths, email addresses and key shapes.
- The only hits are the existing placeholder `remote-a.example.ts.net`, the epic's rule text, an existing
  `auth_method = "api-key"` doc line, and the trailer address `noreply@anthropic.com`.
- This cycle's new strings are test fixtures, such as `kiwi/oc; rm -rf ~` and an ESC sequence.

**Commit and PR hygiene: no PR yet.**
- This cycle's three commits are Conventional Commits (`chore(#839): …`).
- They carry the script's `Co-Authored-By: Claude <noreply@anthropic.com>` with no session link. Round 1 noted the
  same: it is the pipeline's behaviour, not this story's.
- The PR is not open yet. The AI disclosure is added after the script opens it (`CLAUDE.md`).

## Scope check

**Delivered: exactly the rework.**
- **REWORK 2:** three production files. S named one; F explained the other two (see Spec compliance).
- **REWORK 1:** one e2e file.
- **REWORK 2's pin:** one unit test, which F suggested.

There is no unrelated refactor and no under-delivery.

**Round 1's scope notes stand:**
- the edits outside the Blast radius, which F explained;
- the sizes above the brief's estimates, none at 900. The test kit's `hub.rs` is now 310 lines, down from 317.

## Verdict

**PASS.** All 22 acceptance criteria are met and the implementation is spec-compliant. The quality is acceptable.
Ready for O.

## Advisory notes (non-blocking)

**New this round**

1. **Before the PR can merge (O or the run's agent): the ADR-0021 conflict.**
   - Re-checked at about 10:55 MDT. `git ls-remote` shows `main` still at `6eff9cc` (#853).
   - `git merge-tree` reports one conflict: `docs/adr/ADR-0021.md`, "Decisions taken". #853 took items 32 and 33.
   - By the brief's clash rule (2), #839's items 32 to 45 become 34 to 47. No doc or test cites those numbers.
   - `CHANGELOG.md`, `ADR-0003.md`, `pane-control.md` and `profile_verbs/apply/rig.rs` merge cleanly.
2. **ADR-0021 section 9 overstates for `answer`.**
   - **Where:** line 827, the row "`say`, `interrupt`, `answer` with `--pane`".
   - **What:** the clause #839 added reads "An operator or default hold on the body's session is still the held refusal
     with exit 4, and a pane hold keeps its `PaneHeld` form". It is true of `say` and `interrupt` only (see the ruling).
   - **Proposed:** "for `say` and `interrupt` (`answer` sends no prompt and is not gated)".
   - **When:** it is one clause, and it can ride the ADR-0021 edit that advisory 1 needs, if O chooses.
3. **The real client's `session-held` words are pinned by code alone.**
   - **What is pinned:** the builder (`driven_test.rs`), and, through the verbs, `FakeHub` (`first.rs`).
   - **The gap:** through the real client, `driven_e2e_test.rs:599-605` asserts only `session-held`. `HubClient::held`
     could stop calling `session_held` with no test failing.
   - **The fix:** one assertion there would pin it. The text should contain `"kiwi/oc" is held (default hold` and
     `holler release kiwi/oc`. This is test-only and cheap, and not required.
4. **A grammar split that predates this story.**
   - **The two rules:**
     - the hub mints labels by ADR 0005's regex `^[a-z0-9][a-z0-9-]{0,31}$` (`holler-hub/src/token.rs:414`), which
       admits a trailing `-`;
     - `RoutableName`'s segment check (`vocab.rs` `check_segment`) refuses one.
   - **The effect:** for a body whose token label ends in `-`, `session-held` falls back to the id and spells no
     command.
   - **No new harm:** such names are already unroutable by name, because `say` refuses them by the same grammar.
   - **Follow-up:** align the two grammars, outside this story.
5. **`docs/pane-control.md:290` overstates for the fallbacks.**
   - **What it says:** `session-held` "names the `holler release SESSION` that lifts it".
   - **Two fallbacks name none:**
     - a failed roster read (round 1);
     - a name outside the grammar (this round). It is reachable only through a crafted advertisement or advisory 4's
       label.
   - **Weight:** non-blocking. F, T-green and A-dup flagged it too.
6. **A third instance of round 1's advisory 7 (hub text passed through raw), on a non-race path.**
   - **Where:** the `--pane` verbs' `pane_not_driven` (`say_cmd.rs:316`) appends the hub's text.
   - **What:** for an ambiguous id, that text lists the candidate names unescaped (`ambiguous session …: candidates are
     …`).
   - **Why it is not rework:** D5 sanctions "the hub's text", and `say SESSION` printed the same text on `main`.
   - **Follow-up:** one fix covers all three instances: escape the passed-through text, with `quoted` or an
     `embedded`-style escape.

**Carried from round 1** (`git show fde8eb8:docs/handoffs/839/handoff-S.md`; the number is round 1's)

- **2, P8 row (4):** the operator's to revisit.
- **3, A-dup's warns 1 to 6:** follow-ups. Warn 2 is now half fixed: the `session-held` copy is gone, while `FakeHub::why`
  and the pane gate's sentence remain.
- **4, a slow failure:** a gate regression makes `driven_e2e_test.rs` slow to fail, because of the 600 s `Send` budget in
  the conformance suite.
- **5, the spent grant:** the grant spent before the pane gate refuses is `main`'s order.
- **6, the rebind reason:** the body's rebind reason is broader than D2 names.
- **8, `admin/query`:** Acceptance 13's `admin/query` form is not exercised.
- **9, Acceptance 22's doctor case:** it could also pin that no `hub.advertised` call is made.
- **10, a flake:** the `profile_rename_faults_test` flake (#665) predates this story.
