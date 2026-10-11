# Handoff-A: #644 part 2 - the OpenCode agent through `launch` and `relaunch`  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-644-implementation` (base `origin/main` at `263a2d7`; the branch's one commit, `3d4e633`, touches only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/644-brief.md` (outside brief gate round 1: PASS, `docs/handoffs/644-brief-result-r1.md`)   **Reuse map:** the brief's own Evidence, E1 to E26 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK: 1 `block` finding and 8 `warn` findings. The plan is built on code this branch does not have. The brief says the story is "Built after **#802** and **#642 part 3** have merged". Neither has merged:

- **#802** has no PR. Its code exists only on the local branch `issue-802-implementation` at `3fe111d`. That branch has not been pushed to `origin`, and it is based on `92fbc6e`, which is older than this worktree's base.
- **#642 part 3** has no commits yet.

**What is missing on this base.** None of #802's seams exist here: `ServeSpec`, `serve_command`, `verify_serving`, `AGENT_UNSUPPORTED`, `HarnessOp::VerifyServing`, `FakeHarness::serve_specs`, `AGENT_MISMATCH`, `tx_launch/server.rs` and `tests/pane_verbs/launch/serve.rs` are all absent. The harness port still has the one-step `serve(name, port)`.

**Why it blocks.** Four parts of the plan can neither be written as tests that compile nor made to pass inside the Blast radius:

- Scope 3;
- Acceptance 4;
- the `ServeSpec` half of Acceptance 2;
- the `verify_serving` half of Acceptance 5.

The only way F could pass them is to rebuild #802's port, engine, fake and rig changes. Every one of those is marked "Not in this story", so the result would be a parallel copy of an unreviewed branch. The outside gate saw the mismatch (its NIT-4 and W-5) and still passed the brief.

**What holds on the merits.** The rest of the plan extends the right objects:

- the one comparison table (`SpecField`/`FieldValue`);
- the one `Report` that both verbs share;
- the harness port's own refusal, handled by the engine's own rollback.

