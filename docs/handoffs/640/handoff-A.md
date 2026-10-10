# Handoff-A: Phase 3 - #640 part 3 of 3: the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-640-implementation (head `4c6dbb0`; `origin/main` is now `e327569`, see finding 1)
**Brief reviewed:** `docs/handoffs/640-brief.md`   **Reuse map:** `docs/handoffs/640-brief.md`, section "Reuse map (extend, do not duplicate)" (there is no separate survey)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This file replaces part 2's `handoff-A.md`, as the brief's Handoffs line says. Part 2's is in git at `0ad2d8a`.

## Summary

PASS, with no block findings and six warns. The plan extends the objects its Reuse map names, and only those. It makes
`protocol::excerpt` `pub(crate)`, so the crate keeps one copy. It adds `Tapped::Fail` to the one interceptor. It runs
`run_herdr_conformance` as written rather than copying a case. It renames `Timeout.op` once, at the port boundary, and
leaves the transport alone. It puts the scratch harness in a self-contained `tests/<name>/mod.rs` module next to
`wire_herdr/`. No dependency is added, production code still reads no environment, and every rule this part sets goes
into ADR-0021 in the same change. The warns concern three things. First, the tree under the plan: `origin/main`
merged #641 after the brief's baseline, and that brought a sibling opt-in test with a different convention. Second, the
existing helpers the harness has to re-create, which the Reuse map does not name. Third, a few places where the planned
doc and ADR text would not match the code or the ADR's own bookkeeping.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Header ("Its tree equals `origin/main` at `dc300ab`"); E-5 (#641 "not merged") | baseline currency | `origin/main` moved after the brief's baseline. `e327569` (2026-10-09 8:06 PM MDT) merged #641, `TmuxHost` (PR #706). The brief's last amendment (`4c6dbb0`, 8:13 PM MDT) still calls #641 unmerged. The merge touches `CHANGELOG.md` (its entry is now the last one in `[Unreleased]`), `Cargo.lock` and `crates/holler-adapter-host/**`, and none of the other files this part edits. E-5's #641 citation is now merged code (`crates/holler-adapter-host/src/lib.rs:134-137` on `main`), which strengthens Decision 9. The merge also added the first merged opt-in real-instance test (finding 2) and a second production bounded runner (finding 3). | No plan change. Optionally, merge `origin/main` into the branch before T's RED run, so local RED and GREEN runs and the gates (AC 28, `cargo machete`) see the tree the PR merges into. CI tests the PR's merge ref either way. AC 27's placement is unaffected. |
| 2 | warn | Decision 6 (the variable on top of `#[ignore]`; a missing `herdr` fails), AC 9-11, Decision 13 | pattern consistency | The first merged opt-in real-instance adapter test, `crates/holler-adapter-host/tests/real_tmux_test.rs` (#641, on `main` since `e327569`), uses another convention. It has plain `#[ignore = "needs tmux; run with --ignored"]` (9 tests, from `:206`) and no variable, and it returns early and passes when the tool is missing (`tmux_available`, `:94-102`, "skipped: tmux not found"). This plan adds `HOLLER_HERDR_SCRATCH=1` and fails when the gate is set and `herdr` is missing. Everything else already matches: a `tempfile::Builder` root with a prefix (`:106-109`), a 100-byte socket limit (`SUN_PATH`, `:26`, equal to `SOCKET_PATH_LIMIT`), `TMUX`/`TMUX_PANE` removed, `/bin/sh` forced, a guard whose `Drop` stops only its own server, and the suite run with the server as the guard. One merged instance is not a dominant pattern. The brief's reasons also hold for Herdr in particular: its socket is resolved from `HOME`, `XDG_*` and `HERDR_*`, and a workspace-wide `cargo test -- --ignored` already starts private tmux servers and should not also start a Herdr server. So this is a reasoned divergence, not drift. But the brief was written before the merge and names neither convention. | Add a Reuse-map row for `real_tmux_test.rs` as the sibling pattern, with Decision 6 as the reason this part differs. In Decision 13's `docs/testing.md` section, say that the host adapter's tmux tests are opt-in through `--ignored` alone and why Herdr's tests also need the variable. Otherwise the doc describes one convention while the code has two. Add to "For the operator", item 5: #649 and #667 pick one convention for their scratch instances. |
| 3 | warn | Decisions 3-5 (the bounded `herdr` call, start and prove, stop), AC 13's read polls; the Reuse map | reuse / anti-duplication | The harness builds a bounded runner, a readiness poll, a guard that stops and then kills, and two read polls. The workspace already has each of these, and none can be reached from this crate. (a) `crates/holler-cli/tests/support/mod.rs` has `StateDir` (`:69`), `wait_for` (`:151`), `Hub` and `stop_hub` (`:191`, `:210`), which are this repo's Phase 7 candidates. It is another crate's `tests/` and binds `env!("CARGO_BIN_EXE_holler")` (`:141-142`), which does not compile outside `holler-cli`. (b) `crates/holler-adapter-host/src/exec.rs::run` (`:61`, merged in `e327569`) drains stdout and stderr on threads, polls `try_wait`, and kills and reaps on the deadline. It and `crates/holler-adapter-opencode/src/exec.rs::run` (`:48`) are `pub(crate)`, and ADR-0021 §5 bars an adapter from depending on another adapter. (c) #696 (open) moves one runner into `holler-pane` after #663, and `run_probe` is still a stub (`crates/holler-pane/src/probe.rs:33`). So a new harness is justified, and this handoff records that in writing for Phase 7. But the Reuse map names none of these, and the Decisions describe each wait as a loop of its own. | Add one Reuse-map row that names (a)-(c), says why each is unreachable, and says that #696's runner replaces the harness's when it lands (`holler-pane` is already a normal dependency of this crate). In `scratch_herdr/mod.rs`, keep one bounded runner and one bounded poll helper. Shape the runner like the host adapter's `exec::run`: read the piped stdout without blocking the child, poll `try_wait`, and kill and reap on expiry. `status server --json` is small, but an undrained pipe is the known trap. Shape the poll helper like `wait_for`: check first, then sleep the lesser of the interval and the time left. Use the poll helper for Decisions 3-5 and AC 13's polls. Four loops of the same shape inside one new module would be the near-copy Phase 7 looks for. |
| 4 | warn | D4 (the `Timeout` doc in `error.rs`), A4 (§9's `timeout` row), AC 16's `herdr.connect` | ADR currency / contract wording | AC 16 pins `connect` and `connect_with` to `herdr.connect`, which names no port method. The brief documents this exception (E-8, the Reuse map). Yet D4 says "`op` names the port method, as `<port>.<method>`", and A4 says "Its `op` names the port method that ran out ... in every implementation and fake". Neither leaves room for the constructor, and F may not add it (Decision 11: "not change the meaning"; AC 25: "No other line of the ADR changes"). So the contract crate and the ADR would state a rule that this part's own code departs from on its first call. Two smaller points about A4. Its example `harness.health` is an `op` the real OpenCode adapter never returns: `health` "is never an error" (`crates/holler-adapter-opencode/src/lib.rs:34`), while `harness.serve` and `harness.abort` are pinned there (`tests/hermetic_test.rs:467`, `:569`). And the row sits in `class_of`'s classification table, where the Reason column justifies an exit class. The port contract has its own sentence in §2 (`docs/adr/ADR-0021.md:86-88`: "every method returns within I5's bound (default 10 s) or with `timeout`"). | In D4 and A4, add "and `HerdrAdapter::connect`, which is not a port method, names itself: `herdr.connect`", and use `harness.serve` as A4's second example. Prefer putting the rule in §2's sentence and pointing AC 25's `<port>.<method>` grep there. If it stays in the §9 row, add the exception there. |
| 5 | warn | AC 19's second grep | structural rule precision | `grep -nE 'made\.as_str\(\)\|target\.as_str\(\)\|workspace\.label' crates/holler-adapter-herdr/src/adapter.rs` also prints `adapter.rs:338`, which is `workspace: workspace.label.clone(),` in `snapshot`. That is the label the adapter returns in `HerdrPane.workspace`, not message text. AC 19 asks for each remaining use to be an argument of `excerpt(..)`. Read literally, it either fails S on a correct line or invites F to quote and cut a returned field, which would corrupt `HerdrPane.workspace`. `adapter_test.rs:327-352` and the suite's workspace filter would catch that, so the cost is low. | Exempt `adapter.rs:338` from AC 19 by name. Values the adapter returns stay as they are, and the rule covers message text only. |
| 6 | warn | D1-D6 and A1-A9 as a set | doc currency | Three things stay stale after this part. (a) The `snapshot` doc (`crates/holler-pane/src/ports.rs:143`) says "Every pane Herdr has, with its position". The merged adapter leaves out a pane with no cell (`adapter.rs:19-21`), and A9 will say such a pane "has no place in `HerdrSnapshot`". So the ADR and the port doc would disagree. (b) A7 adds a new **PROPOSED** owner for `host.herdr_api_version`. The ADR's Status line sends readers to "Decisions taken" for PROPOSED items (`:3`). Its last line says open items owned by a named story are listed under "Deferred to named stories" (`:562`). A9 edits that list but adds no line for this item. (c) When this part merges, the test kit's `ASSUMPTION (#640)` comments that it answers become false: `crates/holler-pane-testkit/src/herdr.rs:272-274` ("the port's doc names keys `Enter` and `C-c`"), and `src/conformance/herdr.rs:189-196` and `:337-339`. The brief defers them, and no issue is filed (operator item 4). | (a) Add a doc-only D7 in the same form as D1-D6: "Every pane at a cell of a workspace's grid, with its position. A pane with no cell is not listed (ADR-0021, 'Deferred to named stories')." (b) Add one line to the Deferred list, "Who records `host.herdr_api_version`: PROPOSED, #644 (section 10)", and let AC 25 allow it. For operator item 2: #644's brief (`origin/issue-644-implementation` at `7195993`, `docs/handoffs/644-brief.md:1593`, `:1630`) already reads `version()` at its step 4 and records that string in `host.herdr_api_version`, so the PROPOSED owner matches what #644 plans. (c) File the #638 follow-up before this PR merges, rather than leaving it unfiled. |

None of these blocks. Each one either concerns a convention that has only one merged instance and a written reason
to differ (2), or can be fixed with a sentence in the brief or the docs (1, 3-6). Finding 3's justification is written
here, so a scratch harness built as finding 3 describes is not an unjustified near-copy at Phase 7.

Checked and consistent (no finding):

- **The objects extended.** `protocol::excerpt` (`protocol.rs:580`) becomes the crate's one quoting helper. Every
  Herdr-sent value that `adapter.rs` quotes is one of the four sites the brief names (`:232`, `:238`, `:374`, `:388`). I
  read every other placeholder in the file, and each one quotes a caller value, a computed `GridPos` or a count, so the
  brief's NV-8 question is settled. `Tapped::Fail` extends the one interceptor (`wire_herdr/mod.rs:213-251`), and
  `Tap::exchange` is the only `match` on `Tapped`, so the new variant breaks no existing test. `HerdrConfig::with_workspace`
  exists (`adapter.rs:68`).
- **Decision 9's layering.** The adapter maps a `Timeout` once, at the port boundary, and the transport keeps its wire
  vocabulary underneath (AC 20). The OpenCode adapter has the same shape: its `Call` maps `HttpError::TimedOut` to the
  method's `op` (`holler-adapter-opencode/src/lib.rs:303-333`). The host adapter's `host.*` constants are now merged. No
  code branches on an `op`'s text. `Display`, `detail()` and the wire carry it (`error.rs:559`, `:586`, `:673`), and
  reconcile matches `Timeout { .. }` (`reconcile/observe.rs:170`), so the change touches vocabulary only. The seven
  strings must equal `HerdrOp::as_str` (`holler-pane-testkit/src/herdr.rs:66-76`).
- **Placement.** The harness is a directory module, which follows `wire_herdr/` and `common/`. Its pinned API has no
  adapter type, so #649 can move it to the test kit. That move would add `tempfile` to the test kit, which ADR-0021 §5
  limits to `holler-pane` and `serde_json`, so it needs an ADR edit in #649 (operator item 5). The new
  `adapter_messages_test.rs` keeps `adapter_test.rs` (645 lines) below the overlay's 800-line flag.
- **The stack rules.** No dependency changes (AC 30). The adapter's `src/` still reads no environment and spawns
  nothing (AC 29), and every `herdr` spawn goes through one function (AC 8), a single choke point. No secret or personal
  name appears in the planned text: AC 7's literals are generic, and the handoffs write `<scratch-root>`. No protocol
  file, golden file or `holler-proto` file is touched. The baseline greps of E-5 and E-12 print what the brief says on
  this tree.
- **The contract crate.** D1-D6 change doc comments only, and they bring the docs up to merged behaviour and to ADR §10
  as #683 amended it. The epic's amend-first rule ("If either fails, the ADR (#634) and the contract are amended first,
  in their own PR") covers contract changes that a spike forces. A9 puts the one such change, unplaced panes, in the
  Deferred list, where it belongs.
- **Sizes.** The largest touched files afterwards are `wire_herdr/mod.rs` (about 660 lines), `holler-pane/src/error.rs`
  (710, plus about 4) and `adapter.rs` (about 475). `scripts/lint.sh` warns at 600 and fails at 900.
- **CHANGELOG.** Putting this entry right after the part-2 entry follows part 2's own precedent, since part 2's entry
  sits right after part 1's.

## Notes for O

The verdict is PASS, so nothing is required. On the automated path the warns are advisory, and two of them need no
brief change. T can apply finding 3 while writing the harness (one runner and one poll helper are private helpers the
brief allows). F can apply finding 5 by leaving `adapter.rs:338` as it is. If the brief is reopened before T's RED run,
findings 4 and 6 each cost a sentence or two of D/A text and one line of AC 25. Finding 2 is one Reuse-map row and one
sentence in Decision 13. Findings 2 and 6(b) also feed the operator's items 2 and 5.

## Patterns referenced

1. `crates/holler-adapter-host/tests/real_tmux_test.rs` and `crates/holler-adapter-host/src/exec.rs` (on `main` at
   `e327569`): the sibling opt-in real-instance test and the merged bounded runner.
2. `crates/holler-cli/tests/support/mod.rs` (`StateDir`, `wait_for`, `Hub`, `stop_hub`): the harness helpers this repo
   checks first at Phase 7.
3. `crates/holler-adapter-opencode/src/lib.rs` (`Call`, the `harness.*` ops, `health`) and `src/exec.rs`.
4. `crates/holler-pane-testkit/src/{herdr.rs,fault.rs,conformance/herdr.rs}`: `HerdrOp`, `PortOp`, the 11 cases, the
   `ASSUMPTION (#640)` comments.
5. `docs/adr/ADR-0021.md` §2, §5, §9, §10, "Deferred to named stories", "Decisions taken"; `scripts/lint.sh`; #696.
