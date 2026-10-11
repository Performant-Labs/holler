# Handoff-S: #644 part 2 - the OpenCode agent through `launch` and `relaunch`  (spec audit)

**Date:** 2026-10-11
**Branch:** `issue-644-implementation` at `1e9e31b` (merge base `origin/main` `7284001`, #802 merged; `origin/main` has not moved since)
**Issue:** [#644](https://github.com/Performant-Labs/holler/issues/644). The brief `docs/handoffs/644-brief.md` matches the live issue body line for line, so the two are one text.
**Handoffs reviewed:** `handoff-A.md` (Phase 3), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `decisions.md`, `evidence.md`, and the outside model's diff gate (`docs/handoffs/644-diff-result-r1.md`, git-ignored, round 1: PASS)
**Diff audited:** `git diff 7284001..HEAD`: 4 production files, 6 test files, ADR-0021, ADR-0003, `CHANGELOG.md`
**Verdict:** PASS

## A precondition

Met. `handoff-A.md` (Phase 3, on merged #802) is **PASS** with 0 block and 6 warn. The anti-duplication gate `handoff-A-dup.md` is also **PASS**, with 0 block and 2 warn.

## T precondition

Met.

- **RED** (`handoff-T-red.md`): 13 tests failed on assertions, not on compile, setup or timeout errors.
  - `profile_diff_test`: 3 failed.
  - `pane_verbs`: 9 failed.
  - `profile_verbs`: 1 failed.
  - Two refusal-table tests passed before F, and T said why: Scope 3 says the refusal and its rollback are #802's engine and need no code change, so these are regression pins.
- **GREEN** (`handoff-T-green.md`): all of those pass, with 0 blocking issues.
  - Totals: `pane_verbs` 275/0, `profile_verbs` 45/0, `profile_diff_test` 11/0.
  - The configured `test.unit.command`, clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test` and `wire_selftest` all pass.
  - Three removal mutations (M1 to M3) each made the named tests fail, so the tests are not tautological.

## Acceptance criteria

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | **Output, text and JSON.** `launch --agent KEY` prints `..., harness port N, agent KEY` and `data.agent == "KEY"`. Without `--agent` it prints the golden line with `data.agent == null`. `--spec-only` gives `data.agent == null`. The JSON passes the envelope checker with every earlier key in order. | `tests/pane_verbs/launch/agent.rs`:<br>- `the_text_line_names_the_agent_and_is_unchanged_without_one` (:61) checks the whole line by equality, with and without the agent. The golden half equals the line T-red recorded as today's output, and it contains no `agent`.<br>- `a_key_that_reads_as_the_empty_value_is_quoted` (:86) pins A's warn 2: `--agent=-` prints `agent "-"`.<br>- `data_carries_the_recorded_agent_after_spec_only` (:103) checks `data.agent` is the key or `null` (present, not missing) and equals `data.pane.opencode_agent`. It checks key order on the raw stdout: `"data":{"verb":"launch","pane":{` ... `"profile":null,"spec_only":false,"agent":X}`. `data_of` runs `check_envelope` (`launch/rig.rs:616-621`).<br>- `spec_only_reports_no_agent` (:131): the request names an agent and P's spec holds it, yet `data.agent` is `null`. So the value is read from the record, not the request. | Met |
| 2 | **Launch records the agent.** `data.agent == KEY`; one `cas_put`; one generation bump. The fake's `serve_specs()`, the server's agent, `VerifyServing` in the log, and success. | - `tests/pane_verbs/launch.rs::launch_with_agent_stores_the_key_in_the_record` (:162), extended: `data["agent"]`, one `PaneStoreOp::CasPut`, record generation 1.<br>- The "in the new module" clause is proven by #802's `launch/serve.rs::the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies` (:170). It asserts `serve_specs() == [with_agent(AGENT)]`, `server.agent == Some(AGENT)`, `VerifyServing` in the steps and a successful run.<br>- `relaunch_passes_the_in_flight_agent_to_serve_command` (:246) covers the relaunch side.<br>- The brief says in the same sentence that "this story does not repeat it", and its own rule calls a near-copy a rejection, so T did not copy it (A's warn 5). | Met |
| 3 | **Relaunch with and without `--agent`.** The two existing tests gain `data.agent`, and nothing is weakened. | `tests/pane_verbs/relaunch.rs::relaunch_without_agent_keeps_the_stored_key` (:588) asserts `data.agent == "orchestrator"`. `relaunch_with_agent_replaces_the_stored_key` (:611) asserts `"feature-implementor"`. The diff only adds lines and binds `let data =`; no assertion was removed. | Met |
| 4 | **An undefined key is refused, nothing is recorded, nothing runs.** On a launch and a relaunch, with a stand-in open code and with the fake's `agent-mismatch`: exit 3 and the code, the registry unchanged, processes stopped by the rollback, no session of record, the reconcile step once. | `agent.rs` has two tables over one helper, `assert_refused` (:217).<br>- `a_launch_the_server_refuses_records_nothing` (:252) covers a stand-in `agent-unusable` via `fail_next(VerifyServing)`, and `agent-mismatch` from a server that a hook restarted with another agent.<br>- `a_relaunch_the_server_refuses_leaves_the_record_unchanged` (:289) covers both codes.<br>- Each row checks exit 3 and the code. The message must equal the harness's message, then `"; "`, then the step, with the step exactly once. `VerifyServing` was called, with no `CreateSession` and no `CasPut`. `StopOwned` runs once on a launch, and twice on a relaunch (B1, then the rollback). The server ends `Killed`.<br>- On a launch there is no record and the profile log is `[1]`. On a relaunch the record equals the seeded one: same generation, same agent.<br>- #802's `serve.rs::a_verify_refusal_rolls_back_and_stops_the_server` (:419) is the `agent-unsupported` launch row. | Met (regression pins; see Spec compliance on "message unchanged") |
| 5 | **`--agent` with `--profile P`.** Success: the key is in P's spec and the log is `[1, 2]`. A failure at `verify_serving`: P's specs are equal and the log is `[1, 2, 3]`. | - Success: `launch/profiles.rs::launch_with_profile_adds_the_spec_and_bumps_once` (:43), extended. P's spec equals `launch_spec()` plus the key, generation 2, log `[1, 2]`, and `data.agent` is the key.<br>- Failure: the `--profile` row of `a_launch_the_server_refuses_records_nothing` (`agent.rs:256`, `:264-267`). P's specs are empty, as seeded, the log is `[1, 2, 3]`, and the step names P.<br>- The failure half is in `agent.rs`, not `profiles.rs`, as T-red recorded under A's warn 5. | Met |
| 6 | **The field table.** `PATHS` has 16 entries, with `opencode_agent` after `model.effort`. Values serialize like the spec, with and without an agent. `diff_spec` reports KEY vs none and KEY vs another key, and never equal keys. `Key(None)` prints `(default)` and a key named `none` prints `none`. | `crates/holler-pane/tests/profile_diff_test.rs`:<br>- `PATHS` (:214).<br>- `spec_field_values_serialize_like_the_spec` (:234), over `spec()`, `bare` and an agent-carrying spec.<br>- `diff_reports_an_agent_change` (:359): all three cases, plus the JSON `{"field":"opencode_agent","spec":KEY,"live":null}`.<br>- `an_absent_agent_prints_default_and_a_key_named_none_prints_none` (:385). The test reaches `FieldValue::Key(None)` as `value()` of a spec with no agent (`profile_diff.rs:111-113`), and asserts that `(default)` is not a valid key. | Met |
| 7 | **`profile show` sees it.** `opencode_agent: KEY` and `opencode_agent: (default)`, and a live pane whose agent differs is reported as that field's difference. There is no other list to update. | - `crates/holler-cli/tests/profile_verbs/show.rs::show_lists_the_agent_and_reports_an_agent_difference` (:222), in both formats. It checks the two spec lines, the difference line `opencode_agent: spec feature-implementor, live (default)`, the JSON difference, and that the pane with no agent `matches`.<br>- No other list: `git grep` finds no other spec-field list in `README.md` or `docs/` (outside `docs/handoffs/`) and no other `"spec_only"}` key list. The only production consumer of the table is `profile/show.rs`. | Met |
| 8 | **Docs.** ADR-0021 in five places, and a `CHANGELOG.md` `## [Unreleased]` entry linking #644. | ADR-0021:<br>- (a) section 8, `:525-534`: the agent's path, the refusal (code, message plus step), the rollback, I8 with `--profile`, the relaunch case, and the output.<br>- (b) the section 9 row, `:623`: `agent-key-invalid` as open (#700), never echoed. The `verify_serving` code is described as one the harness owns. `agent-mismatch` is not listed and `agent-unusable` is left to #642 part 3.<br>- (c) "Deferred", `:854-856`: now "decided, #644", and the CLI reads no project file.<br>- (d) items 15 to 20, `:898-911`.<br>- (e) section 3, `:276-279`: `opencode_agent` is compared. I checked the claim that every field except `pane` is compared: `ProfileSpec` has 16 leaf fields besides `pane`, all in `ALL`.<br>`CHANGELOG.md:419` is under `[Unreleased]` / Enhancements and links #644. The claim that the real verbs answer `not-implemented` until #649 matches `pane/wiring.rs`. | Met |
| Scope 4 | **Help text.** The new `--agent` line, and both verbs' data-key lists name `agent`. | `agent.rs::the_help_of_both_verbs_names_the_agent` (:153), on both verbs' folded `--help`: the new line is present, the old line is absent, and the key list includes `agent`. Source: `args.rs:60-61`, `launch.rs:54-56`, `relaunch.rs:35-36`. | Met |

## Spec compliance

The operator decisions, each as implemented:

1. **The refusal is `verify_serving`'s, and the CLI reads no file.** `launch_request` and `relaunch_request` are untouched, and no new check runs at a call site. The refusal and its rollback are the engine's (`tx_launch.rs:600-620`, `tx_launch/server.rs`), pinned by the tables of criterion 4. Implemented as stated.
2. **`relaunch` without `--agent` keeps the recorded agent.** The overlay (`launch.rs` `effective_spec`) is unchanged, and its test is extended (criterion 3). Implemented.
3. **No clear flag.** None was added. Implemented.
4. **`opencode_agent` joins `SpecField::ALL`.** Variant `OpencodeAgent` (`profile_diff.rs:43`), between `Effort` and `Role` in `ALL` (`:57`, `:66`), path `opencode_agent` (`:88`). `value()` gives `FieldValue::Key(Option<String>)` (`:111-113`, `:180`). It serializes as the string or `null` through the enum's `untagged`, and `differs` has no new arm. Implemented as stated.
5. **`agent` in the result.** `Report.agent: Option<AgentKey>` sits last, after `spec_only` (`launch.rs:357`). It is read from `launched.pane` (`:373-377`), so it cannot disagree with `data.pane.opencode_agent`. The text clause comes after the harness port and the `(pid P)` and before `; profile` (`:399`, `:407-409`). Implemented as stated.
6. **`(default)` for an absent agent.** `Key(None)` prints `(default)` (`profile_diff.rs:204`). `Key(Some)` shares `Text`'s single `write_escaped` arm (`:198`), so there is no second escaper. Implemented as stated.

**Where the code follows A's recorded findings rather than the brief's literal wording.** None of these is silent. Each is recorded in `handoff-A.md` (warns 1 to 5), `handoff-T-red.md`, F's "Deviations" and `handoff-A-dup.md` (warn 2).

- **"Code and message unchanged"** (Scope 3, Acceptance 4). Once the act has begun, the engine (#802, unchanged here) appends the reconcile step once. The brief contradicts itself on this, because Acceptance 4 also says "the reconcile step is printed once". The tests and ADR-0021:528-529 state what the code does: the code unchanged, then the harness's message followed once by the step. No production code was bent either way. This is a wording defect in the brief, not a deviation in the diff.
- **The `-` key prints quoted.** The key goes through `list::text_value`, the line's existing rule for stored strings (`list.rs:280-291`), which `Report::text`'s own doc already names. Every key that passes the guard prints as typed except `-`, which is the line's empty value.
- **Test placement.** Acceptance 2's new-module clause is not duplicated, and Acceptance 5's failure half is a row in `agent.rs`. The brief's own anti-duplication rule asks for this, and the behavior is proven (table above).

Because the code is right and the brief text is only stale, this is not an `ADVISORY-HOLD` (see Advisory note 1).

## Quality audit

- **Correctness and failure handling.**
  - `data.agent` and the text clause share one source, the stored record. A `--spec-only` run reports `null`.
  - No failure path is new. A `verify_serving` refusal goes through #802's `roll_back`.
  - On a relaunch the refusal comes after B1, so the pane is left with no server and its record unchanged. ADR-0021 and the CHANGELOG say so, and the relaunch table pins it.
  - No new persistence, concurrency or lost-write surface.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!`, `#[allow]` or `unsafe` in any added `src` line.
  - Files: `launch.rs` 457, `relaunch.rs` 89, `args.rs` 170, `profile_diff.rs` 347. Tests: `launch.rs` 748, `launch/agent.rs` 299, `relaunch.rs` 633, `launch/profiles.rs` 212, `profile_verbs/show.rs` 365, `profile_diff_test.rs` 400. All are under 900.
  - `launch/rig.rs` (814), `tx_launch*`, the test kit, `holler-proto` and `holler-body` are untouched.
  - No dead code was added. The unreachable `(pid P)` clause predates this story, and the brief keeps it (Advisory note 5).
- **Public API.**
  - `SpecField` and `FieldValue` each gain a variant. Neither is `#[non_exhaustive]`, but the only production consumer outside `profile_diff.rs` is `profile/show.rs`, which iterates `ALL` and builds `Text`.
  - Neither derives `Deserialize` (`profile_diff.rs:33`, `:164`), so no wire or file format changes.
- **Protocol and wire.** No protocol change, no golden file changed and nothing for `docs/protocol/v2.md`. The JSON key is additive, and ADR-0021 keeps `schema_version` at 1 for an added `data` field.
- **Tests.**
  - Every case runs on the test kit's fakes, which suits CLI verbs over ports.
  - No sleep is used for synchronization.
  - RED-first evidence is in T-red, and the mutation spot-checks are in T-green.
  - The `agent-mismatch` rows reach the fake's own code only because the hook really restarts the server with another agent. The rig's `LinkedHost::run` fires the hook, and its `stop_owned` kills the server whose `--pane` is the pane. That settles the outside model's NV-3. NV-1 and NV-2 are settled by `Rig::new`, which seeds `demo` at generation 1 with no specs.
- **Documentation.**
  - `CHANGELOG.md` has an `[Unreleased]` entry linking #644.
  - ADR-0021 is updated in the five places plus section 3's flag table and copy list.
  - ADR-0003's SPEC FLAGS bullet now lists `--agent KEY` (see the Scope check).
  - No README or `docs/` page shows the result line, a spec-field list or a data-key list.
- **Public-repository privacy.** I grepped every added line, the committed handoffs included, for personal hostnames, tailnet names, IPs, machine and account names, home paths, e-mail and key patterns. It is clean: the only hit, "M1", is T-green's mutation label. The agent keys in tests (`feature-implementor`, `orchestrator`) are names, not secrets.
- **Commit and PR hygiene.**
  - Every commit subject is Conventional (`chore(#644): ...`) and carries `Co-Authored-By: Claude <noreply@anthropic.com>`, but none has a session link. The merged #802 run's commits were the same.
  - The PR is not open yet, so its AI disclosure cannot be audited here (Advisory note 7).

## Scope check

- **Delivered exactly.** Brief items 1 to 4 are done. Every Scope 5 "no change" item is untouched: the flag, the overlay, the record write, the engine's `ServeSpec` plumbing, `profile show`'s listing and the escaping rule.
- **Outside the Blast radius, documented, and accepted.**
  - **`docs/adr/ADR-0003.md:93`.** One token, `--agent KEY`, in the SPEC FLAGS bullet of the two verbs this story owns (`docs_rows.rs` assigns them to #644).
    - A's Phase 3 warn 1 asked for it, F listed it as Deviation 1, and A-dup cleared it as planned, not a drive-by.
    - It fixes a stale list of the CLI surface, which this audit's own documentation check requires.
    - The outside model's NV-5 raised the same point.
  - **ADR-0021 section 3's table row and `--from-current` copy list.** These are in a Blast-radius file, from A's warn 1.
  - **The module-index line in `tests/pane_verbs/launch.rs`'s doc.** This goes beyond "the only edit ... is the `mod agent;` line". A's Phase 3 note asked for it, so the index of sibling modules does not go stale.
- **Under-delivery.** None.

## Verdict

**PASS.** Every acceptance criterion has a test that asserts the behavior. The six operator decisions are implemented as stated. The quality bars hold: guards, file sizes, no format change, docs and privacy. Every deviation from the brief's literal text follows a recorded A finding, and none is silent. Ready for O.

## Advisory notes (non-blocking)

1. **Bring the issue up to date before merge (A-dup warn 2, still open).** The issue body, which is the source of truth, still has the pre-Phase-3 text. Fold in A's warns 1 to 5, a text-only edit:
   - ADR-0003 (line 93) in the Blast radius;
   - "the code unchanged; the harness's message followed once by the reconcile step" in Scope 3 and Acceptance 4;
   - `-` prints quoted;
   - a refused relaunch leaves the pane down with its record unchanged;
   - Acceptance 2's new-module clause is covered by #802's `serve.rs` tests;
   - Acceptance 5's failure half is a `--profile` row in `agent.rs`.
2. **Sibling issues still contradict this story (A's warn 6, still open).**
   - #664's body, item 13, still says "`SpecField::ALL` omits `opencode_agent` ... File a follow-up issue".
   - #642's body still gives #644 part 2 "the spec-level refusal of an undefined key".
   - Neither issue has a comment recording this story's decisions 4 and 1. Record them before those runs reach S, or their auditors may act on stale text.
3. **The recovery sentence has no test.** ADR-0021:531-532 and the CHANGELOG say that a plain `relaunch PANE` brings back a pane whose relaunch was refused.
   - Reading the engine supports it: B1's `stop_owned` and health check pass on a server that is already down (`tx_launch.rs:607-609`). The record's generation is unchanged, so the CAS of the next relaunch matches. The recorded agent is the one the server ran before.
   - No test runs a refused relaunch followed by a plain one. Size XS follow-up.
4. **One ADR parenthetical is imprecise.** ADR-0021:525-526 says "(so a relaunch without the flag keeps the recorded agent)". With `--profile P` the base spec is P's (`relaunch.rs`, `relaunch_request`), so a relaunch takes P's spec's agent. The help text F wrote is exact ("an existing spec keeps its own"). Suggested wording: "keeps the recorded agent, or with `--profile P` the agent of P's spec". It is a wording fix, and the result line now prints the agent in either case.
5. **The `(pid P)` clause in `Report::text` cannot print** (A's Phase 3 note 2, F's known issue). The engine writes `pid: None` in every record since #802, and `Report` reads the record this run stored. So the brief's "appears only for a record written before it" is also not so. The brief kept the clause. Removing it is O's or the operator's call.
6. **Test constants are copied (A-dup warn 1).**
   - `STEP_PHRASE` exists in three sibling modules and `AGENT` in two, beside the rig's `STEP` and `occurrences`.
   - There are three private one-line `AgentKey` builders in `holler-pane` tests.
   - Size XS follow-up: move them to `launch/rig.rs` and `holler-pane/tests/common/mod.rs`.
7. **PR hygiene, after the script opens the PR.** Add the AI disclosure to the PR body per `CONTRIBUTING.md` (`gh pr edit`), as `CLAUDE.md` directs. Make sure the squash commit keeps the `Co-Authored-By` trailer. `CONTRIBUTING.md` also asks for a session link, which none of the pipeline's commits carry here or in #802.
8. **Merge order.** ADR-0021 and `CHANGELOG.md` are shared with #642 part 3, #649 and #664. Whichever run merges later merges `main` and keeps both texts. #642 part 3 names `agent-unusable` in section 9's row, beside this story's general wording.
