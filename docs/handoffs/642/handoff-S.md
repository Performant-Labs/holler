# Handoff-S: Phase 10 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort` (spec audit, round 2)

**Date:** 2026-10-10, 00:37 MDT
**Branch:** `issue-642-implementation` at `40a7753`, 12 commits ahead of `origin/main`. `origin/main` is `abdcbb6` (#640,
merged 00:06 MDT) and is now the merge base. The branch is not on origin yet.
**Issue:** #642, part 2 of 3 (642b; epic #633). Rigor: second-opinion. No UI surface.
**Handoffs reviewed:**
- `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`;
- `handoff-T-green.md`, with its S-round section;
- `handoff-A-dup.md`;
- round 1's `handoff-S.md` (`cab09eb`);
- `decisions.md` and `evidence.md`;
- the brief `docs/handoffs/642-brief.md`, all 1,294 lines, and the issue (`gh issue view 642`);
- the diff-gate artifacts: r1 in `docs/handoffs/` and the hand rerun r3 in the session scratchpad.

**Diff audited:** `git diff origin/main...HEAD`, which equals the two-dot diff.
- Read in full: `src/{attach,tui,exec,lib}.rs`, `tests/{tui_test,attach_test,real_opencode_test}.rs`,
  `tests/real_opencode/rig.rs`, `tests/support/{fake_tmux,stub}.rs` and `tests/fixtures/fake-tmux`.
- Read as diffs: `hermetic_test.rs`, `CHANGELOG.md`, ADR-0021, `holler-pane` and the manifests.
- The merge's own resolution: `git show --remerge-diff f46cd3d`.

**Verdict:** PASS. Round 1's one item is fixed, in the tree and in the history. The merge of `main` is resolved as A-dup
and round 1 asked.

## Round 1, and what changed since

Round 1 (`cab09eb`) was REWORK with one test-only item. T-green's handoff named the operator's machine at lines 9 and
162, and so did the one unpushed commit that carried that text.

- **Fixed.**
  - Line 9 now reads "on Linux (this machine)" and line 162 "its title is the machine's host name".
  - An autosquash folded the edit into the t-green commit, which is now `0033954`.
  - I ran a case-insensitive `git grep` on the tree of every commit in `origin/main..HEAD`, and `git log -S`. No file of
    any of those commits holds the name, and no commit message does. No file on `origin/main` holds it either.
  - The old commit `5d20f61` is still in the local object store. It is not an ancestor of HEAD, and no ref contains it,
    so a push of this branch cannot carry it.
- **`main` is merged in** (`f46cd3d`, 00:19 MDT), not rebased. `--remerge-diff` shows the resolution touched only the
  four conflicts round 1 predicted: `CHANGELOG.md`, `hermetic_test.rs`, `tests/support/stub.rs` and ADR-0021. Each is
  settled as the rules said (Spec compliance, below).
- **Unchanged since earlier phases:**
  - `src/` is identical to F's commit: `git diff 48fcd05 HEAD -- crates/holler-adapter-opencode/src` is empty.
  - Of the tests, only `support/stub.rs` changed after T-green. The real tier does not include `stub.rs`, so T-green's
    real runs still describe this code.
  - Since `dc300ab`, `main` brought into `holler-pane` only doc comments (`error.rs`, `pane.rs`, `ports.rs`,
    `reconcile.rs`) and #663's `probe.rs`, which the adapter does not call.
