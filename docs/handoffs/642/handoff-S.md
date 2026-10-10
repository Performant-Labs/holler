# Handoff-S: Phase 10 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort` (spec audit)

**Date:** 2026-10-10, 00:14 MDT
**Branch:** issue-642-implementation at `004275d`. The merge base is `dc300ab`. The branch is not rebased and not yet on
origin. `origin/main` moved to `abdcbb6` (#640's last part, merged 00:06 MDT) during this audit.
**Issue:** #642, part 2 of 3 (642b; epic #633). Rigor: second-opinion. No UI surface.
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md` and `evidence.md`. Also the brief `docs/handoffs/642-brief.md` (all 1,295 lines), the issue
(`gh issue view 642`), and both diff-gate artifacts: r1 in `docs/handoffs/` and the hand rerun r3 in the session scratchpad.
**Diff audited:** `git diff dc300ab..HEAD`, which equals `origin/main...HEAD`. I read these in full:
- `src/{attach,tui,exec,lib}.rs`;
- `tests/{tui_test,attach_test,real_opencode_test}.rs`, `tests/real_opencode/rig.rs`, `tests/support/{fake_tmux,stub}.rs`
  and `tests/fixtures/fake-tmux`;
- the diffs of `hermetic_test.rs`, `CHANGELOG.md`, ADR-0021, `holler-pane` and the manifests.

**Verdict:** REWORK, one TEST-ONLY item: the operator's machine name appears in T-green's handoff. Everything else passes.

## A precondition

Met. `handoff-A.md` is PASS (0 blocks, 9 warns), and `handoff-A-dup.md` is PASS (0 blocks, 3 warns).

## T precondition

Met. `handoff-T-green.md` says "Blocking issues: None".

| Run | Result |
|---|---|
| RED (`handoff-T-red.md`) | `tui_test` 0/13 and `attach_test` 1/22; the one pass is AC 33's fixture precondition. Opted in, the real tests passed 3/9: AC 12 failed on exactly its 8 TUI cases, and AC 13, 14, 15, 18 and 19a failed on `NotImplemented` |
| GREEN | 13/13, 23/23 and the hermetic 30/30. The real tests passed 9/9 twice (OpenCode 1.18.35, tmux 3.7c), and the workspace 1540 passed, 0 failed. T caught all 8 of its mutations once its added test was in |

## Acceptance criteria

These are the brief's canonical 642b ACs. Each proving test asserts one exact answer: an `Ok` value or one named
variant, never `is_err()` alone.

| AC | Proving test or evidence | Status |
|---|---|---|
| 6 (642b clause) | `attach_test::ac6_attach_to_an_unbound_port_is_unavailable_before_the_resolver`: `unavailable`, and the resolver count is 0, through `on_refused_port`. Also conformance case 6 against real OpenCode | Met |
| 9 | `tui_test::ac9_parse_title_reads_a_whole_id_or_home_and_never_guesses`: all seven inputs | Met |
| 10 | `tui_test::ac10_attach_port_reads_a_loopback_attach_line_and_nothing_else` | Met |
| 11a | `tui_test::ac11a_every_builder_targets_the_exact_session` and `ac11a_no_builder_passes_a_bare_session_name_as_a_target` | Met |
| 11b | the three `tui_test::ac11b_*` tests: raw values, `-c /p##S`, `K=v\;`, and `--dir` left unexpanded | Met |
| 11c | `tui_test::ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane`: `Path`, `Name` and `Default`, each with both env removals | Met |
| 11d (642b clause) | `attach_test::ac11d_attach_to_a_session_reply_that_is_not_that_session_is_unavailable`, for 200 HTML and for another id. The message is one line of at most 200 bytes and names the route and the status. The resolver is not called and tmux is not run | Met |
| 11f | `tui_test::ac11f_the_tui_argv_under_inherit_and_isolated` | Met |
| 12 | `real_opencode_test::ac12_the_adapter_passes_the_harness_conformance_suite`: all 15 cases, in two T-green runs | Met |
| 13 to 19a | `real_opencode_test::ac13_*` to `ac19a_*`, 9/9 twice. I checked each against its AC. AC 18 follows B-3's order: serve and create, then no TUI on either pane (by `shown_session` and by raw `#{pane_start_command}`), then the pins, each message saying "pin". AC 19a checks the same `#{pane_pid}` afterwards | Met |
| 20 | T-green: the workspace passed 1540/0 on Linux, and the opt-in run passed 9/9 twice with the versions recorded. The macOS CI leg runs at the PR | Met on Linux; the macOS leg runs at the PR (Advisory 4) |
| 21 | T-green's Tier 1 results (clippy `-D warnings`, rustfmt, `lint.sh`, `machete`). My greps: no `unsafe`, and no `unwrap`/`expect`/`panic!` in `src/`. `tempfile` is used by `fake_tmux.rs` and `rig.rs` | Met |
| 22 | `--name-only` against the merge base lists only allowed paths. Every `holler-pane` `+`/`-` line is `//!` or `///` (scripted). `Cargo.lock` adds only `tempfile` under `holler-adapter-opencode`. `http.rs`, `server.rs`, the test kit and the workspace `Cargo.toml` are unchanged | Met against the merge base; re-check after the rebase (Advisory 3) |
| 23 | The named part-1 sentence is removed exactly. The part-2 entry sits under `[Unreleased]` / Enhancements. It links #633 and #642, says what the three methods do and that the real tests are opt-in, and says the agent comes after #700. It has no closing keyword | Met |
| 24 | The `4700[0-9]\|--continue` grep finds nothing. The rig uses a scratch `HOME`/`XDG_*`, a private `-S` socket and ports 48100-48199. Its only other port is the brief-mandated 127.0.0.1:9 refusal check. No test reads `#{pane_title}` raw: `pane_format` is used only for `pane_dead`, `pane_start_command` and `pane_pid` | Met |
| 25 | The PR body | **Cannot audit yet: there is no PR.** The PR step must do it (Advisory 4) |
| 26 | Advisory 8 has two nits. Otherwise every edit is as specified: <br>- ADR §2's first sentence is rewritten verbatim, and its other two sentences are kept. <br>- The note carries facts 1-7, with fact 2 corrected per A's W-3. <br>- "Deferred" has the two items, plus W-3's #695 item. <br>- `ports.rs:171`, `ports.rs:13-15` and `lib.rs:32-33` are rewritten, and `ports.rs:119` is untouched. <br>- The `lib.rs:37` pointer is repointed, and `exec.rs:1`'s is removed. <br>- The three outside-gate ids in `hermetic_test.rs` are replaced. | Met |
| 27 | the three `tui_test::ac27_*` tests: the exact `QUERY_FORMAT` and vector; `Live` with a whole title holding a TAB; `Dead(Some(7))` and `Dead(None)`; `NoPane` for the four-TAB reply, another name, three fields, a bad `pane_dead` and an empty reply, each against its own contrast | Met |
| 28a-e | `attach_test::ac28a_*` to `ac28e_*`, each exactly as the AC states. 28(a) has exactly one call token list and no HTTP | Met |
| 29a-e | `attach_test::ac29a_*` to `ac29e_*`. 29(c) runs for `sleep 3600`, the four-TAB reply and a dead pane: one query, no request line, and a message that names the pane and echoes nothing | Met |
| 30a-f | `attach_test::ac30a_*`, `ac30b_*`, both `ac30c_*`, `ac30d_*`, `ac30e_*` and `ac30f_*`. Also the unknown-session (404, 400), title-never-confirms and another-server's-TUI tests | Met |
| 31 | `stub::on_refused_port`: a bind-then-drop candidate, refused just before the call. It retries only when the answer is wrong **and** the port now accepts, at most 3 attempts. `closed_port` is private, all 7 former uses in `hermetic_test.rs` go through it, and `Rig::serve` retries "is in use" within 48100-48199 | Met, with a limit in the conformance path (Advisory 9) |
| 32 | No socket option in the new test code. The hermetic files start only the fixture, plus the `false`, `sh` and `kill` that 642a's tests already start. No file is written and then executed. No thread sleeps in the hermetic files: every `sleep` hit is the placeholder string. `SLACK` is 500 ms | Met |
| 33 | `fixtures/fake-tmux` starts with `#!/bin/sh`. Its only commands are `printf`, `[`, `read`, `shift`, `exit`, `:` and `cat`, and it has no command substitution. It also uses `>&2`, one `{ }` group and `\|\|`, which are POSIX `sh`. `git ls-files -s` shows mode 100755, and `fixture()` checks the mode and names `git update-index --chmod=+x` | Met |

## Spec compliance

Each carried decision is built as stated:

| Decision | How it is built |
|---|---|
| 1 | tmux, not Herdr. `tui_session` is typed `PaneName`, and `tui.rs` alone builds targets |
| 3 | `showing()` gives `None` for everything but a live loopback attach with a whole-id title |
| 4 | The port is read from `#{pane_start_command}` on every call |
| 5 | `remain-on-exit on` comes before the respawn |
| 6 | Recorded in fact 4 and the crate docs |
| 9 | The one `Command::new` for tmux is `tui.rs:122`, its only call site is `attach.rs:222`, and all three `"-t"` are followed by `exact_target` |
| 10 | `TMUX` and `TMUX_PANE` are removed, and there is no `-f` |
| 15 | Both divergences are in the crate docs and the ADR note |
| 16 | No agent is read |
| 17 | The homes are as specified, and `lib.rs` is 555 lines |
| 18 and 19 | Met |
| 20 | `http.rs` is untouched |
| 21 | In the `Resolver` doc and fact 6 |
| 22 | Only the closed codes, and `ac30f` guards `NotImplemented` |
| 23 | `QUERY_FORMAT`, `Query` and `parse_query` exactly as written |

The Behaviour section is met:
- Every method takes one deadline at entry. `select_session` builds its `Call` through `call_until`.
- `capture` starts nothing once the deadline has passed.
- A 200 alone is never enough.
- `attach_tui` sends one `GET /session/:id` through `session_reply`, which `known` itself calls. A reply without
  `directory` is `unavailable` before the resolver is called.
- `select_session` runs the resolver, then the one query, then HTTP, and nothing else when there is no TUI.
- `shown_session` never contacts the server.
- No message echoes an argument, a directory, an env value or a title.

A's W-2, W-3, W-4, W-5, W-6, W-8(b) and W-9 are applied. W-1 and W-8(a) are not, because T pinned the brief as written
before O amended it, and W-1 stays a follow-up (Advisory 2).

F made five choices beyond the brief, all disclosed in `handoff-F.md` and `decisions.md`. None contradicts an AC, and I
accept each:

| Choice | Why I accept it |
|---|---|
| The watch requires the requested port as well as the title (decision 1) | It is stricter, for I3, and T pinned it after mutation M1 |
| `select_session` answers `unavailable` ("no TUI in pane P") at once when its watch finds a dead or missing pane, not `timeout` (decision 2) | The brief has no death branch for `select_session`, and both answers exit 1 |
| The UTF-8 check of `opencode_bin` runs before the resolver (decision 3) | The brief does not place the check, and this way nothing is resolved or touched first |
| `Refusal` carries no text, and `Ran::reason` does (decision 4) | The behaviour is the same, and it mirrors #641 |
| `poll` replaces `settled`'s loop (decision 6) | W-6 asked for it, and it is semantically identical: same order, same bound, same `op` |

## Quality audit

- **Correctness and failure handling.** Every reading fails closed:
  - `parse_query` reads a reply with fewer than five fields, another name or a bad `pane_dead` as `NoPane`, and keeps
    the title whole after the fourth TAB.
  - `parse_title` never guesses, and `attach_port` requires `attach` followed by `http://127.0.0.1:<digits>` with a
    nonzero port.
  - `classify` reads only "missing" as no TUI, and anything else is `unavailable` with tmux's first stderr line through
    `one_line`.
  - `capture` drains both pipes on threads, keeps 64 KiB of each and drops the rest, and kills and reaps on the
    deadline. Its unbounded channel means a late send cannot block.

  The adapter keeps no state, so I8 has nothing of the adapter's to compensate.
- **Build guards.**
  - There is no `unwrap`, `expect`, `panic!` or `unsafe` in `src/`.
  - Every new `#[allow]` carries `// #642`.
  - The largest touched file is `hermetic_test.rs` at 798 lines, and nothing reaches 900. `lib.rs` is 555 (under 600),
    and `attach_test.rs` is 678.
  - Clippy `-D warnings` is clean, so there is no dead code.
- **Protocol.** None changes: no wire format, no golden file, no `docs/protocol/v2.md` edit and no new error code
  (Decision 22).
- **Tests.**
  - Real processes run only in the opt-in tier. The hermetic tier runs the committed fixture and the stub, with bounded
    waits and no fixed sleeps.
  - RED came before F's code. T-green ran 8 sequential and 3 concurrent repeats and 8 mutations, and the one gap (M1)
    is closed by a test.
- **Documentation.** The CHANGELOG, ADR-0021 and the `holler-pane` port docs are updated. There is no new CLI surface,
  log event or protocol field.
- **Public-repository privacy.**
  - **One hit:** the operator's machine name, at `docs/handoffs/642/handoff-T-green.md:9` and `:162`. This handoff does
    not repeat it; `grep -n` on those two lines shows it. No
    file on `main` contains it. Handoff directories reach `main`, and a PR's commit list is public in any case.
  - No other hostname, account name, home path, private IP or secret is in the diff. The only IPv4 literals are
    127.0.0.1 and 192.0.2.1 (TEST-NET).
  - The diff's references to `pfleet`, `$WORKFLOW_ROOT` and Aftersight already appear on `main`.
- **Commit and PR hygiene.**
  - The branch's commits are the script's: Conventional subjects with a `Co-Authored-By:` trailer and no session link,
    as on `main`'s recent squash merges. The squash message is what lands.
  - The PR title, body and AI disclosure come at the PR step (Advisory 4).
- **Outside diff gate (second-opinion).** The gate ran by hand: r3 has `finish_reason: stop`, 3255 completion tokens, no
  BLOCK findings, verdict PASS, and the same prompt as r1. The evidence lives outside the repo, and one figure in
  `decisions.md` is wrong (Advisory 6).

## Scope check

F delivered the brief's files and nothing else (AC 22). The additions are the objects that A's warns asked for:
`call_until`, `session_reply`, `poll`, `classify` in `tui.rs`, and the doc sentences for W-3, W-8(b) and W-9. They also
include F's five disclosed choices. T's additions are all ones the brief allows:
- the `support/fake_tmux.rs` split;
- `Stub::json_once`;
- the rig's `bindable` check;
- the D-1 fix to the `stub.rs` header comment.

Nothing is under-delivered against the canonical ACs. AC 25 is the PR step's.

## Verdict

**REWORK**, one item. `reworkKind: test-only`.

1. **`docs/handoffs/642/handoff-T-green.md:9` ("on Linux ([machine name])") and `:162` ("its title is the host
   name ([machine name] here)").** Each line names the operator's machine, and no file on `main` holds that name.
   Replace each with a neutral phrase, for example "on Linux (this machine)" and "its title is the machine's host name".
   - **TEST-ONLY:** no `src/` change and no test change is needed. The only edit is to T's own handoff file.
   - The PR's commit list on GitHub shows every pushed commit, so the name must stay out of the pushed history as well.
     The branch is not on origin yet, and only `5d20f61` (t-green) carries the name. Fold the edit into `5d20f61` before
     the PR step pushes, for example with `git commit --fixup=5d20f61` and then the rebase onto `origin/main` that
     Advisory 3 requires anyway, run with `--autosquash`. That fold is a job for O or the run's agent, not T.

## Advisory notes (non-blocking for 642b; 1 and 2 must land before #644's relaunch uses `attach_tui`)

1. **A respawn keeps the pane's old title (I3 on re-attach).** F found this, and T-green reproduced it on tmux 3.7c.
   - **The failure:** `attach_tui(pane, P, S)` on a pane whose TUI already showed S on P can answer `Ok` on its first
     poll, before the new TUI runs. A TUI that then fails (for example, under `Inherit`, an `opencode_bin` missing from
     the tmux server's `PATH`) is missed.
   - **Why it is not a brief defect for this run:** the brief's Risk 1 assumed every respawned pane shows the host
     name, and AC 11a and 30(a) pin a call sequence with no room for a reset. Nothing calls `attach_tui` yet: #644 and
     #649 are open with no PR.
   - **File it now, as a follow-up that blocks #644:** chain `\; select-pane -t =<name>: -T ''` into the respawn's own
     tmux invocation, update AC 11a and 30(a)'s pins, and add a test where the pane already shows the session.
   - The ADR note and the crate docs do not mention this yet. Cite the issue in the PR body.
2. **A's W-1: `=<name>:` is the active pane of the session's current window.** ADR fact 1 and the crate docs now state
   this as a precondition. **File it before #644 runs against real tmux.** `=<name>:^` alone does not close it: it is
   still the *active* pane of window 0, so a split of the TUI's window would redirect it. Weigh A's option (b), a
   start-command guard before `set-option`, or a pane-id target.
3. **Rebase before merge.** `git merge-tree --write-tree origin/main HEAD`, against `abdcbb6`, shows 4 conflicts:
   - **`docs/adr/ADR-0021.md`, "Deferred" (Risk 8; #640 merged first, `abdcbb6`).** Keep `main`'s HerdrPort sentence
     ("`HerdrPort`: #640 implements it as merged; a pane with no cell ... before #647 and #650 need it"). Drop `main`'s
     "`HarnessPort` in its final form: #635, then #642" and this branch's "`HerdrPort` in its final form: #636, then
     #640". Keep this branch's agent (#700) and #695 items. `main` still calls HerdrPort provisional in §2,
     `ports.rs:119` and `lib.rs`, so 642b's §2 sentence stands.
   - **`tests/support/stub.rs` and `tests/hermetic_test.rs` (#708).** Follow A-dup's W-1: `on_refused_port` stays the
     only public entry point, the private `closed_port` takes #708's held-connection body, and the mechanism is described
     once.
   - **`CHANGELOG.md`.** Append the part-2 entry after `main`'s newer Enhancements entries, and keep the removal of the
     part-1 sentence.

   After the rebase, re-run AC 22 against `origin/main` and the hermetic files (several runs), and re-read the merged
   ADR "Deferred" list and CHANGELOG against AC 23 and 26.
4. **The PR step (AC 20 and 25).** The script opens the PR as `Implements #642` with the body `Closes #642.`. Before
   merging, set:
   - **The title:** a Conventional subject, for example
     `feat(adapter-opencode): the TUI side of HarnessPort, attach, switch and the shown session (#642 part 2 of 3)`.
   - **The body:**
     - `Part of #642; 642c follows`, with no closing keyword in front of any `#N` and no negated form;
     - the AI disclosure (`CONTRIBUTING.md`);
     - how the three `ASSUMPTION (#642 to confirm)` stand: the shown session is read through tmux `#{pane_title}`, so
       the Herdr half no longer applies, and cross-directory `select-session` and aborting a model turn remain
       unverified;
     - `HarnessPort` is confirmed unchanged and now recorded as built in ADR-0021;
     - the two `FakeHarness` divergences and the follow-up issue numbers from Advisories 1, 2 and 5;
     - the open remainder: 642c, which applies the agent after #700.
   - **The squash message:** a `Co-Authored-By:` trailer, and no "Closes #642".
   - **CI:** confirm both CI legs are green. macOS is the only leg where the fixture runs under macOS `/bin/sh`.
5. **Follow-ups that are still unfiled** (checked with `gh` during this audit):
   - P-2: `http.rs` should tolerate macOS's `InvalidInput`, as `9ebedcc` does.
   - P-3: `FakeHarness` parity and retiring the three `ASSUMPTION` comments.
   - #696 has 0 comments. It still needs A-dup's W-2 record (three runners, two tmux mirrors and two fake record
     formats) and 642a's D-2.
6. **The diff-gate evidence.** The hand rerun's review and `usage.json` exist only in the session scratchpad
   (`642-diff-r3.md`, `642-diff-r3.md.usage.json`). r1 in the worktree is still the refusal. O should:
   - copy r3 beside r1 (the path is gitignored, so it stays local);
   - correct `decisions.md`, where the O entry treats the prompt as ~64,093 tokens. The usage reports
     **77,559 prompt tokens**; ~64,093 is the bytes/4 estimate.

   The review is complete, but some of its line numbers are off, and one function it names (`showing_none`) does not
   exist. I triaged its nine needs-verification items against the code, and all resolve in the code's favour:
   - `string_at` returns `None` for a missing key;
   - the channel is unbounded;
   - the quoted line parses (AC 10 test);
   - `on_refused_port` retries as specified;
   - `ac17` waits for busy before it aborts;
   - `known` maps 400 to `session-not-found`;
   - the other three are plain reads of the code.

   Its W-1 is mistaken: `capture` checks the deadline before it spawns.
7. **Untested branches outside the ACs.** No test pins these to one answer:
   - `attach_tui`'s no-pane-during-watch answer ("no tmux session for pane P");
   - `select_session`'s dead or missing pane during its watch (F's decision 2);
   - the non-UTF-8 `opencode_bin` check.

   `ac30f` guards only against `NotImplemented`. Add these in 642c or with Advisory 1's follow-up.
8. **AC 26 nits.**
   - `tui.rs`'s header says part 2 is built but does not name 642c's agent. `lib.rs` does, which meets the AC read
     collectively, and the agent is not a tmux concern.
   - `exec.rs` drops its dangling "decision 11 of the brief" instead of citing the ADR note. That is right, because the
     note has no fact about the runner.
   - `attach.rs`'s module doc says "otherwise the answer is `timeout`", which leaves out the dead and missing answers.
     `watch`'s own doc is exact.
9. **AC 31 in the conformance path.** The suite calls `serve` on the rig's ports itself, so a port lost to another
   process cannot be re-picked there. T-green's `bindable` check reduces the risk. F's 1-in-3 flake did not recur in
   T's 2 runs, which is too few to call it fixed. A test-kit hook to re-pick a case's ports would close it (the test
   kit is out of this run's scope).
10. **A-dup's W-3.** Add one comment line on `rig.rs`'s `tmux()` saying it is deliberately not `tui::tmux_command`: the
    guard and the raw readings must not depend on the code under test.
