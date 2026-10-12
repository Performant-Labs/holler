# Handoff-A-dup: Phase 7 - #647 part 2: `unattached` and `agent-cannot-dispatch`  (anti-duplication gate)

**Date:** 2026-10-12 (first pass at 11:41 MDT; S REWORK round re-reviewed at 12:33 MDT, see "Re-entry" below)
**Branch:** `issue-647-implementation`
**Diff base:** `6eff9cc` (`main`, merged into the branch)   **Diff head:** `1d12249` (this round: `e393e87..1d12249`, which is F's rework `6f07eff` and T's `1d12249`. The first pass was at `b5fc4bb`: F's code `8b6030e`, T's RED `a3a9e2b` and GREEN `4bd4e75` and `b5fc4bb`, and the pure move `3a09f7d`)
**Reuse map:** `docs/handoffs/647-part2-brief.md` D4 ("Reuse, nothing copied"), D8, D13, D14 and D15, as amended by `docs/handoffs/647/handoff-A.md` (W-1, W-2, W-4, W-5, W-6, W-8). For this round, also S's REWORK item 1 (`handoff-S.md`), which named `findings::quoted`
**Verdict:** PASS

## Re-entry (S REWORK round, `e393e87..1d12249`)

### Summary

PASS. There is no block, one new warn (R1) and one carried warn with new facts (R2). The round changed two production files and two test files.

F's rework (`6f07eff`) quotes the profile name in the W-3 clause of `agent-cannot-dispatch`'s message, in two ways:
- **In the prose:** through the existing `findings::quoted`, as S named.
- **As the suggested `--profile` argument:** through a new `findings::single_quoted`, which applies POSIX single quotes.

The new helper is the round's one new object. It is not a parallel path, for two reasons:
- `quoted` does a different job. Its `{:?}` double quotes let a shell expand `$(...)`.
- The only equivalent helpers live in `holler-cli`, and `holler-pane` cannot depend on that crate.

F put the helper in the module that already holds the quoting both crates share, and justified it in writing. It is still the third copy of the same one-line body, so folding the copies is a follow-up (R1).

T's two test edits (`1d12249`) reuse the existing rigs and helpers. T merged the old profile test into its replacement rather than keeping both.

### Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| R1 | warn | `crates/holler-pane/src/findings.rs:439-441` (`single_quoted`) | This is the third copy of the body `format!("'{}'", text.replace('\'', r"'\''"))`. `holler-cli` already keeps two copies on `main`. One is `profile/delete.rs:206-208`, which is `pub(crate)` and also used by `profile/create.rs:251` and `pane/import/writes.rs:83, 180`. The other is `pane/profile_scope.rs:270-272`, which is private. This is not a block; the reasons are listed below the table. | Follow-up, XS. Make `findings::single_quoted` `pub` and delete the two `holler-cli` copies. Each of the six call sites passes a `ProfileName`'s `as_str()`, so each becomes `single_quoted(name)`. Then drop `delete.rs`'s module-doc note (line 16) that `single_quoted` is shared with `create`. |
| R2 | warn (Phase 3 W-2 and first-pass warn 2, carried and updated) | `crates/holler-pane/src/findings.rs:24-30` | #839 (PR #863) is still open, with its head at `e57a7b5` (`gh pr view 863`, checked at 12:31 MDT). It rewrites the module doc's "Remedies" bullet, which this branch also rewrites. This round's new lines sit directly below that bullet, in the "Untrusted text" bullet, so they join the same conflict region. The round adds no second remedy mechanism: `single_quoted` changes only the message. | No change to the plan: whoever merges second keeps one statement of the remedy rule. That merge must also keep the new sentence of the untrusted-text bullet: a profile is named in prose through `quoted`, and as a command argument through `single_quoted`. |

**Why R1 is a warn, not a block.**
- **The object S named is not equivalent for a shell argument, and F followed the dominant pattern.** Some messages both name a profile and suggest a command that takes it. Those use `{:?}` in the prose and POSIX single quotes in the command:
  - `profile/delete.rs:158-183`;
  - `profile/create.rs:251-259`;
  - `pane/import/writes.rs:76-86, 179-192`;
  - `pane/profile_scope.rs:54`.

  ADR-0021 section 3 (lines 530-531) states the same rule for the reconcile step. F's prose-plus-argument split matches that pattern. S's literal prescription, `quoted` in both places, would not have.
