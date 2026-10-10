# Handoff-A: Phase 3 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-642-implementation at 6c3809c (base `dc300ab`; `origin/main` has since moved to `e327569`, #641 merged)
**Brief reviewed:** docs/handoffs/642-brief.md at 6c3809c (sha256 `c2b09a81b3191e37...`)   **Reuse map:** the brief's "Reuse map (extend the 642a objects; do not duplicate)" (lines 1121-1139)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with 0 blocks and 9 warns. The plan extends the right objects in the right layers:

- The three method bodies go in a private `attach.rs`, and `lib.rs` delegates to it as it delegates `serve` to `server.rs`.
- The pure tmux builders and parsers go in `tui.rs`, and the HTTP steps reuse `Call` and `known`.
- It adds no dependency, error code or signature, and it updates ADR-0021 in the same change.

The most consequential warn is W-1, which I probed on tmux 3.7c. `=<session>:` names the session's current window. So once
another window is current, `attach_tui` kills a program that #641 started, or one a person opened. W-2 (one deadline for
`select_session`) and W-3 (the ADR note credits #641 with a server stop it does not have) come next.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn | Every tmux target: `exact_target(session)` = `=<session>:` (API; Decision 9; AC 11a, "every builder's `-t` value is `=demo:`"). Also the ADR note's fact 1 ("addressed by its exact name"). | contract shape across adapters (#641, #644); cross-cutting (what the adapter may kill) | `=<session>:` is not one pane. tmux resolves it to the session's **current window** and that window's active pane, and the TUI's session holds other panes. #644's decision 6 runs the spec's `command` there through #641's `run` (`new-window -d`), and a person can open or split windows. My probe (tmux 3.7c, a private `-L` server, killed afterwards): **(1)** After a detached `new-window`, `=demo-c1r1:` still resolved to window 0. **(2)** After `select-window -t =demo-c1r1:1`, it resolved to window 1 (`env -- sleep 3601`, the shape of a `run`). **(3)** In that state, `set-option -p -t =demo-c1r1: remain-on-exit on` landed on window 1's pane, overriding #641's window-level `remain-on-exit off` (its W-8). **(4)** `respawn-pane -k -t =demo-c1r1:` then killed window 1's command. **(5)** `=demo-c1r1:^` resolved to window 0 throughout. `shown_session` and `select_session` fail closed on such a pane (no attach port, no id title). Reconcile's only repair is `select_session` (`reconcile.rs:19-22`), so those two are safe. `attach_tui` is not. A `launch` into a kept session ("the next launch of that name reuses it", #641's crate docs), or a `relaunch`, while a window or split a person opened is current, would SIGHUP that program. #641's docs say such a program is never signalled ("anything a person opened, is never signalled"). Neither this brief, #641's docs nor #644's brief says which pane of the session is the TUI's. | Decide the rule and enforce it in one place. **(a)**, the cheaper: target the session's first window, `=<session>:^`, in `exact_target`. Probed: `respawn-pane -k -t '=demo-c1r1:^'` exits 0 and replaces window 0 while window 1 is current. Only the string that AC 11a and 27-30 pin changes. Residual case: window 0 itself was closed. **(b)**: keep the target, and have `attach_tui` refuse (`unavailable`, naming the pane) a live pane whose `#{pane_start_command}` is neither empty nor an `opencode attach`. That costs one query before `set-option` in AC 30(a), and the rig's `sleep 3600` placeholders would become plain shells. A session made with no command reports an empty start command (probed). (a) and (b) combine. State the rule in `tui.rs`, in the crate docs and in fact 1. If O defers it, file it before #644's relaunch first runs against real tmux. |
| W-2 | warn | Added by 642b: "`select_session` builds its `Call` with the port `attach_port` returned". The Reuse map: "`OpenCodeHarness::call` ... unchanged ... for every HTTP step". Both read against Behaviour, General: "Every method takes a deadline of `now + timeouts.call` at entry". | cross-cutting concerns (I5: one deadline per call) | `OpenCodeHarness::call(port, op)` (`lib.rs:180-189`) starts its deadline when it is called, and it binds the port. `select_session` learns its port only from the tmux query. So calling `call` after the query restarts the clock. The HTTP steps then get a fresh `timeouts.call` after a query that may have used most of one: up to about twice I5's bound. No AC can see this. The fixture cannot be slow (AC 33 allows no `sleep`), and AC 29(d) measures `settle`. Every merged sibling takes its deadline once, at entry: 642a's methods through `call`, and #641's through `Call::start` (`crates/holler-adapter-host/src/lib.rs`). | Name the one extension in the Reuse map: a crate-root constructor that takes the deadline, which `call` itself delegates to (for example `OpenCodeHarness::call_until(port, op, deadline)`). `select_session` takes its deadline before the resolver, runs the query under it with its own `op`, and then builds its `Call` from it. A-dup accepts that constructor, or a `Call` built in `attach.rs` from the entry deadline. It rejects `call` invoked after the query. |
| W-3 | warn | The ADR-0021 note "`HarnessPort` as built (#642)", fact 2, carried from 642a: "`HarnessPort` has no stop; the host adapter (#641) stops the server by the recorded pid, its process group and descendants". | ADRs (the record of what was built) | #641 merged as `e327569` at 20:06 MDT on 2026-10-09. Its `stop_owned` stops only the panes its own `run` started and tagged ("Narrowings", item 1, in `crates/holler-adapter-host/src/lib.rs`). It never stops the harness server, which `serve` starts outside tmux. #695 ("stop the harness server that serve started") is open. 642a's crate docs already cite #695 for this job (`lib.rs:45-47`). As drafted, ADR-0021 would credit merged code with a capability it does not have. | Fact 2 ends: "`HarnessPort` has no stop. Stopping the server `serve` started, by its pid and process group, is #695's, which is open. The host adapter as merged (#641) stops only what its `run` started." Add "Stopping the harness server that `serve` started: #695" to "Deferred to named stories", beside the agent item. S reads this as AC 26 met: the fact's meaning stands, and only its attribution changes. |
| W-4 | warn | `exec.rs` gains "`classify(stderr) -> Missing \| Other(first line)`" (the API amendments; Reuse map, "Extend"). | layering; pattern consistency with #641 | Merged #641 keeps its runner free of tmux knowledge. Its `exec.rs` holds `run`, the drains and `kill_group`. What tmux's answers mean lives in its tmux module: `classify`, `Refusal` and the parsers are in `tmux.rs`. This brief puts the tmux stderr rule in the generic runner, beside `kill_group`. Every other reader of tmux's output (`parse_query`, `parse_title`, `attach_port`) goes in `tui.rs`. #696 will replace the adapters' private runners with one runner in `holler-pane`. With `classify` in `exec.rs`, that change would also have to move tmux knowledge. | Put `classify` and its two-way result in `tui.rs`, beside the other parsers. They can be `pub` like the rest, so `tui_test.rs` can pin the stderr table without the fixture, or `pub(crate)`. `exec.rs` returns status, stdout and stderr, and knows nothing of tmux. Reword `exec.rs:5-6` to match; F rewrites that header anyway (AC 26). No AC calls `classify` directly, so T's plan is unaffected. |
| W-5 | warn | Added by 642b: "How this is factored (a sibling of `known` that returns the parsed reply and that `known` itself calls, **or the same rules inline**) is F's choice ...; A-dup checks that the rules are not written twice." | pattern consistency (anti-duplication) | The second option is the duplication that the same sentence says A-dup rejects. Taken, it would restate `known`'s rules in `attach_tui`: a 400 or a 404 is `session-not-found`, and an id mismatch is `unavailable`. Offering it invites a Phase 7 rework. | Strike "or the same rules inline". The plan is one sibling that `known` itself calls, for example `session_reply(call, path, id) -> Result<Value, PaneError>`. `known`'s signature, and its answers to `abort` and `select_session`, stay as they are. `attach_tui` reads `directory` from the object the sibling returns. |
| W-6 | warn | The two new observe-until-settled polls: `attach_tui` until the title shows the id, and `select_session` the same, both "within `settle`". | pattern consistency (anti-duplication within the crate) | The crate already has one such loop: `settled` (`lib.rs:389-406`). It polls every `SETTLE_POLL` (100 ms) until `deadline_after(settle).min(call.deadline)`, sleeps only while time is left, and ends in `call.timeout()`. The Reuse map names neither the loop nor the constant, so each TUI poll could get its own interval and loop. The spike's switch takes about 110 ms, which fits a 100 ms cadence. | Add to the Reuse map one private poll loop in `attach.rs`, shared by `attach_tui` and `select_session`. It reuses `SETTLE_POLL`, with its doc widened from `abort` to "a switch, an attach or an abort" to match `Timeouts::settle`. It also reuses `settled`'s bound and sleep rule. A-dup checks that there is no second interval constant and no second loop. |
| W-7 | warn | The Dependencies table ("#641 ... OPEN, `CONFLICTING`"). E8 ("unmerged: PR #706 ... at `f2ea297`"). Reuse map, "New, justified": "the fake tmux fixture (no tmux double exists in the repo, ...)". AC 22's `git diff --name-only origin/main`. | evidence baseline; pattern consistency (test doubles) | #641 merged as `e327569` at 20:06 MDT, one minute after this brief's last commit (`6c3809c`, 20:05 MDT). **(a)** The mirrored code did not change: `git diff f2ea297 e327569 -- crates/holler-adapter-host` is empty, so the mirror now points at `main`. **(b)** A tmux double is now on `main`: #641's `Fake` (`crates/holler-adapter-host/tests/common/mod.rs`). It cannot be reused. It builds `holler_adapter_host::TmuxHost`, so including it means an adapter-to-adapter dependency (ADR-0021 §5; `tui.rs:11-14`). It also writes its scripts at test time and retries on `ETXTBSY`, which AC 32 forbids here. So the committed fixture stays justified, but on these grounds, not on "no double exists". The two fakes record a call differently: #641 writes one argument per line and then a `-=END=-` line, and this one ends each argument with U+001F. Whoever merges the two has to pick one. **(c)** The worktree is on `dc300ab`. On an unrebased branch, AC 22's command also lists #641's files. `CHANGELOG.md` then conflicts at the end of `[Unreleased]` Enhancements, where #641's entry follows the part-1 entry (lines 218-232 on `main`). AC 23's part-1 lines (214-216) have not moved. | Amend the dependency row and the fixture's justification as in (b). Rebase onto `origin/main` before the diff gate; until then, S reads AC 22 against the merge base. Record the two record formats on #696. 642a A-dup's D-2 runner differences are also still unrecorded there: #696 has no comments. |
| W-8 | warn | E8: "`tui.rs` mirrors by the same names". Reuse map: "`escape_arg`/`escape_dir`/`classify` follow #641's `escape`/`escape_cwd`/`classify` ... by rule and name". The new public items of `tui`. | naming; public surface | **(a)** Only `classify` shares a name. #641's escapes are `escape` and `escape_cwd`, with the same rules, and its colon target is the private `window_target`. **(b)** `tui`'s builders and parsers become public API. That matches the Herdr adapter's public pure modules (`holler-adapter-herdr/src/lib.rs:37-41`) and 642a's `http`, which is public for tests. But `http` carries the rule "public for this crate's tests and is not an interface" (`lib.rs:64-65`, `http.rs:4-7`). The brief adds no such sentence for `tui`, whose builders write the tmux syntax that Decision 1 says `holler-cli` never writes. | **(a)** Before T-red, because T's tests call these names: name the two escapes `escape` and `escape_cwd`, as #641 does. If the brief is not amended, drop "by name" from E8 and the map, and list the name mapping on #696. `exact_target` may keep its name, since its uses are pane targets, with a doc line naming #641's `window_target`. **(b)** `tui.rs`'s module doc says its builders and parsers are public for this crate's tests and are not an interface. Wiring (#649) uses only `TmuxConfig` and `TmuxSocket`. F rewrites that header anyway (AC 26). |
| W-9 | warn | `tui_argv` under `ProcessEnv::Inherit` (`env -u OPENCODE_DISABLE_TERMINAL_TITLE <opencode_bin> attach ...`). `ProcessEnv::Inherit`: "The child inherits the environment the adapter runs in" (`lib.rs:110-111`). `OpenCodeConfig.env`: "(and, in part 2, of the TUI's `attach`)" (`lib.rs:156`). `opencode_bin`: "found on `PATH`" (`lib.rs:152`). | contract shape (forward-compat with #649); cross-cutting (environment, ADR-0021 §13) | tmux starts the TUI, not the adapter. My probe (tmux 3.7c, a private server): I started the server with `HLR_PROBE=from-the-server`, then ran `respawn-pane` from a client with `HLR_PROBE=from-the-adapter`. The pane saw `from-the-server`. So under `Inherit` the TUI gets the tmux server's environment, not the one these docs describe. A bare `opencode_bin` is looked up on the adapter's `PATH` for `serve` but on the pane's `PATH` for `attach`. So a server and its TUI can run two different OpenCode binaries. Under `Isolated` the `env -i` list governs both, so the rig is unaffected. | Docs only in this run. `ProcessEnv::Inherit` and `OpenCodeConfig.env` say the TUI gets the tmux server's environment, with `OPENCODE_DISABLE_TERMINAL_TITLE` removed. `opencode_bin` says wiring passes an absolute path, so the server and its TUI run one binary. Give the same rule one sentence in the crate docs, or in the ADR note's fact 6, and add it to the brief's #649 Forward-compat row. |

### Checked and consistent with existing patterns (no finding)

- **Homes and layering:**
  - `attach.rs` is private and takes the three bodies; `lib.rs` delegates in one line each, as `serve` delegates to
    `server.rs` (`lib.rs:193-195`). This is 642a's A, N-2.
  - The pure builders and parsers in `tui.rs` follow #641's `tmux.rs` and the Herdr adapter's `layout`, `plan` and
    `protocol`.
  - Child modules reach the crate root's private helpers, as `server.rs` already does.
- **Size:**
  - Estimates: `lib.rs` 483 to about 500, `tui.rs` about 260, `attach.rs` about 300, `exec.rs` about 190. No file nears
    900.
  - Flagged: `hermetic_test.rs` (775) crosses about 800 with AC 31's rewrite. The plan names the homes of the new tests
    (`tui_test.rs`, `attach_test.rs`), puts the retry helper in `stub.rs` and caps the file at 850. No action beyond
    keeping the helper's body out of it.
- **Dependencies and the shared runner:**
  - No new runtime dependency and no adapter-to-adapter dependency.
  - `tempfile` arrives only with its first caller, as in #641's manifest.
  - Nothing depends on the crate (I checked every `Cargo.toml`).
  - `run_probe` in `holler-pane` is still #663's stub (`probe.rs:28-36`). A private capturing runner is therefore the
    interim path #696 sanctions, not a parallel path.
- **Closed codes:** no new code or variant (Decision 22).
- **Invariants and messages:**
  - The TUI's port is observed from `#{pane_start_command}`, never remembered (I6), and no keystroke is sent (I4).
  - Messages never echo an argv element, a directory, an env value or a raw title, as #641's never echo an argv element
    or a directory.
  - `tmux_command` removes `TMUX` and `TMUX_PANE` and uses a null stdin, as #641's `run_tmux` does.
- **Test conventions:**
  - `tests/fixtures/` is the repo's dominant spelling (10 files in holler-cli, 5 in holler-proto; one file in
    `tests/fixture/`).
  - `tests/real_opencode/` matches the Herdr adapter's per-purpose `tests/wire_herdr/`.
  - `#[allow(dead_code)] // #642` passes lint check 1.
  - The opt-in gate (`#[ignore]` plus `HOLLER_TEST_OPENCODE=1`, and a failure when opted in with a binary missing) is
    stricter than #641's (`#[ignore]`, and a skip when tmux is missing). It uses the repo's `HOLLER_TEST_*` naming. That
    is reasonable for a harness that could reach a model.
  - The real rig mirrors #641's: a socket path under 100 bytes, and `kill-server` by socket, never a signal.
- **ADR and `holler-pane` edits:**
  - Required by ADR-0021 §2 (lines 100-103) and its "Deferred" item, which name #642. 642a's A ruled them in (B-1).
  - AC 22 confines `holler-pane` to `//!` and `///` lines.
  - Risk 8 covers #640 part 3's edits of the same sentences.
- **642c placement:** the agent stays out of this run (Decision 16). 642c's planned shape, a resolver beside `workdir`
  applied by `serve`, is 642a A's N-1(c).

## Notes for O

Not required for a PASS. Each warn has a point by which it must be settled:

1. **Before T-red**, because they change what T pins: W-1 (the target string in AC 11a and 27-30) and W-8(a) (the escape
   names T's tests call). If the brief is not amended, both become follow-ups. W-1's must land before #644's relaunch runs
   against real tmux.
2. **In this run, for F.** The brief already allows these, and A-dup checks them: W-2 (the deadline-taking constructor),
   W-4 (`classify` in `tui.rs`), W-5 (the sibling of `known`), W-6 (one poll loop) and W-8(b) and W-9 (doc sentences).
3. **For F and S on AC 26:** W-3 corrects the attribution in the carried fact 2 and adds one "Deferred" item.
4. **For O:** W-7, meaning the dependency row, the fixture's justification, the rebase before the diff gate, and the #696
   note (the two fake record formats plus 642a A-dup's D-2).

## Patterns referenced

- `crates/holler-adapter-opencode/src/{lib,exec,tui,server,http}.rs` and `tests/{hermetic_test.rs,support/stub.rs}` at
  `dc300ab` (642a as merged).
- `crates/holler-adapter-host/src/{lib,exec,tmux}.rs` and `tests/{common/mod.rs,fake_tmux_test.rs,real_tmux_test.rs}` at
  `e327569` (#641 as merged); `crates/holler-adapter-herdr/src/lib.rs`.
- `crates/holler-pane/src/{ports,lib,probe,reconcile}.rs`; `crates/holler-pane-testkit/src/{harness,fault}.rs` and
  `src/conformance/harness.rs`; `docs/adr/ADR-0021.md` §2, §4 and §5.
- #644's brief at `7195993` (steps A2-A7, decision 6). 642a's `handoff-A.md` and `handoff-A-dup.md` at `d6393b2`. Issues
  #695, #696 (no comments), #700, #649 and #663.
- My probes, on tmux 3.7c, each on a private `-L` server killed afterwards: window targeting, `^` and `respawn-pane -k`
  (W-1), and the environment of a respawned pane (W-9).
