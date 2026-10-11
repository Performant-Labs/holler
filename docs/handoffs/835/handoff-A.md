# Handoff-A: #835 - `holler pane import` leaves panes Holler does not drive alone, with `--adopt-claude-code`  (up-front plan review, second pass: the amended brief)

**Date:** 2026-10-11
**Branch:** `issue-835-implementation` at `f41d44e`. The base is `origin/main` at `4366fb8`; I fetched at 15:33 MDT and nothing had merged since.
**Brief reviewed:** `docs/handoffs/835-brief.md` at `f41d44e` (sha256 `18e5debe773dff24...`): the issue, plus the contract amendment (lines 27-81, "§1" to "§7" below)   **Reuse map:** no separate file; pass 1's, updated below   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

This pass replaces pass 1 (BLOCK, `33416fa`). Git history keeps that pass's text.

## Summary

BLOCK: 1 `block`, 7 `warn`.

**The amendment settles pass 1's two blocks.**

- **The port contract (§1).** It names the method, its signature, the tmux format it reads, its answer for a missing session and the records to update. It reads names only, leaves `ps` unchanged, and names the fake and its pins.
- **The rule (§2).** `claude` is left alone; any other answer is imported as today; an unreadable signal fails closed; the name is one constant.
- **Pass 1's warns.** Each is settled or recorded (table below). The plan still extends the right objects: #650's skip path (`Outcome::left_alone`, `Action::Skipped`), the `observe` chain, `HostPort` beside `ps`, and the verb's argument checks.

