# Handoff-A: Phase 3 - #641 host adapter (tmux sessions, process control, the launcher primitive)  (up-front plan review, round 4)

**Date:** 2026-10-09
**Branch:** issue-641-implementation (at 0f18b80)
**Brief reviewed:** docs/handoffs/641-brief.md, as amended after round 3   **Reuse map:** docs/handoffs/641-brief.md §Files "Reuse map"   **Wireframe:** N/A (no UI surface)
**Earlier rounds:**
- Round 1: BLOCK at 69e71b5, with 3 blocks and 9 warns. That handoff is this file as of commit 475d6fe.
- Round 2: BLOCK at 81b0ddd, with 2 blocks and 4 warns. That handoff is this file as of commit 770c948.
- Round 3: BLOCK at 95e2260, with 1 block and 4 warns. That handoff is this file as of commit 2af088d.

Every finding of the three rounds is applied in the brief as amended.
**Verdict:** PASS

## Summary

PASS. Round 3's block (B-6) and its four warns (W-14 to W-17) are applied as asked, and the amendment adds no drift.

The plan stays consistent with the codebase:
- one crate behind the frozen `HostPort`, depending on `holler-pane` only;
- the conformance suite reused, not copied;
- no new error variant;
- the ownership record kept in tmux (I6);
- every new file well under the size gate.

