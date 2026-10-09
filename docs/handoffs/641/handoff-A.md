# Handoff-A: Phase 3 - #641 host adapter (tmux sessions, process control, the launcher primitive)  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-641-implementation (at 69e71b5)
**Brief reviewed:** docs/handoffs/641-brief.md   **Reuse map:** docs/handoffs/641-brief.md §Files "Reuse map"   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on three findings. All three are at the boundary where the adapter hands values to tmux or to the OS. The structure of the plan is right:

- one adapter crate that depends on `holler-pane` only;
- the frozen `HostPort` implemented as is;
- the conformance suite reused, not copied;
- no new error code and no `unsafe`;
- ownership recorded inside tmux (I6, no file);
- nothing in the tree to extend (no launcher, no production subprocess helper).

The gap is that tmux re-parses its own command line, and the brief does not account for that:

- **B-1.** An argv element that ends in `;` ends the tmux command, and the elements after it run as tmux commands. On tmux 3.7c, `run`'s exact command line renamed the session. `run-shell` in that position would run a shell.
- **B-2.** tmux format-expands the `-c` start directory. `#(...)` in a real, existing directory name ran a shell command, and Decision 6's directory check does not stop it.
- **B-3.** The `kill` binary has no seam. A default-run test that gives the fake tmux a tagged pane makes the adapter signal a real process group on the machine running the tests. The pipeline host runs the live fleet.