The warns are wording and coordination fixes. They are cheap to apply in the same amendment.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | Dependency note, Depends on, Scope 3, Acceptance 1 (placement), 2, 4, 5; Evidence E11-E18, E24-E26 | dependency direction / pattern consistency | The plan extends #802's seams, which are not on this base (`263a2d7` = `origin/main`). #802 has no PR and has not been pushed; #642 part 3 has no commits. Acceptance 4 and the `verify_serving` halves of Acceptance 2 and 5 cannot be met without editing `ports.rs`, `tx_launch.rs` (798 lines), the test kit's fake and the launch rig, which are outside the Blast radius. | Hold the run until #802 is merged, then start a fresh run off the new `origin/main` with the evidence re-cut from merged code. Or split the issue (see Notes for O). |
| 2 | warn | Acceptance 5 ("leaves P unchanged"), Acceptance 2 ("recorded a `ServeSpec` ... for both `serve_command` and `verify_serving`") | ADR / contract | Two clauses cannot pass as written. (a) ADR-0021 "Decisions taken" 1: a failed act restores P's specs by a second write, so P's generation moves by two. `a_failed_act_restores_the_profile_specs` pins the log `[1, 2, 3]`. (b) #802's fake records the spec of `serve_command` only; `verify_serving` records nothing (`3fe111d`, `harness.rs:332-344`). | (a) Say "P's specs equal what they were; the generation moved by two and the log shows the edit and its reversal". (b) Say "`serve_specs()` holds the spec with KEY, the server runs KEY, and `verify_serving` answered `Ok` (the fake refuses `agent-mismatch` otherwise)". |
| 3 | warn | Acceptance 8 ("ADR-0021's launch/relaunch row") | ADR | The plan names one ADR place, but this story makes five places stale (details). Under the stack rule, an ADR that a change extends is updated in the same change. | List the five places in Acceptance 8. |
| 4 | warn | Blast radius, the help text | docs / contract consistency | The help line of `--agent` (`args.rs:60-61`, "Without it, the server's default agent") contradicts operator decision 2. The verbs' `--help`, taken from their `Args` doc comments (`launch.rs:54-55`, `relaunch.rs:35`), lists the data keys without `agent`. `args.rs` is outside the Blast radius, and the radius limits `launch.rs` to "the `Report` struct and its text only". | Name the three doc comments in the Blast radius. |
| 5 | warn | Scope 2, the text form of `FieldValue::Key` | contract shape | `Key(None)` prints `none`, but `none` is a valid `AgentKey` (ASCII letters). The text of `profile show` therefore cannot tell an agent named `none` from no agent; the JSON can. `Argv(None)` prints `none` safely only because a set argv prints as a JSON array. | Decide in the brief: accept it and say so in `Display`'s doc, or print a set key in a form that cannot read `none`. |
| 6 | warn | Acceptance 2 and 3, the cases of the new module | anti-duplication (tests) | `launch_with_agent_stores_the_key_in_the_record` (`tests/pane_verbs/launch.rs:122-142`) and E26's keep/replace pair (`relaunch.rs:570-615`) already pin the record half. "E26 stays as it is" pushes `agent.rs` to near-copies of them with one more assertion each. | Extend those tests with the new assertions (`data.agent`, one CAS write), or keep the new module to what is new. |
| 7 | warn | Acceptance 7 and 8, the lists to update; "`apply` sees it" | scope accuracy | Three clauses find nothing to change on this base. (a) The only literal 15-field list is `PATHS` in `profile_diff_test.rs`: `profile_verbs/show.rs` asserts single lines with `contains`, and `fake_profile_store_test.rs` holds no field list. (b) No README or `docs/` text shows the launch result line (`grep "harness port"`). (c) `profile apply` is a stub (`profile/apply.rs`, #664 is in its brief stage). | Say "none found on this base" for (a) and (b), so S does not audit edits that cannot exist. Move "apply sees it" to #664's acceptance. |
| 8 | warn | Operator decision 4, and the stories beside it | coordination | Two in-flight texts disagree with decision 4. #664's brief, item 13 (branch `issue-664-implementation`), says `ALL` omits `opencode_agent` and asks for a follow-up issue. #700's journal says "#647 owns `SpecField`" (#647's brief does not use `profile_diff.rs`). Once this lands, #664's row c/d split changes: a healthy pane whose agent differs is `reported`, not `adopted`. | Record the ownership in #644's issue and in #664's brief (strike item 13). No code change here. |
| 9 | warn | Acceptance 1, 4 and 5: where the test code goes | file structure / size | On #802's branch, `launch/rig.rs` is 793 lines (739 here) and `tests/pane_verbs/launch.rs` is 687. A helper added to the rig takes it past about 800. | Keep every new helper in `launch/agent.rs`. The only edit to `launch.rs` is the `mod agent;` line. |

### Details

**1. The dependency is not met (block).**

- **Base.** After a `git fetch origin`, `git rev-parse origin/main` and `git merge-base HEAD origin/main` are both `263a2d7`. `origin/main`'s newest commit is #816 (#808).
- **#802.**
  - `gh pr list --head issue-802-implementation --state all` returns `[]`.
  - `git ls-remote origin 'refs/heads/issue-802*'` returns nothing.
  - The branch exists only in the local worktree `.claude/worktrees/0802-serve-command`, at `3fe111d`, on top of `92fbc6e`.
  - It overlaps 9 files that `main` has changed since then: `ports.rs`, `tx_launch.rs`, `ADR-0021.md`, `CHANGELOG.md`, the launch tests (`launch.rs`, `launch/rig.rs`, `launch/guards.rs`), `relaunch.rs` and `doctor.rs`. So it needs a rebase before it can merge.
- **#642.** The issue is open. Its worktree `0642c-opencode-agent` sits at `263a2d7` with no commits. #802's own title calls itself "step 1 of #642 part 3".
- **This base.**
  - `grep -rl 'verify_serving\|ServeSpec' crates/` matches 0 files.
  - The port is `fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError>` (`ports.rs:186`), and the fake's op is `HarnessOp::Serve` (`holler-pane-testkit/src/harness.rs:34`).
  - The rig has none of `with_agent`, `after_server_start`, `snapshot_into` or `serve_specs`.
  - `tests/pane_verbs/launch.rs:10-15` declares `rig`, `displaced`, `guards`, `profiles` and `record_bytes`, with no `mod serve;`. Acceptance 1's placement names a module that does not exist.
- **Where the evidence was cut.** E11-E18 and E24-E25 quote `3fe111d`, as the brief says. E26's line numbers are also #802's: here the tests are at `relaunch.rs:548-615`.
- **What F would have to build.** To pass Acceptance 4, F would build `ServeSpec`, `verify_serving`, the act's verify step, the fake's recorder and `AGENT_MISMATCH`, and the rig's hooks. All of these are in files the brief puts out of scope ("Not in this story: `HarnessPort` and `HostPort` (#802) ... `tx_launch.rs`"), and `tx_launch.rs` is at 798 lines. That is the parallel path this gate exists to stop.
- **Guaranteed conflicts.** #802 also rewrites the ADR-0021 text this story edits:
  - section 8, "Launch and relaunch as built (#644)";
  - section 9's `pane launch`, `pane relaunch` row and the row of open codes;
  - section 2, item 7.

  It also changes 5 of this story's named Blast-radius files: `tests/pane_verbs/launch.rs`, `launch/profiles.rs`, `relaunch.rs`, `ADR-0021.md` and `CHANGELOG.md`. Running both at once breaks `parallel-agent-coexistence.md` "Merge / coordination hygiene" (disjoint ownership, a designated merge order).

**2. Two acceptance clauses that cannot pass as written.**

- **(a) Acceptance 5.** The I8 compensation is a ratified decision (ADR-0021 "Decisions taken" 1, `ADR-0021.md:756-759`). The amended acceptance of #663 says "specs equal", not "generation equal". The existing test `launch/profiles.rs:61-86` asserts `profile.generation == 3` and the log `[1, 2, 3]` after a failed act.
- **(b) Acceptance 2.** #802's fake has a recorder only for `serve_command` (`serve_specs`, `harness.rs:175` and `:328` at `3fe111d`). `verify_serving` compares `spec.agent` with the running server's agent and records nothing (`:332-344`). The test kit is not in the Blast radius, so the clause must name what the fake can show.

**3. The ADR-0021 places this story makes stale.**

- **(a)** Section 8, "Launch and relaunch as built (#644)" (`ADR-0021.md:417-444`): the agent is recorded and reported, and a `verify_serving` refusal rolls back and records nothing. #802 rewrites this paragraph, so edit #802's merged text, not today's.
- **(b)** Section 9, the `pane launch`, `pane relaunch` row (`:522`): the open codes this story's tests pin as refusals. List `agent-unusable` as the code #642 part 3 will answer, unless #642 part 3 has merged by then. The row also lacks #700's own `agent-key-invalid`. `agent-mismatch` belongs to the fake and does not belong in the ADR.
- **(c)** "Deferred to named stories", `:749-750`. That bullet says #644's last part refuses "a key the project does not define". This story completes it, and operator decision 1 fixes the mechanism: the server's `verify_serving` answers, and the CLI reads no project file. #700's A-dup and S handoffs both flagged this bullet for "#644 amends it when it lands".
- **(d)** "Decisions taken", `:752-778`. Items 8-10 set the precedent for dated operator decisions. Operator decisions 1-5 of 2026-10-11 belong there.
- **(e)** Section 3, `:194-200`. It describes what `--from-current` copies and what `profile_diff` compares. Add `opencode_agent` to what is compared.

**4. Help text.** No `PaneCmd` variant has a doc comment (`pane/mod.rs:36-49`), so clap uses each `Args` struct's doc comment as the verb's `--help`.

- `launch.rs:54-55` and `relaunch.rs:35` print `{"verb", "pane", "profile", "spec_only"}`. They must gain `agent`.
- The `--agent` line belongs to `SpecFlags` (`args.rs:60-61`), which both verbs share. On a relaunch, and on a launch whose profile spec names an agent, leaving out `--agent` keeps the spec's agent (E5, operator decision 2), not the server's default.
- One suggested wording: "Without it, a new spec has none (the server's default agent) and an existing spec keeps its own."

**5. `none`.** `AgentKey::parse` accepts `none` (`argv.rs`, E3).

- The JSON form is unambiguous (`"none"` against `null`).
- Only the text of `profile show` and of its difference lines is ambiguous, and those are what #664's report and #665's finding reuse.

This is the operator's call, not F's, so the brief should make it.

**6. Near-copies.** The test rig is shared, and the stack rule rejects near-copies of the harness helpers at Phase 7.

- The brief freezes E26 ("stay as they are"), so the relaunch keep/replace cases would be written a second time just to add `data.agent`.
- Adding one assertion to an existing test does not weaken its #700 pin.

**7. Clauses that find nothing.** These were grepped on this base:

- `grep -rn "harness.port_policy\|model.effort"` over `crates/holler-cli/tests/profile_verbs/` and `crates/holler-pane-testkit/tests/` finds no field list.
- `grep -rn "harness port" README.md docs/` finds nothing.
- `profile apply` returns `not_implemented(664)` (`crates/holler-cli/src/profile/apply.rs:21-23`).

**8. The stories beside this one.**

- **#664** (in its brief stage): row c "matches its spec (`diff_spec` empty) -> adopt" and row d "differs -> reported". Its item 13 says "`SpecField::ALL` omits `opencode_agent` ... File a follow-up issue". Both shift when this story lands, which is the intended result.
- **#700's journal** (`docs/handoffs/0700-implementation/decisions.md:190`, `:229`) assigns `SpecField` to #647.
- **#647's brief** (branch `issue-647-implementation`) does not use `profile_diff.rs`. No branch adds the variant today, so this is coordination, not duplication.

**9. Sizes.** `scripts/lint.sh` fails at 900 lines; the stack rule flags about 800. Every file this story's source edits touch is small: `launch.rs` 442, `relaunch.rs` 88, `profile_diff.rs` 333, `profile/show.rs` 198. The pressure is in the launch test rig once #802 lands.

### What holds on the merits (no amendment needed)

- **One `Report` for both verbs.** `Report` (`launch.rs:344-412`) is the one data and text form of both verbs: `relaunch.rs:19,57` reaches it through `emit_outcome`. One change covers both.
  - Derive `agent` from `launched.pane` in `Report::new`, as `spec_only` is, and never from the request's spec. Then `data.agent` cannot disagree with `data.pane.opencode_agent`.
  - The text needs no escape: `AgentKey`'s guard also runs on decode.
- **The additive key fits the contract.** ADR-0021 `:592-593` says adding a key to a verb's `data` does not change `schema_version`, and ADR-0003's Versioning counts an addition as MINOR. The name `agent` follows the cadence of the `data` keys, which are named after their flags (`profile`, `spec_only`).
- **The 16th field.**
  - `SpecField::OpencodeAgent` after `Effort` follows `ProfileSpec`'s field order (`opencode_agent` after `model`, ADR-0021 `:48`), which is the order `ALL` documents.
  - `FieldValue` derives `Serialize` with `#[serde(untagged)]` (`profile_diff.rs:158-159`), so `Key(Option<String>)` serializes as a string or `null`. This settles the outside gate's NV-3.
  - The only exhaustive matches are all in `profile_diff.rs`: `Display` over `FieldValue`, and `as_str`/`value` over `SpecField`.
  - Neither type derives `Deserialize`, and no hub, protocol or persisted form reads either one. So the field touches no wire or file format.
  - `differs` needs no new arm, and `Key`'s `Display` must reuse `write_escaped`, not a second escaper.
  - `profile show` lists fields from `SpecField::ALL` (`profile/show.rs:135-142`), so it needs no change.
- **Scope 3's placement.** The refusal comes from the harness port and the rollback from the engine. The CLI adds no check and reads no file: `launch_request` and `relaunch_request` read only `ProfileStore::get` and `PaneStore::get` (`launch.rs:92-105`, `relaunch.rs:64-88`). This settles NV-10. It keeps one source of truth and does not enforce at a call site.
- **Test placement.** A sibling module in `tests/pane_verbs/launch/` for both verbs' agent cases follows #802's `serve.rs`, which hosts relaunch cases too (E24). This answers the outside gate's W-2.

## Notes for O

**The fix is sequencing, not wording.** A brief amended on this base still cannot be built. The fresh run needs a base that contains #802. Choose one path; the brief's own "Depends on" picks path A.

**Path A (recommended).**

1. Push `issue-802-implementation` and rebase it onto current `main`; it overlaps the 9 files listed in detail 1. Take it through its own gates and merge it.
2. Then start a fresh #644 run off the new `origin/main`, in a new worktree. Do not use `resumeFromRunId`.
3. Before that run:
   - Re-cut E11-E18 and E24-E26 from the merged `main`, not from `3fe111d`.
   - Re-check every contract name the acceptance relies on: `ServeSpec`, `serve_command`, `verify_serving`, `AGENT_UNSUPPORTED`, `AGENT_MISMATCH`, `HarnessOp::VerifyServing`, `FakeHarness::serve_specs`, and the rig's `with_agent`, `after_server_start` and `fail_next`.
   - Confirm that `mod serve;` exists, for Acceptance 1's placement.
4. Decide on #642 part 3, which the brief also lists as a dependency. The tests do not need it: the fake stands in, and to the fake `agent-unusable` is only an open-code string. If the run goes before #642 part 3, the ADR and the CHANGELOG must call `agent-unusable` the code #642 part 3 will answer, not a merged code.

**Path B (the operator's call, if part of the story should land now).** Split the issue.

- **2a, on today's `main`:**
  - Scope 1 and 2.
  - Acceptance 1, 3, 6 and 7.
  - Acceptance 2, record and one CAS write only.
  - Acceptance 5, the profile write only.
  - Acceptance 8 without any `verify_serving` text.
- **2b, after #802:**
  - Scope 3.
  - Acceptance 4.
  - Acceptance 2's `ServeSpec` clause and Acceptance 5's `verify_serving` clause.
  - The ADR's refusal text.

Designate the merge order with #802 for the files both touch: ADR-0021 sections 8 and 9, the module list of `tests/pane_verbs/launch.rs`, and `CHANGELOG.md`.

**Either way, fold warns 2-9 into the same amendment** so the re-review is one round:

- Acceptance 5: "specs equal, generation moved by two".
- Acceptance 2: name what the fake can show.
- Acceptance 8: the five ADR places of detail 3.
- Blast radius: add `args.rs:60-61`, `launch.rs:54-55` and `relaunch.rs:35`.
- The text form of `none`.
- Extend the #700/E26 tests instead of copying them.
- Mark the empty list, doc and `apply` clauses.
- The #664 and #647 ownership note.
- Keep helpers in `agent.rs`.

**Where to amend.** The GitHub issue is the source of truth, and this brief is a mechanical copy of it. Amend #644's body, then copy it again.

## Patterns referenced

1. `crates/holler-pane/src/ports.rs:186` and `crates/holler-pane-testkit/src/harness.rs:34`: the one-step `serve` and `HarnessOp::Serve` this base has. Compare `git show 3fe111d:crates/holler-pane/src/ports.rs` and `3fe111d:crates/holler-pane-testkit/src/harness.rs:321-344`: #802's `serve_command`, `verify_serving` and the fake's recorders.
2. `crates/holler-pane/src/profile_diff.rs:31-193`: the one field table, `FieldValue`, its untagged `Serialize` and its `Display`.
3. `crates/holler-cli/src/pane/launch.rs:319-442` and `relaunch.rs:19,35,57`: `Verb`, `Report` and `emit_outcome`, which relaunch shares, and the verbs' `--help` docs.
4. `docs/adr/ADR-0021.md:417-444`, `:518-534`, `:584-593`, `:733-778`, and `docs/adr/ADR-0003.md` "Versioning": the launch paragraph, the codes by verb, the `schema_version` rule, and the deferred and decided items.
5. `crates/holler-cli/tests/pane_verbs/launch/profiles.rs:61-86` and `.aftersight/releases/claude/workflow/parallel-agent-coexistence.md:96-103`: the I8 compensation as tested, and the rules for coordinating merges.