**The new block is the shape §1 chose.** The method is default-armed, and §6 says not to touch `wiring.rs` (#649). But #649's branch passed its a-dup gate at 15:27 MDT today. It wires the production host as `BindingHost` over `WiredHost` over `TmuxHost`, and both decorators forward only the four methods that exist.

- **The effect.** A default-armed method is not forwarded. So the real import's read reaches the default's refusal for every pane: every pane gets `command-unreadable`, and every real run is refused `import-blocked`.
- **No check catches it.** Nothing fails to compile, in either merge order, and no test of the production chain fails.
- **Pass 1 caused this.** Its finding 6 recommended default-armed to keep out of #649's files, without reading #649's branch. This pass retracts that advice.

**The warns:**

- the records: §13 has no finding table, and `no-tmux-session` is not a code;
- `fake_tmux_test.rs` must change, and #832 now edits the same files;
- the two new codes should name the condition, not the action or "command";
- the rule's limits are unrecorded, and pass 1 misdescribed the old launcher's test;
- a lost chain is still recorded as OpenCode on a guess;
- the constant and its predicate need one named, public home;
- the conformance case must hold on a real tmux.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | §1: "**Default-armed, not required** ... A required method would force edits to `Unwired` in `wiring.rs` (owned by #649, running now)"; §6: "Do **not** touch `wiring.rs` (#649)" | contract shape; coexistence with a story in flight; cross-cutting (port decorators) | **The production host is decorated.** #649 (branch `issue-649-implementation`, a-dup PASS at 15:27 MDT, not merged) builds `ports.host` as `BindingHost` over `WiredHost` over `TmuxHost` (its `crates/holler-cli/src/pane/wiring.rs:95`, `:153`, `:170-175`). Both decorators implement `HostPort` by forwarding the four methods that exist (its `wiring/bind.rs:146-170`, `:306-322`).<br>**A default-armed method is not forwarded.** Both decorators inherit the default. So the real import's read answers the default's refusal for every pane:<ul><li>every pane gets the problem `command-unreadable`;</li><li>every dry run shows every pane blocked;</li><li>every real run is refused `import-blocked`.</li></ul>The fail-closed rule turns one missing forward into an import that cannot run in the operator's cutover (#654).<br>**No check catches it.**<ul><li>Nothing fails to compile, in either merge order.</li><li>#835's tests run on `FakeHost` directly (`tests/pane_verbs/doctor/rig.rs:167-177`).</li><li>#649 runs the host conformance suite over `BindingHost` (its `tests/pane_integration/bind.rs:317-322`). A new case would catch that decorator, but only if CI runs the merged code.</li><li>No test runs the suite over `WiredHost`.</li></ul>**#649's own rule.** Its review's W-4 says that every decorator forwards every port method, the default-armed ones included. It pins this for the harness (`the_wired_harness_forwards_every_method_the_default_armed_ones_included`, its `pane_integration/bind.rs:246`). So §1's reason for default-armed, staying out of #649's files, cannot hold: the forward belongs in those files either way. | **Amend §1 and §6. Recommended (A): make the method required.**<ul><li>Decision 12 (#802, ADR-0021 `:976-978`) chose a required change so that an omission "is a compile error at every call site". Every implementation is in this workspace, so the compiler then finds every decorator and test double, now and later.</li><li>Implementations on `main`: `TmuxHost`, `FakeHost`, the `Mutant` (`host_conformance_test.rs:89`), `TestHost` (`holler-pane/tests/ports_test.rs:202`), `Unwired` (`wiring.rs:151`; `not-implemented`, as its other methods), and the `LinkedHost`s (`tests/pane_verbs/launch/rig.rs:669`, `tests/profile_verbs/apply/rig.rs:640`).</li><li>After #649: `BindingHost`, `Wired<dyn HostPort>`, and `tests/pane_integration/bind.rs`'s `LinkedHost`. Each forwards.</li><li>Drop the default and its open code (`command-names-unsupported`). `command-unreadable` (renamed per finding 4) stays for a read that fails.</li></ul>**Acceptable (B): keep default-armed.** Then:<ul><li>forward in `BindingHost` and `Wired<dyn HostPort>`;</li><li>pin `WiredHost`: run the host suite over `WiredHost::new(Ok(Arc::new(FakeHost::new())))`, beside #649's `BindingHost` run;</li><li>list the open code in §9's open-codes row (ADR-0021 `:755`).</li></ul>**Either way:**<ul><li>Rebase this worktree on `main` once #649 merges, before T writes RED.</li><li>Lift §6's "do not touch `wiring.rs`" for exactly these forwards.</li></ul> |
| 2 | warn | §3: "add it to the finding table in ADR-0021 section 13"; §2: "`no-tmux-session`"; §6's file list | documentation; one source of truth | **§13 has no finding table** (ADR-0021 `:863-899`).<ul><li>The import's codes have one source: `ImportFinding::code()` and `ALL` (`import/findings.rs:101-142`).</li><li>`crates/holler-pane/tests/import_test.rs` pins them with their levels (`NEW_CODES` `:25-36`, `levels()` `:55-82`). §6 does not list it.</li><li>A table in the ADR would be a second list.</li></ul>**`no-tmux-session` is not a code.** `ps`'s `pane-not-found` is `tmux-session-missing` (`import/observe.rs:155-171`), and `import_test.rs:115-118` asserts that `no-tmux-session` never appears.<br>**Counts in prose.** `findings.rs:4-10` says "sixteen kinds" and "the other ten"; `import_test.rs:11-14` says "ten new codes". Both change.<br>**The §9 pins.** `crates/holler-cli/tests/pane_verbs/import.rs:81-114` pins §9's `pane import` row and its open-codes row. | **§13.** Add a "Decided (#835)" paragraph, modelled on "Decided (#650): the launcher table's shape" (`:877-884`). It holds:<ul><li>the signal and the rule;</li><li>the two codes and their levels;</li><li>the switch's refusal;</li><li>the limits (finding 5).</li></ul>No code table.<br>**The rest:**<ul><li>Add `import_test.rs` to §6.</li><li>Correct the prose counts.</li><li>Spell the missing-session code `tmux-session-missing`.</li><li>In §2's "amended since" sentence (`:115-119`), say why A or B was chosen (finding 1).</li></ul> |
| 3 | warn | §6: "new adapter tests go in a new file, not `tests/fake_tmux_test.rs`"; the coexistence notes | size and structure; coexistence | **`fake_tmux_test.rs` must change.**<ul><li>`METHODS: [HostOp; 4]` (`:25-30`) and `call`'s `match` (`:67-75`) are exhaustive over `HostOp`. With a new variant, the file does not compile.</li><li>§1 relies on each method's timeout `op` equalling `HostOp::as_str`. That check is in this file (`every_method_is_bounded_and_reaps_its_child`, `:86-111`, the assertion at `:95`).</li><li>The change is about three lines: 818 to about 821, under the lint's 900.</li></ul>**#832 edits the same places** (PR #836, open):<ul><li>`ports.rs`: a default-armed `HerdrPort::observe_pane`, just above `HostPort`;</li><li>`wiring.rs`: `Unwired`;</li><li>ADR-0021: §2's table (`:106`), §9, and the end of §13.</li></ul>**Size.** Under option A, `tests/pane_verbs/launch/rig.rs` (814 lines) gains a forward of about four lines. | In §6:<ul><li>The `METHODS` and `call` extension in `fake_tmux_test.rs` is required. New test functions go in a new file.</li><li>Name #832 among the coexistence notes; expect textual merges in those files.</li><li>After the rebase, #649 has moved the real-tmux helpers into `tests/real_tmux/mod.rs`. The opt-in case goes beside them.</li></ul> |
| 4 | warn | §3: "Code: `left-alone-not-driven`"; §2: "`command-unreadable`"; §1: e.g. `command-names-unsupported` | naming; codes stable once merged (`findings.rs:10`) | **`left-alone-not-driven` names the action.** The row's `action: skipped` already says that. Every other import code names a condition: `harness-not-opencode`, `tmux-session-missing`, `server-down`, `sources-disagree`.<br>**It also becomes false after #831.** Holler will then drive these panes, and the default leaves them alone only because the switch is off.<br>**"command" already means something in this verb:** the launcher entry's argv. That is `command-not-argv`, the record's `command`, `command_of` (`assess.rs:345`) and the help text (`holler-cli/src/pane/import.rs:55`). A reader of the report will take `command-unreadable` to be about the launcher table. | Name the condition, and what was read. For example:<ul><li>`harness-claude-code`, beside `harness-not-opencode`. It stays true after #831, when an adopted pane can carry it as information.</li><li>`harness-unreadable`, for the read that failed.</li></ul>Under option B, choose the default's code the same way. §3 leaves the final spelling to the ADR. |
| 5 | warn | §2: "Any other answer is imported as OpenCode ... matches the old launcher's own test for a Claude Code process (`claude` on argv[0])" | contract; the rule's stated reach | **The rule sees a Claude Code pane only while `claude` is its foreground process.**<ul><li>A pane whose `claude` has exited, or is suspended, shows its shell: `bash`, the same as the old launcher's OpenCode panes.</li><li>Such a pane is imported as OpenCode and flagged `server-down`. For that pane, this is the misclassification the story fixes.</li><li>No tmux name can tell the two apart, so the rule is the right one. But the brief does not record the limit.</li></ul>**The name is narrower than the old launcher's test.**<ul><li>On Linux, tmux reads the foreground process group leader's argv[0] (`/proc/<pgrp>/cmdline`) and cuts it at the first space. It reduces a name that starts with `/` to its last component.</li><li>`scripts/pane-restart:86` instead matches `(^\|/)claude$` against every argv element. So `node .../claude` passes the old test, and tmux reports `node`.</li><li>Pass 1 described the old test as "`claude` on argv[0]", and §2 repeats that. The mistake is pass 1's.</li></ul> | Record the limits in the §13 paragraph and in the help text:<ul><li>The import identifies a Claude Code pane by a live `claude` in its foreground.</li><li>Run the import, and read its dry run, while those panes' Claude Code sessions are running.</li><li>A Claude Code pane that the dry run shows as `record` is one of these cases.</li></ul>Correct §2's sentence about the old test. |
| 6 | warn | §2: "An unreadable signal fails closed ... It is never recorded as OpenCode on a guess" | correctness of the fail-closed rule | **Two paths still record a pane on a guess.**<ul><li>**A lost chain.** `Observed::Lost` is a chain whose thread could not start or panicked. Today it is the warning `observe-failed`, and the pane is recorded (`assess.rs:237-241`; `findings.rs:153-158`).</li><li>**A failed `ps`.** If `ps` fails with anything but `pane-not-found`, that is a warning and the pane is recorded (`observe.rs:157-171`). The new read must run anyway, or the pane must count as unreadable.</li></ul>§3 also says a left-alone pane has one finding, so `ps`'s finding is dropped for it. | State it in §2: every path on which the names were not read is the unreadable problem and never a record. That covers the read's own error, a read that was skipped and a lost chain. T pins the failed-`ps` case; the lost chain takes the same branch. |
| 7 | warn | §2: "The name lives in one constant in `holler-pane` (so #831 and #832 reuse it); nothing classifies command names anywhere else" | placement; anti-duplication | **The plan does not say where the constant and its predicate live.**<ul><li>A constant in `import/` sits behind the private `observe` module, so #831 and #832 cannot reuse it.</li><li>A predicate written in `observe.rs` would be written again by the next story that needs it.</li></ul> | Put the constant and a public predicate in `pane.rs`, beside `HarnessKind` (`:136-138`; the file is 281 lines). For example, `runs_claude_code(names: &[String]) -> bool`. `import/observe.rs` calls it. |
| 8 | warn | §1: "one conformance case in the test kit's host suite and one `Mutant` break" | contract: the conformance suite binds the real adapter | **The suite runs against real implementations too.** It runs against `TmuxHost` on a private server (`holler-adapter-host/tests/real_tmux_test.rs:206-211`, opt-in), and, after #649, over `BindingHost`.<br>**Two things a real tmux does differently from the fake:**<ul><li>A real session lists its shell (`sh` under the tests' config). So the case must not assert an empty list. That is already the suite's rule for `ps` (`conformance/host.rs:17-20`) and the fake's documented exception (`host.rs:68-72`).</li><li>Right after `run`, tmux can still report `env`, because `env` may not have started `sleep` yet. A single read can therefore miss `sleep`.</li></ul> | **T writes the case to these rules:**<ul><li>a missing session is `pane-not-found`;</li><li>the started program's name appears within a bounded poll;</li><li>another session's program is never listed;</li><li>no assertion of an empty list.</li></ul>**The `Mutant`** breaks one of these rules, for example by answering with every session's names. |

### Pass 1's findings, as the amendment settles them

| Pass 1 | Status now |
|---|---|
| 1 (block): no port contract | **Settled by §1.** It names the method, its signature, the missing-session answer, the tmux format, names only, `ps` unchanged, the records, the fake and the pins. Its shape is finding 1 above. |
| 2 (block): no rule | **Settled by §2:** `claude` only; anything else as today; an unreadable signal fails closed; one constant. Findings 5 to 7 refine it. |
| 3 (warn): level `skipped` | **Recorded.** The level is `info`, as the issue says, and §3 gives the reason. Accepted: the issue is the source of truth. |
| 4 (warn): `usage` | **Settled by §4.** `usage` is kept, with its reason. It is raised at one site, `Input::read` step 1, so a dry run is refused too, and it uses the wording of `not_implemented_message(831)`. The documentation is listed. |
| 5 (warn): after #831 | **Recorded as out of scope (§5).** Naming the owner is the operator's job. |
| 6 (warn): blast radius | **Listed by owner (§6).** Findings 1 and 3 correct the list. |
| 7 (warn): #832 | **Left as a note for the operator (§7).** #832 now has two commits (PR #836). It adds `HerdrPort::observe_pane` (working directory and last screen change) and no harness signal, so there is no second signal today. |

## Notes for O

The automated path stops on this BLOCK (`needs-amendment`). Amend `docs/handoffs/835-brief.md`, then start a **fresh** run, not `resumeFromRunId`. Make the run after #649 merges, from a worktree based on that `main`. This is pass 2: one more BLOCK would mean escalating to the operator.

1. **Finding 1: choose A (required) or B (default-armed with forwards and a `WiredHost` pin).**
   - Rewrite §1's "Default-armed, not required" bullet with the reason for the choice.
   - Add the implementations, or the two forwards, to §6.
   - Lift "do not touch `wiring.rs`" for exactly those forwards.
   - Add the line on sequencing after #649.
   - Under A, delete the default and its open code.
2. **Finding 2: the records.**
   - In §3, replace "the finding table in ADR-0021 section 13" with a "Decided (#835)" paragraph in §13.
   - Add `crates/holler-pane/tests/import_test.rs` to §6.
   - In §2, spell the missing-session code `tmux-session-missing`.
3. **Finding 3: the file list.**
   - Allow the three-line `METHODS` and `call` change in `fake_tmux_test.rs`.
   - Add #832 (PR #836) to the coexistence notes.
4. **Finding 4: rename the two codes** for the condition and for what was read, for example `harness-claude-code` and `harness-unreadable`, or record why not.
5. **Finding 5: record the rule's limits.** Put them in the §13 paragraph and in the help text, and correct §2's "`claude` on argv[0]".
6. **Finding 6: close the remaining paths.** State that a lost chain and a skipped read are unreadable too.
7. **Finding 7: name the home** of the constant and its predicate (`pane.rs`, public).
8. **Finding 8: give T the case's rules:** no empty-list assertion, and a bounded poll for the name.

**For the operator, outside this story.** #832's `HerdrPort::observe_pane` is default-armed too, and #649's `BindingHerdr` and `LazyHerdr` do not forward it. Its default answers an empty observation. So over the real wiring, `pane list` would silently show no working directory and no last screen change. That is #832's to settle, and a reason to prefer A here.

## Reuse map (pass 1's, updated; for the amended brief and the Phase 7 gate)

| Need | Extend, do not copy | Where |
|---|---|---|
| A pane left alone | `Outcome::left_alone`, plus one constructor beside `not_opencode`. `Action::Skipped`. One new kind in `ImportFinding::ALL`, with its `code` and `level`. | `holler-pane/src/import/assess.rs:59-95`; `import.rs:66-89`; `import/findings.rs:62-165` |
| The unreadable signal | One more kind at `Level::Problem`, through `assess`'s existing `blocked`, so `Plan::refusal` refuses it as `import-blocked`. | `assess.rs:126-161`; `import.rs:130-157` |
| The observation | `observe()`'s chain: one more read on the same per-pane thread. `tmux_finding` and `failed_message` for its errors. | `import/observe.rs:79-84`, `:155-181` |
| The rule | One public constant and predicate, beside `HarnessKind`. | `holler-pane/src/pane.rs:136-138` |
| The port read | `HostPort`, beside `ps`. In the tmux adapter: a listing builder beside `panes()` and `stop_listing()`, a strict parser beside `parse_ps` that filters dead panes as `PS_FORMAT` does, and `Call` with `refused()` for its errors. | `ports.rs:203-219`; `holler-adapter-host/src/tmux.rs:28-33`, `:112-136`, `:214-228`; `src/lib.rs:383-392`, `:439-449` |
| The forwards | `BindingHost` and `Wired<dyn HostPort>` (#649). The pin pattern: #649's harness forwarding test and its `BindingHost` conformance run. | #649's `crates/holler-cli/src/pane/wiring/bind.rs:146-170`, `:306-322`; `tests/pane_integration/bind.rs:246`, `:317-322` |
| The fake and its pins | `FakeHost` and a new `HostOp` variant. A case in `conformance/host.rs`, a `Break` in the `Mutant`, and `METHODS` and `call` in `fake_tmux_test.rs`. | `holler-pane-testkit/src/host.rs`; `src/conformance/host.rs`; `tests/host_conformance_test.rs`; `holler-adapter-host/tests/fake_tmux_test.rs:25-30`, `:67-75` |
| The switch | The `PaneImport` arguments, `Input::read` step 1 and `usage()`. `output::not_implemented_message`. | `holler-cli/src/pane/import.rs:79-96`, `:149-166`, `:218-222`; `output.rs:251-255` |
| The docs | ADR-0021 §2 (`:107`, `:115-119`), §9 (`:698`, and `:755` under B only) and §13 (a "Decided (#835)" paragraph); ADR 0003 (`:63`); `cli-surface.txt` (`:166-167`); `CHANGELOG.md` | as listed |

At Phase 7, flag any of these as a parallel path:

- a second tmux runner, or a listing that bypasses `run_tmux` and `Call`;
- a second per-pane thread scaffold in the import;
- an import-private copy of `ps`'s error mapping;
- a new `Action` or `Level`;
- a comparison with `"claude"` outside the one predicate;
- a decorator or test double that does not forward the new method (under B);
- a host read that reaches `TmuxHost` around `BindingHost`.

## Patterns referenced

- `crates/holler-pane/src/import.rs` and `import/{assess,observe,findings}.rs`: the #650 plan, its skip path, its level taxonomy and the lost-chain branch.
- `crates/holler-pane/src/ports.rs` and ADR-0021 §2 and decision 12: the frozen ports, the #645 default-armed precedent and #802's "a compile error at every call site".
- #649's branch: `crates/holler-cli/src/pane/wiring.rs` and `wiring/bind.rs` (the production decorators), `tests/pane_integration/bind.rs` (W-4's forwarding pins), and `docs/handoffs/649/handoff-A.md` (W-4).
- `crates/holler-adapter-host/src/{lib,tmux}.rs` and `tests/fake_tmux_test.rs`: how `ps` lists a session, and the `OP_*` pin.
- `crates/holler-pane-testkit/src/{host.rs,conformance/host.rs}` and `tests/host_conformance_test.rs`; `crates/holler-pane/tests/import_test.rs`; `scripts/pane-restart:78-87`.