- **The equivalent helpers cannot be reached.** `holler-cli` depends on `holler-pane`, not the other way round. Folding the two copies would edit four `holler-cli` files outside this story (`delete.rs`, `create.rs`, `import/writes.rs` and `profile_scope.rs`). That is the unplanned drive-by that `parallel-agent-coexistence.md` bars.
- **The helper is in the right place.** `findings.rs` already holds the quoting and command builders that both crates use. Ten `holler-cli` modules use `findings::quoted`, and `say_cmd.rs` and `profile_scope.rs` use `findings::doctor_command`. Three records make the choice findable:
  - the helper's doc comment names the `holler-cli` copy and the dependency direction;
  - F's handoff records it ("Where the helper lives");
  - so does F's journal entry.
- **There is no rival mechanism inside `holler-pane`.** `observe/agent.rs:134` is the only place where `holler-pane` puts a profile value into a suggested command. No string in `holler-pane/src` builds a `holler profile ...` command with a value, and `profile-drift` names its profile only in prose (`reconcile/drift.rs:80`).

**The first pass's other two warns are unchanged.** Neither file changed in this round: `git diff --stat e393e87 HEAD` is empty for both.
- Warn 1: `scratch/verbs.rs:105-115`, where `assert_end_state` copies `Scratch::findings` inline. It is still T's to fold.
- Warn 3: the host adapter's four `list-panes -s` builders. It is still a follow-up.

### Also checked

- **The prose reuses the existing sanitizer.** It names the profile through `quoted(profile.as_str())`, as the sibling finding `profile-drift` does (`reconcile/drift.rs:80`). No sanitizer was copied or restated.
- **T's tests reuse the existing helpers.**
  - `profile_members_message` (`pane_verbs/doctor/agent.rs:172-190`) is built from `Rig::new`, `Seed::in_profile` and the file's `seed`, `whole` and `one`. It replaces `a_profile_members_message_names_its_profile` and keeps that test's remedy assertion, so no near-duplicate test is left.
  - `a_claude_pane_with_no_tmux_client_is_unattached` (`pane_claude/doctor.rs:203-214`) uses the file's `mixed`, `doctor` and `kinds`, and the kit's `FakeHost::detach_clients`, as `doctor/attach.rs` does.
  - The profile name `It's $(id) Demo` mirrors `profile_scope.rs:555`. That is a `#[cfg(test)]` unit test, which an integration test cannot import, and no shared fixture exists to reuse.
- **The rework introduced no drift.**
  - The remedy is still the table's `holler pane relaunch PANE --agent=KEY`.
  - `Finding` and `FindingKind` are unchanged, and no JSON field changed.
  - The helper is `pub(crate)`, so the crate's public API is unchanged.
  - The guide describes the profile clause abstractly (`docs/pane-control.md:606, 650`), so its text still matches the message.
- **Sizes.** `findings.rs` is 458 lines and `observe/agent.rs` 149. The test files are `doctor/agent.rs` at 408 and `pane_claude/doctor.rs` at 235.
- **Remote `main` is at `05471b9`.** Two commits landed since `6eff9cc`, #847 (11:44 MDT) and #848 (11:56 MDT), and both are test-only. Neither touches `findings.rs`, the `holler-cli` copies or any file of this round.

### Notes for F

None, since the verdict is PASS. R1 is a follow-up candidate for O and the operator. R2 belongs to whoever merges second of #839 and this story.

---

The first pass (at `b5fc4bb`) follows, unchanged.

## Summary

