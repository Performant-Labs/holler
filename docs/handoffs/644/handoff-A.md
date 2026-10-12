# Handoff-A: #644 part 2 - the OpenCode agent through `launch` and `relaunch`  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-644-implementation` at `53ee140` (merge base with `origin/main`: `7284001`, #802 merged; the branch's own commits touch only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/644-brief.md` (revised after the plan check, evidence re-cut from `7284001`)   **Reuse map:** the brief's own Evidence, E1 to E37 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

Supersedes round 1 (`275052f`, BLOCK: the plan was built on #802 before #802 had merged).

## Summary

PASS: 0 `block` findings and 6 `warn` findings.

**Round 1's block is resolved.**

- The base is merged `main` (`7284001`).
- Every seam the plan names exists on it: `ServeSpec`, `serve_command`, `verify_serving`, `AGENT_UNSUPPORTED`, `AGENT_MISMATCH`, `HarnessOp::VerifyServing`, `FakeHarness::serve_specs`, `tx_launch/server.rs` and `tests/pane_verbs/launch/serve.rs`.
- All 38 path-cited Evidence blocks match the tree line for line (checked by script). E35's two greps give the same output when re-run.
- The round-1 warns 2 to 9 are folded in correctly.

**What the plan extends:**

- the one `Report` both verbs print, with the new key taken from the stored record;
- the one comparison table, `SpecField` and `FieldValue`;
- the harness port's own refusal, answered by the engine's own rollback. No check runs at a call site and the CLI reads no file.

**The warns** are two ADR gaps and an unclear text form for the key `-`. Three more are acceptance wordings that would mislead T or S, and the last is coordination with the runs going in parallel. Each one can be fixed in a few lines.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Acceptance 8(e); Blast radius (ADR files) | ADR | ADR-0021 section 3's table "Spec flags, one per `ProfileSpec` field" has no `--agent` row. The `--from-current` copy list below it leaves out `opencode_agent`. ADR-0003's SPEC FLAGS bullet (`:93`) lists every spec flag except `--agent KEY`. All three date from #700, and no test catches them. Acceptance 8(e) edits the sentence right after the table and leaves the table stale. | Widen 8(e): add the row `--agent KEY` to `opencode_agent` and add `opencode_agent` to the copy list. Add `docs/adr/ADR-0003.md` line 93 to the Blast radius for the one token. |
| 2 | warn | Scope 1 (text) and Scope 5 ("`Report::text`'s escaping" needs no change) | cross-cutting (terminal-safe text) | `Report::text`'s doc says "A stored string goes through `list::text_value`" (`launch.rs:377-378`). That function also quotes `-`, the empty value (`list.rs:35`, `:280-292`). `-` is a valid `AgentKey` (`argv.rs:155-156`), and the same line prints a missing session as `session -`. So `--agent=-` would print `agent -`, which reads as "no agent". | Print the key with `text_value(key.as_str())`. A key that passes the guard prints unchanged, except that `-` prints as `"-"`. Pin `--agent=-` in Acceptance 1's golden test. See details for the dead `(pid P)` clause in the same lines. |
| 3 | warn | Scope 3 and Acceptance 4 ("with that code and message unchanged") | contract accuracy | Once the act has begun, the message gains `; to reconcile, run holler pane doctor` (`failure_body`, `launch.rs:435-441`), and E25 asserts exactly that. The clause also contradicts Acceptance 4's own "the reconcile step is printed once". A test of message equality fails, and the tempting wrong fix is to stop appending the step. | Write: "the code unchanged; the message is the harness's message followed, once, by the reconcile step". |
| 4 | warn | Acceptance 8(a), the ADR text of a refusal on a relaunch | ADR / operator-visible behavior | On a relaunch, B1 stops the old server before the new one is asked (`act_live`, `tx_launch.rs:607-612`). `roll_back` stops only what the run made (`:726-742`). A `verify_serving` refusal therefore leaves the pane with no server, while its record still says `healthy`. That was already true of any failure after B1, but this story lets a mistyped `--agent` on a healthy pane cause it. "Rolls back and records nothing" describes only a launch. | Say in 8(a) that a refused relaunch leaves the pane down with its record unchanged. A plain `relaunch PANE` brings it back, because it keeps the recorded agent (decision 2). A check before B1 is a possible follow-up (see details). |
| 5 | warn | Acceptance 2 ("In the new module ..."), Acceptance 4 (launch with a stand-in code), Acceptance 5 | anti-duplication (tests) | Acceptance 2's new-module clause repeats what E24 already asserts, and the brief says so in the next sentence. Acceptance 4's launch case with a stand-in code is E25's body with a different string. Acceptance 5's two cases are two existing `profiles.rs` tests with `--agent` added. The brief's own rule calls such a near-copy an anti-duplication rejection. | Drop Acceptance 2's new-module clause. Write Acceptance 4 as one table in `agent.rs` (details). Extend `launch_with_profile_adds_the_spec_and_bumps_once` with `--agent`. Make Acceptance 5's failure half a `--profile` row of that table. |
| 6 | warn | Dependency note: the stories built beside this one | coordination | The note names only #642 part 3. #649 and #664 also edit ADR-0021 and `CHANGELOG.md`, and #664 edits ADR-0003 too. Two sibling briefs still contradict this one: #664's item 13 (`SpecField::ALL` omits `opencode_agent`, file a follow-up) and #642's "Out of scope" (#644 part 2 owns "the spec-level refusal of an undefined key"). | Apply the merge rule ("whichever merges later merges `main` and keeps both texts") to #649 and #664 too. Record decision 4 on #664's issue and decision 1 on #642's, so their S phases do not act on stale text. No code change. |

### Details

**1. The ADR places.**

- `docs/adr/ADR-0021.md:252-267` is the table "Spec flags, one per `ProfileSpec` field, used by `launch`, `relaunch` and the profile spec". It has 12 rows and no `--agent KEY` row.
- `:269-272` says `--from-current` copies `herdr.workspace`, `herdr.grid`, `host.cwd`, `harness.kind`, `model`, `role`, `env`, `context`, `command`, `probe.check` and `probe.expect`, but not `opencode_agent`. `spec_from_pane` copies it (E21), and the same ADR's records table says it is copied (`:48`).
- `docs/adr/ADR-0003.md:93` is the SPEC FLAGS bullet of `pane launch` and `pane relaunch`. `docs_rows.rs` `STORY_GROUPS` assigns both verbs to #644. `docs_cli_test` parses only `holler ...` rows, so it does not see this bullet.
- `#664` edits ADR-0003 too, but its `profile apply` row, not line 93, so one token here does not collide with it.
- The stack rule asks for an ADR that a change extends to be updated in the same change. This story finishes the `--agent` feature on the verbs it owns, so the text belongs here.

**2. The key's text form, and the `(pid P)` clause.**

- `text_value` exists so that "a table cell or a `key=value` field stays one word" and an empty value is never confused with a value (`list.rs:270-279`).
- An `AgentKey` needs no escape, and Scope 5 is right about that. It is not exempt from the empty-value rule, though, because `-` passes `AgentKey::parse`.
- The `(pid P)` clause in the same lines cannot print:
  - `Report` is built from the record this run stored (`Launched.pane`, the `cas_put` result).
  - The engine writes `pid: None` in every record (`tx_launch.rs:391`).
  - So the brief's "appears only for a record written before it" is not so.
  - Keeping the clause is harmless and removing it is inside the Blast radius. That is O's call; nothing depends on it.

**3. and 4. The engine paths, read on the merged tree.** These also settle the outside gate's round-1 items NV-1 and NV-2, which could not be checked on the old base.

- **Relaunch (NV-1).** `relaunch` and `launch` share `run`, which sets `TxFailure.acted` once the act was entered (`tx_launch.rs:206`, `:226`, `:560-595`).
  - `act_live` runs `server::stop_old` before `bring_up`, and only `bring_up`'s error is rolled back.
  - So the old server is not restarted.
  - The record is not written: `cas_put` comes after `bring_up`.
- **The reconcile step (NV-2).** Both verbs print through `emit_outcome`, so the reconcile step is appended once for a relaunch too.
- **Why no check before B1 in this story (finding 4).** It would need a new read-only `HarnessPort` method, which is outside this story. It would also ask the old server, whose agent set can be stale: an operator may relaunch to pick up an agent added since it started. That is why operator decision 1 holds for a relaunch as well.

**5. How to keep the new tests from being copies.**

- **Acceptance 2.** E24's `the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies` (`serve.rs:165-240`) already asserts all four points of the new-module clause:
  - `serve_specs() == [with_agent(AGENT)]`;
  - `server.agent == Some(agent(AGENT))`;
  - `VerifyServing` in the harness log;
  - a successful run.

  Its relaunch sibling covers the relaunch half (`serve.rs:242-266`). So keep only the extension of `launch_with_agent_stores_the_key_in_the_record`.
- **Acceptance 4.** Use one table-driven test over {launch, relaunch} x {a stand-in open code through `fail_next(HarnessOp::VerifyServing, ..)`, the fake's own `agent-mismatch` from a server running another agent}. Put the rollback assertions in one helper in `agent.rs`. Most of the cells are new; only "launch with a stand-in code" overlaps E25, and as one row of a table that is acceptable.
- **Acceptance 5.** The existing tests are `launch_with_profile_adds_the_spec_and_bumps_once` (log `[1, 2]`) and `a_failed_act_restores_the_profile_specs` (log `[1, 2, 3]`), at `launch/profiles.rs:42-86`. Extend the first with `--agent` and assert the key in P's spec. Make the failure half the `--profile` row of the Acceptance 4 table: a `verify_serving` refusal, P's specs equal, the log `[1, 2, 3]`.
- **No rig edit is needed.** Everything `agent.rs` needs is already `pub(crate)` or `pub` in `launch/rig.rs`: `agent`, `launch_with`, `error_of`, `data_of`, `occurrences`, `STEP`, `PANE`, `PORT`, `Rig::after_server_start` and `Rig::record`. `ServeSpec::default().with_agent(..)` (ADR-0021 section 2, decision 3) stands in for `serve.rs`'s private `with_agent`, so no helper is copied.
- **The module index.** The module doc of `tests/pane_verbs/launch.rs` (lines 4-10) lists its sibling modules. Let the `mod agent;` edit add one line for `agent.rs`, or that list goes stale.

**6. The runs going in parallel, as found on their branches and issues today.**

- **#642 part 3** (`issue-642-implementation` at `6752b70`):
  - Blast radius: the OpenCode adapter, `ADR-0021.md`, `CHANGELOG.md`, and one comment in `ports.rs`.
  - Its D5 lists `agent-unusable` wherever the ADR lists `agent-unsupported`. That is the one same-line clash, ADR-0021:609, and this brief already plans for it.
  - Its D4(b) refuses a key that is missing from `GET /agent` (`no agent named <key>`). So this brief's decision 1 composes with it, even though #642's "Out of scope" says otherwise.
- **#649** (issue body, D17): a new "Wiring as built (#649)" paragraph in ADR-0021 and a CHANGELOG entry. Its D5 keeps `launch.rs` out of its radius. No source overlap.
- **#664** (`issue-664-implementation` at `780a578`): ADR-0021 ("Apply as built", the `profile apply` row), ADR-0003 (its row) and CHANGELOG. In code it composes:
  - `apply/rig.rs`'s matching spec is `spec_from_pane`;
  - the test kit's fixtures carry no agent (`fixture.rs:75`, `:111`);
  - no #664 test sets one.

  So adding the row changes none of #664's outcomes. What would mislead is its brief text: item 13 and the C18 notes still say `SpecField::ALL` omits `opencode_agent`, point the row at #647's parked part 2, and ask for a follow-up issue. #664's S phase could act on that.
- **"Decisions taken."** Only this story appends items to it, so there is no numbering clash.

### What holds on the merits (no amendment needed)

- **One `Report` for both verbs.** `relaunch.rs:19,57` prints through `launch.rs`'s `emit_outcome`.
  - `agent` is taken from `launched.pane`, as `spec_only` is. The JSON key therefore cannot disagree with `data.pane.opencode_agent`.
  - The key is additive, after `spec_only`. ADR-0021:679 says that adding a field to a verb's `data` does not change `schema_version`.
  - The test kit's `check_envelope` checks only the envelope's four keys (`envelope.rs:202-207`), so the envelope checker needs no change.
- **The 16th field.**
  - `SpecField::OpencodeAgent` goes between `Effort` and `Role`, the order of `ProfileSpec`'s fields.
  - `FieldValue` derives `Serialize` with `#[serde(untagged)]` (`profile_diff.rs:158-159`), so `Key(Option<String>)` serializes as the string or `null`. That matches the spec's JSON pointer, which reads `null` when the field is absent, so `spec_field_values_serialize_like_the_spec` holds.
  - `differs` needs no new arm, and `diff_spec` goes through `spec_from_pane`, which already maps the field.
  - Every match on `SpecField` or `FieldValue` is inside `profile_diff.rs`. `profile/show.rs` only iterates `ALL` and builds `Text`.
  - Neither type derives `Deserialize`, so no wire or file format changes.
  - The only literal 15-element lists are `ALL` and `PATHS`. The other `; 15]` arrays in the workspace are unrelated conformance cases.
  - `(default)` cannot be an `AgentKey`, and it is not the `none` that `Argv(None)` prints.
- **Scope 3's placement.** The refusal is the harness port's (E15) and the rollback the engine's (E14).
  - `launch_request` and `relaunch_request` read only `ProfileStore::get` and `PaneStore::get` (`launch.rs:92-105`, `relaunch.rs:64-88`). This settles round 1's NV-10.
  - `SpecFlags` is shared only by the two verbs (`grep SpecFlags`), so the new `--agent` help line is true wherever it shows.
- **File sizes.**
  - The source files touched stay small: `launch.rs` 442, `relaunch.rs` 88, `args.rs` 170, `profile_diff.rs` 333.
  - The test files grow a few lines each: `tests/pane_verbs/launch.rs` 740, `relaunch.rs` 629, `launch/profiles.rs` 205, `profile_diff_test.rs` 336, `profile_verbs/show.rs` 325. None nears 800.
  - `launch/rig.rs` (814) and `tx_launch.rs` (831) are not edited.
  - `scripts/lint.sh`'s 900-line gate counts only `crates/**/*.rs`.
- **No other data-key, flag or result-line list.**
  - `cli-surface.txt` lists flag invocations, not help text.
  - No test pins the exact `data` keys of either verb.
  - No README or `docs/` page shows the result line (E35, re-run).
  - `#652`'s branch has no doc content yet.

## Notes for O (non-blocking)

Fold findings 1 to 6 into the issue and copy it again, as round 1 did; none needs a re-review by A. Three notes for T, which keep Acceptance 1 and 4 from being met by tests that prove nothing:

- **Key order is visible only in the raw output.** The workspace's `serde_json` has no `preserve_order` (`Cargo.toml:48-51`), so a parsed `Value` (`check_envelope`'s `data`) sorts the keys. Check the order on the raw stdout, for example the substring `"spec_only":false,"agent":` closing `data`. Note that `data.pane` holds a `profile` key of its own.
- **A refused relaunch.** `Rig::assert_matches` does not hold after a failed relaunch: the record stays and its server is stopped. Assert as `a_failed_relaunch_never_closes_the_records_pane` does (`relaunch.rs:202-222`): the record is unchanged and `StopOwned` appears twice (B1, then the rollback).
- **The natural `agent-mismatch` case.** It needs the server to run another agent than the spec. One way, with no rig edit, is a `Rig::after_server_start` hook that restarts the pane's fake server from an argv with another `--agent`.

## Patterns referenced

1. `crates/holler-cli/src/pane/launch.rs:343-442` and `crates/holler-cli/src/pane/list.rs:270-296`: `Report`, `Report::text`, `emit_outcome` and `failure_body`, which both verbs share, and the one text rule for stored strings (`text_value`, `NO_VALUE`).
2. `crates/holler-pane/src/profile_diff.rs:31-222` and `crates/holler-cli/src/profile/show.rs:134-167`: the one field table, `FieldValue`'s untagged `Serialize` and its `Display`, and the two places `profile show` prints them.
3. `crates/holler-pane/src/tx_launch.rs:187-227`, `:560-620`, `:726-742` and `tx_launch/server.rs:599-620`: the shared `run`, the act with B1 before `bring_up`, the rollback, and `verify_serving` after the health wait.
4. `crates/holler-cli/tests/pane_verbs/launch/serve.rs:24-53`, `:165-266`, `:414-455`, `launch/profiles.rs:42-86`, `relaunch.rs:202-222`, `:563-629` and `launch/rig.rs:276-290`, `:505-580`: the tests and rig hooks the new cases extend or would copy.
5. `docs/adr/ADR-0021.md:121-212`, `:252-275`, `:494-529`, `:609`, `:679`, `:821-884`, `docs/adr/ADR-0003.md:93` and `.aftersight/releases/claude/workflow/parallel-agent-coexistence.md:96-103`: the ADR text this story edits or leaves stale, and the merge rules for the runs going in parallel.