- **Checked after the merge.**
  - O recorded `tui_test` 13/13, `attach_test` 23/23 and `hermetic_test` 30/30. Workspace clippy `-D warnings` was clean,
    and `lint.sh` exited 0 (O's S-rework-round entry in `decisions.md`).
  - I ran one read-only `rustfmt --check --edition 2021` on `hermetic_test.rs` and `attach_test.rs`, which include
    `stub.rs` and `fake_tmux.rs`: clean.

## A precondition

Met. `handoff-A.md` is PASS (0 blocks, 9 warns), and `handoff-A-dup.md` is PASS (0 blocks, 3 warns).

## T precondition

Met. `handoff-T-green.md` says "Blocking issues: None". Its S-round section changed no test and no code.

| Run | Result |
|---|---|
| RED (`handoff-T-red.md`) | `tui_test` 0/13; `attach_test` 1/22, where the one pass is AC 33's fixture precondition. Opted in, the real tier passed 3/9: AC 12 failed on exactly its 8 TUI cases, and AC 13, 14, 15, 18 and 19a failed on `NotImplemented` |
| GREEN | 13/13, 23/23 and the hermetic 30/30, 8 times in sequence and 3 times concurrently. The real tier passed 9/9 twice (OpenCode 1.18.35, tmux 3.7c), and the workspace 1540/0. T's 8 mutations are all caught |

## Acceptance criteria

These are the brief's canonical 642b ACs. Each proving test asserts one exact answer, an `Ok` value or one named
variant, never `is_err()` alone. The files hold 13 tests (`tui_test.rs`), 23 (`attach_test.rs`) and 9 opt-in tests
(`real_opencode_test.rs`). `hermetic_test.rs` still has its 30.

| AC | Proving test or evidence | Status |
|---|---|---|
| 6 (642b clause) | `attach_test::ac6_attach_to_an_unbound_port_is_unavailable_before_the_resolver`: `unavailable`, and the resolver count is 0, through `on_refused_port`. Also conformance case 6 against real OpenCode | Met |
| 9 | `tui_test::ac9_parse_title_reads_a_whole_id_or_home_and_never_guesses`: all seven inputs | Met |
| 10 | `tui_test::ac10_attach_port_reads_a_loopback_attach_line_and_nothing_else`: both `env` forms, tmux's re-quoted form, no flags, flags in another order, and the `None` lines | Met |
| 11a | `tui_test::ac11a_every_builder_targets_the_exact_session` and `ac11a_no_builder_passes_a_bare_session_name_as_a_target` | Met |
| 11b | The three `tui_test::ac11b_*` tests: raw values, `-c /p##S`, `K=v\;`, and `--dir` left as `/p#S` | Met |
| 11c | `tui_test::ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane`: `Path`, `Name` and `Default`, each with both env removals | Met |
| 11d (642b clause) | `attach_test::ac11d_attach_to_a_session_reply_that_is_not_that_session_is_unavailable`, for 200 HTML and for another id. The message is one line of at most 200 bytes and names the route and the status. The resolver is not called and tmux is not run | Met |
| 11f | `tui_test::ac11f_the_tui_argv_under_inherit_and_isolated` | Met |
| 12 | `real_opencode_test::ac12_the_adapter_passes_the_harness_conformance_suite`: all 15 cases, in both T-green runs | Met |
| 13 to 19a | `real_opencode_test::ac13_*` to `ac19a_*`, 9/9 twice. I read each against its AC. AC 14 polls `shown_session` for up to 2 s and reads `#{pane_dead}` raw. AC 18 follows B-3's order: serve and create; then no TUI on either pane, by `shown_session` and by raw `#{pane_start_command}`; then the pins, each message saying "pin". AC 19a compares raw `#{pane_pid}` before and after | Met |
| 20 | T-green: the workspace passed 1540/0 on Linux, and the opt-in run passed 9/9 twice with the versions recorded. The macOS leg runs in CI at the PR | Met on Linux; macOS at the PR (Advisory 3) |
| 21 | T-green's Tier 1 (clippy `-D warnings`, rustfmt, `lint.sh`, `machete`), and O's clippy and `lint.sh` after the merge. My greps find no `unsafe`, `unwrap`, `expect`, `panic!` or `todo!` in `src/`. All six `#[allow]`s are in tests and carry `// #642`. `tempfile` is a dev-dependency, used by `fake_tmux.rs` and `rig.rs` | Met |
| 22 | Re-checked against the new base. `git diff --name-only origin/main...HEAD` lists only the crate, `CHANGELOG.md`, ADR-0021, `holler-pane`'s `ports.rs` and `lib.rs`, this run's `docs/handoffs/` files, and `Cargo.lock`, which adds only `tempfile` to `holler-adapter-opencode`. Every `+` and `-` line under `crates/holler-pane` is `//!` or `///`. `http.rs`, `server.rs`, the test kit and the workspace `Cargo.toml` are unchanged | Met |
| 23 | The named part-1 sentence is removed exactly. The part-2 entry sits under `[Unreleased]` / Enhancements, right after the part-1 entry. It links #633 and #642, says what the three methods do and that the real tests are opt-in (`HOLLER_TEST_OPENCODE=1`), and says the agent comes after #700. It has no closing keyword | Met |
| 24 | The `4700[0-9]\|--continue` grep finds nothing. The rig uses a scratch `HOME`/`XDG_*`, a private `-S` socket and ports 48100-48199. Its only other port is the brief-mandated 127.0.0.1:9 refusal check. `pane_format` reads only `pane_dead`, `pane_start_command` and `pane_pid`, so no raw `#{pane_title}` is printed | Met |
| 25 | The PR body | **Not auditable here: the PR does not exist yet.** `gh pr list --head` shows only 642a's #705. The PR step does it (Advisory 3) |
| 26 | The edits, each as specified apart from two nits (Advisory 9): <br>- ADR §2's first sentence is rewritten verbatim, and its other two sentences are kept. <br>- The note carries facts 1-7. Fact 1 adds W-1's "active pane of the session's current window", and fact 2 is as A's W-3 corrected it. <br>- "Deferred" follows Risk 8 after #640 (Spec compliance). <br>- `ports.rs:171`, `ports.rs:12-15` and `lib.rs:31-33` are rewritten, and `ports.rs:119` is untouched. <br>- `lib.rs`'s crate docs and its pointer are repointed, and `exec.rs`'s brief pointer is removed. <br>- The three outside-gate ids in `hermetic_test.rs` are replaced | Met |
| 27 | The four `tui_test::ac27_*` tests: <br>- the exact `QUERY_FORMAT` and vector; <br>- `Live`, with a whole title holding a TAB; <br>- `Dead(Some(7))` and `Dead(None)`; <br>- `NoPane` for four TABs, another name, three fields, a bad `pane_dead` and an empty reply, with the contrast asserted first | Met |
| 28a-e | `attach_test::ac28a_*` to `ac28e_*`, each as the AC states. 28(a) has exactly one call token list and no HTTP request, and 28(b) runs all nine cases | Met |
| 29a-e | `attach_test::ac29a_*` to `ac29e_*`. 29(c) runs for `sleep 3600`, the four-TAB reply and a dead pane: the message names the pane and echoes nothing the adapter did not author, the stub sees no request line, and there is exactly one call | Met |
| 30a-f | `attach_test::ac30a_*`, `ac30b_*`, both `ac30c_*`, `ac30d_*`, `ac30e_*` and `ac30f_*`. Also the unknown-session (404 and 400), title-never-confirms and another-server's-TUI tests. `ac30e` is deterministic: with `call = 0`, `http::request` and `exec::capture` both answer `timeout` before they connect or spawn | Met |
| 31 | `stub::on_refused_port` is the only public refused-port helper. It checks that a connect to the candidate is refused just before the call. It repeats, at most 3 attempts in all, only when the answer is wrong **and** the port now accepts. AC 6's test and all 7 refused-port uses in `hermetic_test.rs` go through it, and `closed_port` appears only in `stub.rs`. `Rig::serve` re-picks within 48100-48199 on "is in use". The candidate is now #708's held connection, not bind-then-drop (Spec compliance) | Met, with a limit in the conformance path (Advisory 8) |
| 32 | No socket option is set in the test code. The hermetic files start only the fixture, plus 642a's `false`, `sh` and `kill`, unchanged from `main`. No file is written and then executed. The one `thread::sleep` in the hermetic files is 642a's bounded 20 ms poll. `SLACK` is 500 ms | Met |
| 33 | `fixtures/fake-tmux` starts with `#!/bin/sh`. Its only commands are `printf`, `[`, `read`, `shift`, `exit`, `:` and `cat`, with no command substitution. Its `>&2`, `{ }` group and `\|\|` are POSIX `sh`. `git ls-files -s` shows mode 100755, and `fixture()` checks the mode and names `git update-index --chmod=+x` | Met |

## Spec compliance

Each carried or new decision is built as stated:

| Decision | How it is built |
|---|---|
| 1 | tmux, not Herdr. `tui_session` is typed `PaneName`, and only `tui.rs` builds targets |
| 3 | `showing()` gives `None` for everything but a live loopback attach whose title is a whole id |
| 4 | The port is read from `#{pane_start_command}` on every call |
| 5 | `remain-on-exit on` comes before the respawn |
| 6 | Recorded in fact 4 and the crate docs |
| 9 | The one tmux `Command::new` is `tui.rs:122`, and its one caller is `attach.rs:222`. All three `"-t"` are followed by `exact_target` |
| 10 | `TMUX` and `TMUX_PANE` are removed, and there is no `-f` in `src/` |
| 15 | Both divergences are in the crate docs and the ADR note |
| 16 | No agent is read or applied |
| 17 | The homes are as specified, and `lib.rs` is 555 lines |
| 18 and 19 | The committed fixture; refused ports re-checked |
| 20 | `http.rs` is untouched |
| 21 | In the `Resolver` doc and fact 6 |
| 22 | The three methods build only `SessionNotFound`, `Unavailable` and `Timeout`, and pass the resolver's `Err` through. `ac30f` guards against `NotImplemented` |
| 23 | `QUERY_FORMAT`, `Query` and `parse_query` exactly as written |

The Behaviour section is met:
- Each method takes one deadline at entry. `select_session` builds its `Call` through `call_until`.
- `capture` starts nothing once the deadline has passed.
- A 200 alone is never enough.
- `attach_tui` sends one `GET /session/:id` through `session_reply`, which `known` itself calls. A reply without
  `directory` is `unavailable` before the resolver runs.
- `select_session` runs the resolver, then the one query, then HTTP, and nothing else when there is no TUI.
- `shown_session` never contacts the server.
- No message echoes an argument, a directory, an env value or a title.
- I4 holds: there is no `send-keys` anywhere.

A's W-2 to W-6, W-8(b) and W-9 are applied. W-1 and W-8(a) are not, because T pinned the brief as written. W-1 is
recorded in the crate docs and fact 1, and it stays a follow-up (Advisory 2).

F made five choices beyond the brief, all disclosed, and none contradicts an AC. I accept each, as round 1 did:
- the watch's port check;
- `select_session` failing at once on a dead or missing pane;
- the UTF-8 check before the resolver;
- `classify`, with the text in `Ran::reason`;
- `poll`, which keeps `settled`'s order, bound and `op` (compared against `dc300ab`).

`kill_group` is unchanged.

**The merge resolution,** checked against Risk 8, A's W-3, A-dup's W-1 and round 1's Advisory 3:
- **`stub.rs`.** `on_refused_port` is the only public entry point. The private `closed_port` has #708's held-connection
  body, and the module docs describe the mechanism once.
  - This changes AC 31's step (1) from bind-then-drop to the held connection. I accept it, for four reasons:
    - #708 merged on `main` after the brief was written;
    - A-dup asked for exactly this, to keep AC 31's "one helper";
    - `decisions.md` records it;
    - the new candidate cannot be handed to a later `bind(0)`, so it serves AC 31's purpose better.
  - Steps (2) and (3) are unchanged.
- **`hermetic_test.rs`.** The branch's header is kept, and no test body changed in the resolution.
- **ADR-0021 "Deferred".**
  - Kept: `main`'s #640 sentence ("`HerdrPort`: #640 implements it as merged; ...").
  - Dropped: `main`'s "`HarnessPort` in its final form" and the branch's "`HerdrPort` in its final form".
  - Added: the agent item (#700) and W-3's #695 item.
  - §2's first sentence stands, because `main` still calls `HerdrPort` provisional there (but see Advisory 6).
- **`CHANGELOG.md`.** Both entries are kept, and the part-1 sentence stays removed.

## Quality audit

- **Correctness and failure handling.** Every reading fails closed:
  - `parse_query` reads fewer than five fields, another session's name or a bad `pane_dead` as `NoPane`, and keeps the
    title whole after the fourth TAB.
  - `parse_title` never guesses. `attach_port` requires `attach` followed by `http://127.0.0.1:<digits>`, with a nonzero
    port.
  - `classify` reads only the "missing" messages as no TUI. Anything else is `unavailable`, with tmux's first stderr
    line through `one_line`.
  - `capture` drains both pipes on threads, keeps 64 KiB of each, and kills and reaps on the deadline. Its unbounded
    channel means a late send cannot block.

  The adapter keeps no state, so I8 has nothing of the adapter's to compensate.
- **Build guards.**
  - There is no `unwrap`, `expect`, `panic!`, `todo!` or `unsafe` in `src/`, and every `#[allow]` carries `// #642`.
  - The largest touched `.rs` file is `hermetic_test.rs` at 798 lines, then `attach_test.rs` (678) and `lib.rs` (555).
    Nothing reaches 900. `lint.sh` checks only `crates/**/*.rs`, so the 1,294-line brief is outside it.
  - Clippy `-D warnings` is clean after the merge, so there is no dead code.
- **Protocol.** Nothing changes: no wire format, no golden file, no `docs/protocol/v2.md` edit and no new error code
  (Decision 22).
- **Tests.**
  - Real processes run only in the opt-in tier. The hermetic tier runs the committed fixture and the stub, with bounded
    waits only.
  - RED came before F's code. T's 8 mutations are all caught, and the one gap (M1) is closed by a test.
- **Documentation.** The CHANGELOG, ADR-0021 and the `holler-pane` port docs are updated. There is no new CLI surface,
  log event or protocol field.
- **Public-repository privacy.** Clean.
  - The machine name is gone from the tree and from every branch commit (above).
  - I grepped all 5,748 added lines. They hold no home path, account name, e-mail address, tailnet or private hostname,
    and no secret or key. The only IPv4 literals are 127.0.0.1 and 192.0.2.1 (TEST-NET).
  - `pfleet`, `$WORKFLOW_ROOT` and Aftersight already appear on `main`.
- **Commit and PR hygiene.**
  - Every subject is a Conventional Commit except the merge commit's, which `.githooks/commit-msg` exempts (`Merge *`).
  - Every commit has a `Co-Authored-By:` trailer, and none has a session link, as on `main`'s recent squash merges.
  - No commit message puts a closing keyword in front of a `#N`. The handoff files quote one only to forbid it, and
    GitHub does not act on file contents.
  - The PR title, the body, the AI disclosure and the squash message come at the PR step (Advisory 3).
- **Outside diff gate (second-opinion).** The hand rerun r3 is a real completion, and its prompt is byte-identical to the
  worktree's r1 prompt:
  - `finish_reason: stop`;
  - 77,559 prompt tokens and 3,255 completion tokens;
  - no BLOCK findings, and the verdict PASS.

  It reviewed the code as of T-green. Since then only `stub.rs` (the merge) and doc text have changed, so I accept O's
  choice not to re-run it. The evidence still lives only in the session scratchpad (Advisory 5).

## Scope check

F delivered the brief's files and nothing else (AC 22). Beyond the brief's list, F added:
- the objects A's warns asked for: `call_until`, `session_reply`, `poll`, and `classify` in `tui.rs`;
- the doc sentences for W-3, W-8(b) and W-9;
- F's five disclosed choices.

T's additions are all ones the brief allows:
- the `support/fake_tmux.rs` split;
- `Stub::json_once`;
- the rig's `bindable` check;
- the D-1 fix to the `stub.rs` header.

The merge added nothing beyond the four resolutions. Nothing is under-delivered against the canonical ACs, and AC 25 is
the PR step's.

## Verdict

**PASS.**
- Every canonical 642b AC is met, except AC 25 (the PR body). That one belongs to the PR step by design, because the PR is
  opened after S.
- The decisions are built as stated. The deviations are disclosed and justified: F's five choices, and the
  held-connection candidate the merge brought in.
- The quality checks are clean.
- Round 1's REWORK item is fixed in the tree and in every commit on the branch.

## Advisory notes

These are non-blocking for this verdict. 1, 2 and 3 belong to the PR step, before the merge.

1. **The stale title on a re-attach (I3). File it before merging: it is recorded nowhere durable yet.**
   - **The failure:** `respawn-pane -k` keeps the pane's old title (tmux 3.7c; F found it and T-green reproduced it). So
     `attach_tui(pane, P, S)` on a pane whose TUI already showed S can answer `Ok` on its first poll, before the new TUI
     runs. This affects a relaunch onto the same session, on the same port or a new one. A TUI that then fails, for
     example under `Inherit` with `opencode_bin` missing from the tmux server's `PATH`, is missed. Reconcile (#647) would
     see the dead pane only later.
   - **Why it is not a hold:**
     - nothing calls `attach_tui` before #644 and #649, which are open with no PR;
     - the brief's Risk 1 assumed a respawned pane shows the host name, and AC 11a and 30(a) pin a sequence with no room
       for a reset;
     - A and T rated it non-blocking, and round 1 kept it advisory.
   - **Why it must be filed now:**
     - it is in no doc comment, no ADR fact and no CHANGELOG line;
     - no issue holds it (checked with `gh` during this audit);
     - 642a's handoffs were removed at its PR step, so these may be too.
   - **The follow-up blocks #644.** Chain `\; select-pane -t =<name>: -T ''` into the respawn's own tmux invocation,
     update AC 11a's and 30(a)'s pins, and add a test where the pane already shows the session. Cite the issue in the PR
     body.
2. **A's W-1: `=<name>:` is the active pane of the session's current window.** ADR fact 1 and the crate docs state this
   now, but no issue is filed. File it before #644 runs against real tmux.
   - `=<name>:^` alone does not close it: it is still the *active* pane of window 0, so a split of the TUI's window would
     redirect it.
   - Weigh A's option (b), a start-command guard before `set-option`, or a pane-id target.
3. **The PR step (AC 20 and 25), required before the merge:**
   - **The title.** A Conventional subject, for example
     `feat(adapter-opencode): the TUI side of HarnessPort, attach, switch and the shown session (#642 part 2 of 3)`.
   - **The body.** Replace the script's default body, which puts a closing keyword in front of #642 and would close the
     issue on merge. 642c still remains. The new body holds:
     - `Part of #642; 642c follows`, with no closing keyword in front of any `#N` and no negated form;
     - the AI disclosure (`CONTRIBUTING.md`);
     - how the three `ASSUMPTION (#642 to confirm)` comments stand: the shown session is read through tmux
       `#{pane_title}`, so the Herdr half no longer applies; cross-directory `select-session` and aborting a model turn
       remain unverified;
     - `HarnessPort` confirmed unchanged, and now recorded as built in ADR-0021;
     - the two `FakeHarness` divergences, with the follow-up numbers from Advisories 1, 2 and 4;
     - the open remainder: 642c, which applies the agent after #700.
   - **The squash message.** The repo builds it from the commit messages (`squash_merge_commit_message:
     COMMIT_MESSAGES`): here, 12 phase subjects and a `Merge` line.
     - Pass `--subject` (the title) and `--body` to `gh pr merge --squash`.
     - The body ends in the `Co-Authored-By:` trailer and holds no closing keyword.
     - `delete_branch_on_merge` is off, so also pass `--delete-branch`.
   - **CI.** `test (ubuntu-latest)` and `test (macos-latest)` must both be green. macOS is the only leg where the
     fixture runs under macOS `/bin/sh`, and where AC 20's hermetic tests have not run yet.
4. **Follow-ups that are still unfiled** (checked with `gh`):
   - P-2: `http.rs` should tolerate macOS's `InvalidInput`, as `9ebedcc` does.
   - P-3: `FakeHarness` parity, and retiring the three `ASSUMPTION` comments.
   - #696 has 0 comments. It still needs A-dup's W-2 record (three runners, two tmux mirrors and two fake record
     formats) and 642a's D-2.
5. **The diff-gate record.**
   - **The evidence.** `docs/handoffs/642-diff-result-r1.md` is the runner's refusal, and r3 and its `usage.json` exist
     only in the session scratchpad. Copy them beside r1 as `642-diff-result-r3.md` (gitignored), so the worktree holds
     the run, not only the refusal.
   - **The token count.** O's "Assumed" bullet reads the prompt as ~64,093 tokens, which is the runner's bytes/4 estimate.
     The model counted **77,559**. The runner's own note says that another model (qwen38), over the same chat
     transport, returned a plausible artifact that reviewed nothing for a prompt near 78k tokens.
   - **r3 is a real review,** but its line numbers drift, and it names a function (`showing_none`) that does not exist.
     For a diff this large next time, split the gate with `--diff A..B`, as the runner advises, rather than lifting the
     ceiling.
6. **`HerdrPort`'s "provisional until the spike #636 reports" is stale** (new; not 642b's to change). It appears in ADR §2,
   `ports.rs:12-15` and `:119`, and `lib.rs:31-33`.
   - #636 closed on 2026-10-09 at 10:35 MDT. #640, which merged at 00:06 MDT, left the wording, and "Deferred" now says
     #640 implements `HerdrPort` as merged.
   - 642b kept it, as AC 26 and Risk 8 require. It needs a small doc follow-up in #640's line.