PASS, with no block and three warns. F extended every object the Reuse map named and built no parallel path in production code. The two reads are required methods on the existing `HostPort` and `HarnessPort`. Each has one call site in reconcile, at the two hooks the chain already had. The OpenCode adapter now has one reader of `GET /session/:id/message` (`prompt::messages`) and one of `GET /agent` (`server::agent_list`). The default-agent rule is stated once (`server::as_default`), and `usable` and `agents.rs` share it. That settles the plan review's W-1, the one warn that would have become a block here. The remedy goes through the one table (`remedy_for`, with `remedy` as its delegate) and `with_agent` comes after `with_fix`, as W-2 asked. The warns: a test helper copies `Scratch::findings` inline (T's RED commit); W-2's clash with #839's remedy builders still waits for whichever story merges second; and the host adapter's `list-panes` listing now has four copies, following the model the brief named.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_integration/scratch/verbs.rs:105-115` | `assert_end_state` copies `Scratch::findings` (the same file, lines 35-47) inline instead of calling it: the same `pane doctor` call and the same mapping of a finding to `(pane or "-", kind)`. It came in T's RED commit `a3a9e2b`. Test code only. The likely reason is that the copy keeps the whole report for the failure message, which `findings()` does not return. Acceptance 18 asked for each finding to be "read as `Scratch::findings` reads one". | Extract one private `fn finding_pairs(data: &Value) -> Vec<(String, String)>` and call it from both methods. That keeps the single doctor call and the `{report}` in the message. Or call `self.findings()` and drop the report from the message, since `assert_eq!` already prints both lists. T's to fold, because F writes no test. It needs no F respawn. |
| 2 | warn (Phase 3 W-2, carried) | `crates/holler-pane/src/findings.rs:39-49, 104-146, 373-387`, against `origin/issue-839-implementation` (S PASS at `d4ec665`, `main` merged in at `e57a7b5`, not on `main`) | This branch extends the one table: `remedy_for(pane, fix, agent)` and `Finding::with_agent`, which recomputes the remedy through the table. #839 sets two remedies outside the table instead (`with_switch_remedy`, `without_remedy`). It also rewrites `remedy`'s doc to "The table answers by kind and fix alone; reconcile sets two remedies itself". Both branches edit the same module-doc remedy paragraph and the same `remedy` doc lines. So whichever merges second will hit a textual conflict, and then `findings.rs` will have two mechanisms for one concern: a remedy that carries an observed value. | The second merge keeps one statement of the rule. The module doc names both exceptions: a parsed `AgentKey` joined as `--agent=KEY`, and #839's parsed session id. It also files one follow-up to fold the two mechanisms. For example, `remedy_for`'s third parameter could become an enum of observed values (an `AgentKey` or a parsed session id), so #839's switch remedy also goes through the table, or the reverse. This is not a block: on `main`, this story extended the one table, which is the object the map named. |
| 3 | warn | `crates/holler-adapter-host/src/tmux.rs:164-173` (`attached_listing`); `crates/holler-adapter-host/src/lib.rs:428-436` (`attached_clients`) | This is the fourth copy of the `list-panes -s -t =NAME: -F <format>` builder (`panes` at 126, `stop_listing` at 138, `foreground_listing` at 151). It is also the third copy of the method body `Call::start`, then `run_tmux`, then parse or `unavailable(BAD_*)`, or `refused` (`ps` at 403, `foreground_commands` at 415). F followed the dominant pattern, which the brief named as the model (E61 to E63; the module's "one pure function per command line"). So it is not drift. | Follow-up, XS. Add one private `session_listing(name, format)` that the four named builders call, and one private `listing<T>(&self, op, argv, parse, bad)` that the three methods call. Keep the named functions, which are the module's cadence. The next `list-panes` read would then cost two lines. |

No duplication in the production code F wrote. Each object in the Reuse map was extended:

- **Ports.** `HostPort::attached_clients` and `HarnessPort::agent_view` are required methods on the existing traits, so no new port and no new `Ports` field. `HarnessRouter::agent_view` goes through `by`, and the routing table's endpoint row names it. `AgentView` and `TurnAgent` live in `ports/agent_view.rs`, beside the harness vocabulary (W-5). The dispatchers' sort and dedup live only in `with_dispatchers`, and both the adapter and the fake pass through it.
- **Reconcile.** `check_tmux` asks `attached_clients` after `ps` (`observe.rs:174, 190-203`), with `Finding::about` and `observe_failed`. The agent check is a child module (`observe/agent.rs`, W-8), shaped like `observe/process.rs`, with its call-name constant kept locally as that module keeps `HARNESS_SESSION_ACTIVITY`. It reuses the parent's `Server::has`, `observe_failed` and `own_ports`, and builds the finding as `check_session_of_record` does (`Finding::about(..).with_session(..).with_ports(own_ports(pane))`). There is one call site per new method in `holler-pane` (`observe.rs:191`, `observe/agent.rs:59`).
- **Findings.** There is one table: `remedy` delegates to `remedy_for`. `with_agent` is applied after `with_fix`. `Finding` gains no field, and `relaunch_with_agent` uses the existing `RELAUNCH`.
- **OpenCode adapter (W-1).** `prompt::messages` is the one reader of the message route, quoting no body; `from_messages` now calls it. `server::agent_list` is the one reader of `GET /agent`, and `verify_serving` calls it. `server::as_default` is the one statement of the default-agent rule; `usable` is rebuilt on it, with its refusal messages byte-identical. `agents.rs` reuses `known`, `session_path`, `string_at` and `Call::unexpected(_status)`. A grep finds no other sender of `GET_MESSAGES` or `GET_AGENT`. `can_dispatch` and the `native` test are new, with nothing on `main` to share, as D15 states.
- **Host adapter.** `parse_attached` reuses `decimal`, `refused` and `Refusal::is_missing`.
- **Claude Code.** `ClaudeHarness` and `FakeClaude` call `at.process()?`, the one conversion, before answering the empty view (W-4).
- **Test kit.** `attach_client` and `detach_clients` follow `end_session`, and `seed_turns` and `seed_dispatchers` follow `seed_session`. Both pairs bypass the faults and the log, and reuse `not_found` and the data-directory model. `FakeHarness::agent_view` runs `World::reach` and then `World::known`, in `abort`'s order. `FakeHarnesses` adds one forward.
- **Wiring and doubles.** `BindingHost`, both `Wired` and every test double gain one-line forwards. `Unwired` answers `not-implemented`.

## Also checked

- **Tests reuse the kit and the rigs.** The doctor tests use `doctor/rig.rs` (`make_live`, `calls_since`, `one`, `of_kind`). The OpenCode tests use `support/stub.rs` and `support/fixtures.rs`. The host tests use `tests/common`. `rig::HOST_READS` and `HARNESS_READS` are now the one list that both read-only assertions use (W-6); no `HostOp::Ps`-only assertion is left.
  - `forwarding.rs`'s `Counting` host is new, but there is no recording host to extend. The host doubles are the forwarding `LinkedHost`s, `LostChain`, `TestHost` and the `Mutant`. The brief named the file (W1).
  - `fake_harness_agent_test.rs` restates four one-line helpers of `fake_harness_test.rs`. Each integration test is its own crate, and the kit has no `tests/common`, so this is the directory's cadence.
- **The "newest `max_turns`, oldest first" slice is written twice:** in the adapter (`agents.rs:66`, over raw message infos, before mapping) and in the fake (`harness.rs:585`). This is not flagged, because a fake models the port by design and each is pinned by its own tests.
- **Sizes.** `scripts/lint.sh` exits 0. The touched files above 800 lines are `pane_verbs/launch/rig.rs` (865) and `fake_tmux_test.rs` (822), and the brief named both. `holler-adapter-opencode/src/lib.rs` is 779, so its next addition belongs in a submodule.
- **No change to the hub, the proto or persisted state, and no new error code.** ADR-0021 section 12 took the implementation's clarifications in the same change (W-4's process/port clause, W-7's bound, the sharpened `task`-rule wording).
- **Privacy.** A grep of the added lines finds only `pfleet#372`, which `main` already cites (ADR-0020, ADR-0021, the guide).
- **Outside diff gate.** The `deepseek-v4-pro` round 1 (`647-diff-result-r1.md`) passed, with no duplication finding.

## Notes for F

None. The verdict is PASS. Warn 1 is T's, in a test file. Warns 2 and 3 are follow-up candidates for O and the operator. Warn 2 must be resolved by whoever merges second of #839 and this story.