Three new warns follow. All are about how precisely the brief is specified. None is a parallel path or a layer violation.
- **W-18.** The brief assumes the shell window always outlives a stop. When the harness window is the session's last, the TERM ends the session. The next poll then answers `can't find session`, and Decision 8's "`Ok` for `stop_owned`" would return `Ok` with a TERM-ignoring member still running. That is W-14's failure again, and I probed it (Q6). Decision 15 and the #644 row rest on the same assumption.
- **W-19.** AC 6 gives the fakes two answer modes: one answer for every call, or a numbered queue. AC 6h's poll bullets need answers keyed to the arguments and to the kill record. A queue sized by expectation flakes, because the number of polls that fit in a grace depends on timing. AC 6a's "no longer exists" check also runs on the macOS CI leg, which has no `/proc`.
- **W-20.** AC 9's grep forbids `TmuxSocket::Default` and `TmuxSocket::Name` anywhere under `tests/`. AC 6f, a default-run test in `tests/fake_tmux_test.rs`, must build exactly those hosts. Both lines are in every version of the brief.

Each warn has a one-line fix that changes no decision. Because this run continues, T can apply each fix within the brief's intent and journal it (see Notes for O). A fresh run would take them as brief amendments.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-18 | warn | Decision 4 (the polls and the "done" rule) against Decision 8 ("missing ... `Ok` for `stop_owned`"); Decision 15 ("The session and its shell window survive `stop_owned`", "`ps` is never empty while the session exists"); the #644 row ("relaunch = `stop_owned` then `run`") | completeness of the stop (the issue: stop "the processes that pane owns"); forward-compat with #644 | The shell window that `ensure_session` creates is not permanent. An interactive shell exits after an idle `TMOUT`, which hardened hosts often set, or when a person types `exit`. The harness window is then the session's last. The TERM closes that window, the session ends, and the next poll answers `can't find session: NAME`, or `no server running on ...` if it was the server's last session. Both are Decision 8's "missing", which is `Ok` for `stop_owned`. Read literally, that returns `Ok` and skips Decision 4's `kill -s 0` check. A member that ignores TERM and HUP then outlives the stop (probed: Q5, Q6). No AC 6h bullet answers a poll with "missing", and AC 3 and AC 4 keep the shell window, so neither suite would catch it. The same assumption makes two statements false once the shell is gone. A stop can end the session, so #644's relaunch `run` gets `pane-not-found`. And `ps` can be empty while the session exists, because a dead shell pane under a global `remain-on-exit on` is not listed. The suite already says a real session "may end with its last process", and its case 6 ensures the session again (`conformance/host.rs:17-19, 158-172`). Graded warn, like W-14: a process is left running, and no stranger is signalled. | **Decision 4:** "A poll whose listing is missing (Decision 8's strings) lists no pane: every TERMed pane is gone, and a group is done only once `kill -s 0` reports `No such process`. Decision 8's `Ok` applies to the first listing." **AC 6h:** add a bullet. After the TERM, a poll answers `can't find session: demo-c1r1` (exit 1) while `-s 0 -- -101` answers exit 0. After the grace the kill record gains exactly one `-s KILL -- -101`, and `stop_owned` is `Ok` once `-s 0` reports `No such process`. **Decision 15:** qualify both statements with "while the shell window exists". **The #644 row:** relaunch is `stop_owned`, then `ensure_session`, then `run`, as the suite's case 6 does. **Follow-ups:** carry the #644 items to #644 itself as an issue comment the orchestrator files: this one, and B-6's "make `--project` absolute". #644 builds against `FakeHost`, which shows neither. |
| W-19 | warn | AC 6, the fakes ("one for every call or from a numbered queue"); AC 6h ("call by call", "unless a bullet says otherwise the fake `kill` answers `-s 0` with that text", the W-14 bullet's "exit 0 while the kill record holds no `-s KILL -- -101`, and with the text after"); AC 6a ("its pid ... no longer exists") | test-spec precision (the tester overlay: "This repo has a history of fixed-sleep flakes") | AC 6h's answers are keyed to two things AC 6 does not name. **The arguments:** `TERM` and `KILL` succeed while `-s 0` says `No such process`. **The record:** the W-14 bullet answers exit 0 until a `KILL` is recorded. Neither is "one answer for every call", and a numbered queue cannot express them reliably. How many polls fall inside a grace depends on timing, at most grace / 50 ms + 1, so up to 5 for AC 6h's 200 ms. If T sizes a queue to an expected count and a fifth poll lands inside the grace, that poll reads the "gone" answer early, no `KILL` is sent, and the test fails at random. AC 6h's first bullet and its W-10 bullet have the same problem for the fake tmux's listings. Separately, AC 6a is a default-run test, and CI's `cargo test --workspace` also runs on `macos-latest`, which has no `/proc` (`ci.yml:16-21, 128`). The outside review (r1, NV-3 and NV-8) asked the same question about the queue. | **AC 6:** add a third answer mode. The fake `kill` answers by its arguments and may read its own record, so "exit 0 until the record holds `-s KILL -- -101`, then `No such process`" is one script line. The fake tmux's `list-panes` may read the kill record the same way: it lists `101` live until the record holds its `KILL`. If T keeps a numbered queue instead, size it by the poll bound and say so in the test. **AC 6a:** check that the pid is gone with `kill -s 0` through the kill binary or with `ps -p`, never with `/proc`. |
| W-20 | warn | AC 9 ("`grep -rn 'TmuxSocket::Default\|TmuxSocket::Name' crates/holler-adapter-host/tests` prints nothing") against AC 6f ("`TmuxSocket::Name(n)` puts `-L n` ... `TmuxSocket::Default` puts neither"), which §Files places in `tests/fake_tmux_test.rs` | test-spec consistency; test isolation (the issue: no story touches a live fleet) | The two ACs contradict each other, and every version of the brief since 69e71b5 has both. AC 6f cannot show the `-L` flag or the no-flag case without building a `Name` host and a `Default` host in `tests/`. T could satisfy both only by dodging the grep, for example with a braced import, which hides what the check is for. The check does matter. Suppose a fake-tmux test names `Default` and forgets `with_tmux_binary`. It then runs the real `tmux` against the default socket, and that socket may be a live fleet's server: on the pipeline host (Decision 14), or on the self-hosted runner that the ubuntu leg uses for same-repo runs (`ci.yml:35-40`). | Scope AC 9's grep to `tests/real_tmux_test.rs`, which matches its title, "Isolation of real-tmux tests". Keep the check's purpose for the fake tests by adding to AC 6: every host in `fake_tmux_test.rs` is built by one helper that always sets `with_tmux_binary` and `with_kill_binary` to the fakes, and AC 6f's `Name` and `Default` hosts go through that helper. A grep can pin that `TmuxHost::new` appears only once in that file. |

Verified and consistent:

- **Round 3 is applied in full.**
  - B-6(a): Decision 6, the `.` bullet of AC 6g, the second bullet of AC 14, and Decision 12's relative-cwd narrowing.
  - B-6(b):
    - Decision 3's read and its refusal order;
    - Decision 7: the read replaces `has-session`, so a successful `run` makes three spawns;
    - Decision 10: never `display-message` for an existence check;
    - AC 6d (three entries), AC 6e, AC 6g, AC 6i and AC 14;
    - Decisions 12 and 15, the #644 row, Risks and Evidence.
  - W-14 is applied as a fix, not only documented: Decision 4's two-phase stop with the group probe, AC 4's member, the W-14 bullet of AC 6h, and AC 7's signal list.
  - W-15: Decisions 3, 4, 5 and 13. AC 6h gains the `4294967295` pane, and AC 6i gains `0 @7`, `1 @7`, `4294967295 @7` and the `ps` case.
  - W-16: AC 6e is reworded. W-17: the fallback is `tests/common/mod.rs`.
  - O's round-3 transcript agrees with my round-3 probes. O rightly did not re-run the `-1` and pid-1 cases, which address processes the probe does not own.
- **No new duplication.**
  - Remote `main` is still 3bdd129, the branch's merge base.
  - No production crate runs a subprocess under a deadline. `holler-load-test`'s `Command::output()` calls are unbounded harness code, and `probe.rs` is still #663's stub.
  - The only `kill` code in the tree is test-only `libc::kill` in `holler-cli/tests`. So the kill-binary seam is not a parallel path, and `exec.rs` stays the justified new object, with W-7's follow-up.
- **Dependency direction and naming are unchanged and consistent.**
  - Dependencies: `holler-pane` only, with the test kit and `tempfile` as dev-dependencies (ADR-0021 §5; `holler-adapter-herdr/Cargo.toml`).
  - `with_*(mut self) -> Self` matches `holler-hub/src/live.rs:453` and `holler-proto/src/error.rs:264`.
  - Test files open with `#![allow(...)] // #641`, as `holler-adapter-herdr/tests/*.rs` do.
  - AC 6f's re-executed child test has a precedent in `holler-hub/tests/token_store_test.rs:616-624`.
- **The codes fit ADR-0021 §9.** `usage` (exit 2) covers the cwd and `argv[0]` refusals. `unavailable` (exit 1, a runtime failure) covers a missing session directory. No open code is used, which would exit 3.
- **`exec.rs`'s drain-and-wait design holds on real tmux.** No round had probed this before. A `new-session` that starts the server returns with its pipes at EOF (Q1). The daemonized server's fds 0 to 2 are `/dev/null` (Q2), and a started pane's are its pty (Q4). So no drain thread waits on the server.
- **Names cannot become flags.** Pane names start and end with `[0-9a-z]` (`holler-proto/src/vocab.rs` `check_segment`), so no name begins with `-`.
- **Platform.** CI runs `ubuntu-latest` and `macos-latest` only; Windows is off per ADR 0002. So `/bin/sh` fakes are fine, and the default run is also the macOS leg (W-19).
- **Hygiene.** No tracked file holds the host name from the issue title, and the outside-model prompt and result files are git-ignored. The outside review of the current brief (deepseek-v4-pro, r1 at 17:53, after O's 17:48 amendment) is a PASS. Its NV-3 and NV-8 are W-19.

## Probe evidence (round 4)

**Setup.**
- Versions: tmux 3.7c and procps-ng 4.0.4 (Linux).
- Each script ran its own private server, `tmux -S ./s -f /dev/null`, on a relative socket inside a fresh `mktemp -d` directory in the reviewer's session scratchpad.
- Every call ran with `TMUX` and `TMUX_PANE` unset, through `/bin/sh`, so that targets such as `=demo-c1r1:` reach tmux literally.
- An exit trap killed each server and removed its directory. Afterwards no probe directory, tmux server or probe process was left.
- The scripts (`a641r4-probe1.sh`, `a641r4-probe2.sh`) are in the scratchpad, not in the repo.
- The only real signals were one TERM and one KILL, both to the probe's own pane group L (Q6). Every other `kill` call used signal 0.

**Results.** Below, `<dir>` is the probe directory.

- **Q1, a server-starting call through a pipe.** `new-session -d -s demo-c1r1 -c <dir>/A`, with stdout and stderr captured through a pipe, started the server. It exited 0 with empty output, and the pipe reached EOF after 104 ms.
- **Q2, the server's fds.** The server's fds 0, 1 and 2 were `/dev/null`: tmux daemonizes, so the server does not hold the client's pipes.
- **Q3, `run`'s read through a pipe.** It printed `<dir>/A`, exit 0.
- **Q4, `run`'s `new-window` vector through a pipe.** It printed `<pid> @1`, exit 0. The started pane's fds 0 to 2 were its pty, not the client's pipe.
- **Q5, the session gone, no server left.** After `kill-session` of the server's only session, the server exited (exit-empty). Both the `stop_owned` listing form (`list-panes -s -t =demo-c1r1: -F '#{pane_pid} #{pane_dead} #{@holler-pid}'`) and `has-session -t =demo-c1r1` answered `no server running on ./s`, exit 1.
- **Q6, W-18: a stop that closes the session's last window.**
  - A second session, `demo-c9r9`, kept the server up.
  - In `demo-c1r1`, `run`'s vector started `env -- sh leader.sh member.sh member.pid`. The leader was L. The member M ran `trap '' TERM HUP; exec sleep 6` and was in group L. The window was tagged per Decision 3 (`tag exit 0`).
  - Then the shell window was closed with `kill-window`, which left the harness window as the session's only window. The listing before TERM was `[<L> 0 <L>]`.
  - `LC_ALL=C kill -s TERM -- -<L>` exited 0.
  - 0.5 s later, the poll listing (`list-panes -s -t =demo-c1r1: ...`) answered `can't find session: demo-c1r1`, exit 1. At the same time `kill -s 0 -- -<L>` and `kill -s 0 -- <M>` both exited 0, so M was alive.
  - `kill -s KILL -- -<L>` exited 0. 0.2 s later, `kill -s 0 -- -<L>` printed `/usr/bin/kill: (-<L>): No such process`.

## Notes for O

This is a PASS, so nothing stops the run. These notes are here so that each warn gets a decision.

1. **Fixes for this run.** If the run continues on this brief, T can apply W-18 to W-20 within the brief's intent and journal each one in `decisions.md`:
   - W-18: the poll rule and its AC 6h test.
   - W-19: fakes that answer by argument and by record, and a portable pid check.
   - W-20: AC 9's grep read as scoped to `tests/real_tmux_test.rs`, plus the single fake-host helper.

   None changes a decision. F follows the poll rule, which Decision 4's "done" definition already supports.
2. **Fixes for a fresh run.** Apply the Suggested fix column as written. For W-18 that means Decision 4, AC 6h, Decision 15 and the #644 row.
3. **Follow-up to file (W-18).** Leave a comment on #644, and on #670 if its flag layer is the right home, saying two things:
   - `--project` must be absolute before it is recorded, because the real adapter answers `usage` for a relative one when it creates the session;
   - relaunch is `stop_owned`, then `ensure_session`, then `run`.

   `FakeHost` shows neither, so #644's own tests cannot find them.
4. **Process, not architecture.** `decisions.md` has an O entry for the round-1 amendment only. The round-2 and round-3 amendments (95e2260, 0f18b80) are not journaled. Also, the brief's header names deepseek-v4-pro as the outside model and says "the issue and the epic fix it". Neither the issue nor the epic names a model, and the repo's `CLAUDE.md` names glm-5.3-flash. I did not review rigor; reconcile the header before S audits the run's gates.

## Patterns referenced

- `crates/holler-pane/src/ports.rs:156-168` and `crates/holler-pane-testkit/src/conformance/host.rs:17-20, 158-172`: the port, and the suite's reading that a real session "may end with its last process", which is why case 6 ensures the session again.
- `crates/holler-pane-testkit/src/host.rs:52-61, 208-214`: the fake's rules and its missing-session-first order.
- `docs/adr/ADR-0021.md` §2, §5 (lines 171-188) and §9 (lines 321-401): the ports, the crate dependency rules, the closed codes and their exit classes.
- `crates/holler-adapter-herdr/{Cargo.toml, src/lib.rs, tests/common/mod.rs}`: the neighbor adapter's dependencies, crate docs and shared test helpers.
- `.github/workflows/ci.yml:16-21, 35-40`: the OS matrix, and the self-hosted runner for same-repo ubuntu runs.