Each fix is an addition to the brief: one escaping rule, one builder method, a test-safety rule and a few ACs. No existing decision has to change.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-1 | block | Decision 3 and AC 6d ("argv passed exactly, never through a shell"); Risks | cross-cutting: validation at a trust boundary (epic B2) | tmux splits its argument vector into commands at every argument that ends in `;`. It does this before option parsing and regardless of `--`, and it turns a trailing `\;` into `;`. Verified on tmux 3.7c, private `-S` socket: `new-window -d -P -F '#{pane_pid} #{window_id}' -t '=demo-c1r1:' -- env -- sh -c '<write args to a file>' <file> 'x;' rename-session pwned`. The program received only `x`, and the session was renamed `pwned`. Separately, `'y\;'` arrived as `y;`. So the plan breaks the fake's rule "an element is never joined to another or re-split" (`holler-pane-testkit/src/host.rs:55-57`). It also breaks the port's "never through a shell" (`ports.rs:160`) once the element after the `;` is `run-shell`, and epic B2. AC 6d's `"$(id);x"` does not end in `;`, so it cannot catch this. | In `tmux.rs`, one pure function escapes every value the adapter did not write itself before it enters a tmux command vector: a value that ends in `;` gets a `\` inserted before that final `;`. Verified: `x\;`, `y\\;` and `\;` arrive as `x;`, `y\;` and `;`, and nothing else changes. `{`, `}`, `#{session_name}`, `~`, `-t`, `a b` and `""` passed through literally without any escape. Pin it in AC 6d and in a real-tmux test (Notes for O, items 1, 2, 4). |
| B-2 | block | Decision 6 (`new-session -d -s NAME -c CWD`) | cross-cutting: validation at a trust boundary (epic B2) | tmux format-expands the start directory. Verified: `-c "<dir>/p#S"` gave `session_path` `<dir>/p`. `-c "<dir>/#(touch <dir>/job-ran)"`, a real directory made with `mkdir -p` that passes Decision 6's "existing directory" check, **ran `touch` through the tmux server's shell**. The cwd is the stored `host.cwd`, the same operator- and profile-supplied data the argv guard exists for. Even a harmless path with a `#` (`proj#2`) starts the session in the wrong directory without any error, and the harness then works on the wrong project. The cwd is also an element of the vector, so a trailing `;` truncates it (B-1). | `ensure_session` doubles every `#` in the cwd (`##` is a literal `#`), then applies B-1's escape. Decision 6's check stays as it is, on the unescaped value. Verified: with `#` doubled, `session_path` equals both paths above exactly and no job runs. |
| B-3 | block | Decision 1 builder (the tmux binary is injectable, `kill` is fixed to `PATH`); Decision 4; AC 6 (AC 6e in particular) | testability seam; epic rule "no story touches a live fleet or a running pane" | The fake tmux answers every call with one canned stdout. Suppose a default-run test drives `stop_owned` with a canned `list-panes` line whose tag equals its pid; AC 6e ("every recorded call that names the session") invites one. The adapter then runs the real `kill -s TERM -- -<pid>`, and `kill -s KILL` after the grace, against whatever process group has that number on the machine running the tests. That machine is the pipeline host, where the live fleet runs. Without a seam, Decision 4 also has no default-run coverage: the ownership filter, TERM then KILL, and "No such process" counted as gone. AC 3 and AC 4 are `#[ignore]`, and CI never runs them (Decision 11). | Add `with_kill_binary(PathBuf)` (default: `kill` from `PATH`) beside `with_tmux_binary`, and send every signal through it. Add the rule: "no default-run test lets the adapter signal a pid the test did not spawn. A test that can reach a signal uses a fake `kill` that only records its arguments". Suggested default-run ACs for Decision 4 are in Notes for O, item 5. |
| W-1 | warn | Decision 6 cwd check; Decision 7 `argv[0]` containing `=`; the brief's claim "the adapter must match [the fake] on every input" | pattern consistency (fake and adapter parity) | Two refusals exist only in the adapter. (a) A cwd that is not an existing directory is `usage` even when the session already exists. For an existing session the fake answers `Ok` and ignores the cwd (`host.rs:52-53`, `170-179`). (b) An `argv[0]` containing `=` is `usage`; the fake would run it. Verbs are tested on the fake, so neither refusal shows up in their tests. | List both as deliberate narrowings, the way Decision 12 records ownership: in the crate docs' error table and in decisions.md. For (a), consider checking the cwd only when the session will be created: on a bad cwd, ask `has-session -t =NAME` and answer `Ok` if the session exists. Then a launch onto a leftover session (W-6) does not exit 2. Changing `FakeHost` is outside this blast radius; it is a follow-up only if the operator wants parity. |
| W-2 | warn | Decision 7 (`argv[0]` containing `=` is `usage`); Decision 8 (`Unavailable` carries tmux's first stderr line) | identity and secrets | An `argv[0]` containing `=` has the shape a `NAME=value` secret arrives in. The repo's rule is that a refusal never echoes such text: `holler-pane/src/argv.rs:7-12` and `96-100` (`EnvVarName`), and `error.rs:392-393` ("none carries a secret"). | The `usage` message names the rule (`argv[0]` contains `=`), never the element. `Unavailable.what` is tmux's stderr line, never an argv element or the cwd. T asserts that a sentinel value placed in `argv[0]` does not appear in the message. |
| W-3 | warn | Decision 3 (`run` is `new-window` then `set-option`) | transactions and ownership | Suppose `set-option` fails for a reason other than the window being gone, for example because the deadline runs out between the two calls. `run` returns an error but leaves an untagged process running. `stop_owned` can never stop it, and #644's rollback relies on `stop_owned`, so a retry starts a second copy. | On that failure, signal the process group of the pid `new-window` printed (KILL, through B-3's seam) before returning the error. Parse the `new-window -P` output strictly (`<u32> @<digits>`) before reusing it. Keep the two calls. A single invocation, `new-window ... \; set-option -w -F @holler-pid '#{pane_pid}'`, did not tag the new window on tmux 3.7c (probe K2 below). |
| W-4 | warn | AC 7 (`no_broad_kill_in_source` over `include_str!` of every `src/*.rs`) | pattern consistency | The repo's source-guard tests walk `CARGO_MANIFEST_DIR/src` at runtime: `crates/holler-hub/tests/logging_guard_test.rs:20-70`, and `crates/holler-cli/tests/hold_single_path_test.rs:15-40`, which also strips `//` comments. A fixed `include_str!` list misses any file added later. AC 7's `grep` also counts comments, so the crate docs could not even name the tools the adapter never uses. | Walk `src/` at runtime as the precedents do, and skip `//` comment text. In AC 7's `grep`, apply the same comment exclusion, or word the rustdoc without those names. |
| W-5 | warn | AC 8 gates; Test plan (RED) | process and gates | AC 8 leaves out two CI checks. `bash scripts/lint.sh` is CI's first step: it requires a trailing `// #641` on every `#[allow]`, including the test files' `#![allow(clippy::unwrap_used, ...)]`, and it fails any file at 900 lines. `bash scripts/changelog-check.sh` is also missing. The tester overlay's Tier 1 runs `cargo clippy --workspace --all-targets -- -D warnings`. The plan allows a compile-failure RED, which `docs/agent-overlays/tester.md` forbids ("RED means ... not on a compile error"). With T running before F, nobody is named to land the type skeleton a behavioural RED needs. | Add the two scripts and the workspace-wide clippy to AC 8. O or T decide the RED form and who lands `TmuxHost`/`TmuxSocket` answering `not-implemented`, and journal the choice. |
| W-6 | warn | Forward-compat table | forward compatibility | Decisions 3 and 5 have three consequences that consumers need to know and that the table does not list. (a) `run` creates its window detached (`-d`), so a client attached to the session keeps showing the shell window. (b) The session and its shell survive `stop_owned`, and `HostPort` has no call that ends a session. So `close` (#646) leaves the tmux session behind (the next launch of that name reuses it), and `ps` is never empty while the session exists. (c) Processes the external launcher started before cutover are untagged, so `stop_owned` never stops them. This affects #650 and #654: the first relaunch of an imported pane. | Write (a), (b) and (c) into the crate docs and decisions.md, and add rows for #646 and for #650/#654 to the forward-compat table. No code change in this story. |
| W-7 | warn | Reuse map row `run_probe` ("new, justified") | duplication (future) | `exec.rs` will be the first bounded-subprocess runner in the workspace's production code. #663 will write a second one inside holler-pane for `run_probe` (ADR-0021 §5: holler-pane's one side effect is the probe runner). The justification holds today: `run_probe` is a stub, and holler-pane cannot depend on an adapter. | Record a follow-up for #663: expose its bounded runner from holler-pane, or say why it can't. Then this adapter can later switch `exec.rs` to it, and the two runners do not drift apart. |
| W-8 | warn | Ambient tmux configuration (`TmuxSocket::Default` loads the operator's `~/.tmux.conf`) | cross-cutting: configuration assumptions | The tests force a config that sets only `default-shell`, so they never meet options that change what the design assumes. With `remain-on-exit on`, a stopped pane stays listed as dead. Decision 4's "poll until the pane is gone" then waits out the whole grace on every stop, and dead windows from every `run` pile up. Likewise, `exit-unattached on` ends a server that `new-session -d` just started. | Count `#{pane_dead}=1` as gone in the stop poll (`ps` already reads it). Consider `set-option -w -t <window_id> remain-on-exit off` in the same tmux invocation as the tag. Name the tmux defaults the design assumes in the crate docs. |
| W-9 | warn | Issue title | public repository | Issue #641's title ends with "for Jupiter", a personal host name. The brief rightly leaves it out. | Keep it out of the CHANGELOG entry, the PR title and body, commit messages, rustdoc and test names. |

The rest of the plan is consistent with existing patterns. Verified:

- **Dependencies.** The direction matches ADR-0021 §5. Nothing depends on `holler-adapter-host` (`grep` over every manifest and source file). The test kit does not depend on any adapter.
- **Nothing to extend.** No production code in the workspace spawns a process; only `holler-load-test` and test harnesses do. No launcher exists in the repo.
- **Conformance.** `run_host_conformance(fresh)` with a per-case guard is exactly what the suite's rustdoc prescribes (`conformance/host.rs:75-92`).
- **Errors.** The `Timeout { op }` names match `HostOp::as_str` (`host.rs:35-44`), and every error used is in the closed `PaneError` set.
- **Builder.** The `with_*` methods follow `with_holds` (`holler-hub/src/live.rs:453`) and `with_hold_kind` (`holler-proto/src/error.rs:264`).
- **Targets.** `=NAME` targets are safe given the ADR 0005 name grammar.
- **Signals.** The `kill` binary keeps the crate free of `unsafe` and of new dependencies; `libc` is a test-harness dependency only.
- **Re-exec test.** AC 6f's re-executed child test has a precedent in `holler-hub/tests/token_store_test.rs:616-625`.
- **Sizes and changelog.** Every file fits well under 900 lines, and the CHANGELOG entry follows the convention of every story in this epic.
- **The brief's own tmux observations.** I re-checked them: `env --` execs in place (PID = PGID = SID, the command name is `sleep`). `new-session -c <missing dir>` exits 0, so Decision 6's check is needed. A server-starting `new-session` does not hold the caller's stdout/stderr pipe (no hang when the output is drained to EOF).

## Probe evidence

tmux 3.7c on Linux. Every probe ran on a private server: `tmux -S /tmp/hlr-a641-XXXXXX/s -f /dev/null` with `TMUX` and `TMUX_PANE` unset. Each run killed its server and deleted its directory on exit, and no `hlr-a641-*` directory is left. The probe scripts were kept in the reviewer's session scratchpad, not in the repo. Everything below is reproducible with these commands.

- **E1 (B-1).** `new-window -d -P -F '#{pane_pid} #{window_id}' -t '=demo-c1r1:' -- env -- sh -c 'printf "%s|" "$@" > "$0.out"' <dir>/e1 'x;' rename-session pwned`. Result: `e1.out` held only `x|`, and `list-sessions` printed `pwned`.
- **E2 (B-1).** The element `'y\;'` arrived as `y;`.
- **Man page (B-1).** `man tmux`, PARSING SYNTAX, documents this: "a trailing semicolon is also interpreted as a command separator", and "Individual semicolons or trailing semicolons that should be interpreted as arguments should be escaped twice ... `$ tmux neww 'foo\;' bar`". So the escape in B-1 is tmux's own documented one.
- **R1 (fix for B-1).** The elements `'x\;' 'y\\;' '\;' '{' '}' '{ display-message hi }' '#{session_name}' '~' '-t' 'a b' ''` arrived as `x;`, `y\;`, `;`, then each of the rest unchanged. The session kept its name.
- **R2.** Braces alone (`'{' 'kill-server' '}'`) were passed literally; no command block was parsed.
- **E3 and E4 (B-2).** `-c "<dir>/p#S"` gave `session_path` `<dir>/p`. `-c "<dir>/#(touch <dir>/job-ran)"`, after `mkdir -p` of that path, created `<dir>/job-ran`.
- **C1 and C2 (fix for B-2).** The same two paths with every `#` doubled gave a `session_path` equal to the original path, and no job ran.
- **E5.** `new-session -c <missing dir>` exited 0 with `session_path` set to the missing path.
- **K1.** `env -- sleep 7` gave `comm=sleep`, `pgid == sid == pane_pid`.
- **K2.** `new-window ... \; set-option -w -F @holler-pid '#{pane_pid}'` in one invocation exited 0, but `#{@holler-pid}` was empty on every window.

## Notes for O

These amendments are all additions; no decision is reversed.

1. **Add Decision 13, "Escaping at tmux's command line" (B-1, B-2).** tmux splits its argument vector into commands at every argument that ends in `;`, before option parsing and after `--` too, and it turns a trailing `\;` into `;`. tmux also format-expands the `-c` start directory: `#S`, `#{...}`, and `#(cmd)`, which runs `cmd` through a shell. So:
   - `tmux.rs` passes every value it did not author through one pure escape function. That covers each argv element after `--` and the cwd. A value that ends in `;` gets a `\` inserted before that last `;`.
   - The cwd first has every `#` doubled (`##`).
   - Constant formats such as `-F '#{pane_pid} #{window_id}'` are not escaped.
   - Values read back from tmux (the pid and the window id) are parsed and validated (`u32`; `@` followed by digits) before they are reused in a command.

   Add a Risks line citing the probe (E1, E4).
2. **Extend AC 6d (B-1).** `run(demo-c1r1, ["prog", "x;", "y\\;", ";", "kill-server"])` makes the fake record, after `env --`, these separate arguments in order: `prog`, `x\;`, `y\\;`, `\;`, `kill-server`.
3. **Extend AC 6g (B-2).** `ensure_session` with an existing directory `<tmp>/p#S` records `-c <tmp>/p##S`. With an existing directory `<tmp>/q;` it records `-c <tmp>/q\;`.
4. **Add AC 11, a real-tmux test (`#[ignore]`) named `argv_and_cwd_pass_exactly` (B-1, B-2).** It uses the AC 9 helper and checks two things.
   - `run` starts a program that writes each of its arguments to a file, with the argv elements `x;`, `y\;`, `;`, `rename-session`, `pwned`, `#{session_name}`. The file holds exactly those elements, and `list-sessions -F '#{session_name}'` still prints `demo-c1r1`.
   - `ensure_session` is called with `<dir>/p#S` and with `<dir>/#(touch <dir>/ran)`, both made with `create_dir_all`. Each session's `#{session_path}` equals its cwd exactly, and `<dir>/ran` is never created within a bounded wait.
5. **Extend Decision 1 and AC 6 (B-3).**
   - Builder method `with_kill_binary(PathBuf)`, default `kill` from `PATH`, used for every signal the adapter sends.
   - The rule: "no default-run test lets the adapter signal a pid the test did not spawn. A test that can reach a signal uses a fake `kill` that only records its arguments."
   - Suggested AC 6h. It runs against a fake tmux that answers `list-panes` call by call (for example from a numbered queue of answers) and a fake `kill`:
     - Only a pane whose `@holler-pid` equals its `pane_pid` is signalled. The listing also has an untagged pane and a pane whose tag does not match its pid, and the kill record holds only `-s TERM -- -<tagged pid>`.
     - A pane still listed after the grace gets `-s KILL -- -<pid>`.
     - `kill` answering `No such process` counts as gone (`Ok`).

The warns reach T and F through this handoff. W-2, W-3, W-4 and W-8 are code or test choices inside the plan. W-1, W-5, W-6 and W-7 are entries for O in decisions.md. W-9 applies to the PR and the commits.

## Patterns referenced

- `crates/holler-pane/src/ports.rs:150-168` (the port), `argv.rs:7-12, 96-100` (no-echo refusals), `error.rs:392-467` (variants, "none carries a secret")
- `crates/holler-pane-testkit/src/host.rs:35-61, 169-219` (`HostOp` names and the fake's rules), `conformance/host.rs:1-96` (the suite and how an adapter runs it)
- `crates/holler-hub/tests/logging_guard_test.rs:20-70`, `crates/holler-cli/tests/hold_single_path_test.rs:15-40` (the source-guard pattern), `crates/holler-hub/tests/token_store_test.rs:616-625` (the re-exec child pattern)
- `docs/adr/ADR-0021.md` §2 (ports) and §5 (crate rules); epic #633 (B2, "no new `unsafe`", "no story touches a live fleet"); issues #641, #644, #646, #663
- `scripts/lint.sh` (allow links, 900-line gate), `docs/agent-overlays/tester.md` (Tier 1, RED rule)