7. **Untested branches outside the ACs.** No test pins these to one answer:
   - `attach_tui`'s "no tmux session for pane P" during the watch;
   - `select_session`'s dead or missing pane during its watch (F's decision 2);
   - the non-UTF-8 `opencode_bin` check.

   `ac30f` guards only against `NotImplemented`. Add these in 642c or with Advisory 1's follow-up.
8. **AC 31 in the conformance path.** The suite calls `serve` on the rig's ports itself, so a port lost to another process
   cannot be re-picked there. T-green's `bindable` check reduces the risk. F's 1-in-3 flake did not recur in T's 2 runs,
   which is too few to call it fixed. A test-kit hook to re-pick a case's ports would close it.
9. **Nits** (fold into 642c):
   - **AC 26, `tui.rs`.** The header describes the built module but does not name 642c's agent. `lib.rs` does, and the
     agent is not a tmux concern.
   - **AC 26, `exec.rs`.** The header drops its brief pointer instead of citing the ADR note. That is right, because the
     note has no fact about the runner.
   - **`attach.rs`'s module doc.** "Otherwise the answer is `timeout`" leaves out the dead and missing answers.
   - **`hermetic_test.rs:4-5`.** It says the helper "retries when another bind took the port". Since #708, `stub.rs` says
     a held port cannot be handed to a later bind, and that the retry is a backstop.
   - **A-dup's W-3.** It was not applied at the merge. Add one comment line on `rig.rs`'s `tmux()`: it is deliberately not
     `tui::tmux_command`, so the guard and the raw readings stay independent of the code under test.
   - **History.** The autosquash folded T's whole re-entry edit into `0033954`, including the "Test-only rework (S
     round)" section. So the t-green commit (23:46 MDT) answers an S round that came after it (00:17 MDT). This is
     harmless under a squash merge.
   - **`handoff-T-green.md:180`.** The heading cites `#678`, which is an unrelated merged PR (the CLI stubs).
