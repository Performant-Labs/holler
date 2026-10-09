# Handoff-A: Phase 3 - #642a the OpenCode adapter, server side (`serve`, `health`, `create_session`, `list_sessions`, `abort` over OpenCode's HTTP API)  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-642-implementation (at 5a4d68e; `origin/main` 3bdd129 merged in by 1c4a38c)
**Brief reviewed:** docs/handoffs/642-brief.md at 5a4d68e (this run is 642a only; the brief also holds 642b's plan)   **Reuse map:** docs/handoffs/642-brief.md, "Reuse map (extend, do not duplicate)" (lines 935-968)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This re-review replaces the BLOCK at 4831580 (2 blocks, 9 warns), which reviewed the brief at 53237d4.

## Summary

PASS: no finding is a block, and both earlier blocks are resolved. 642a reuses the frozen `HarnessPort`, the closed codes and
`FakeHarness`'s `op` strings. It answers the three TUI methods with `PaneError::NotImplemented`, documented as the skeleton's
answer for "a verb or method whose story has not landed" (error.rs:404-406), as `Unwired` does. It depends only on
`holler-pane`, `serde_json` and `httparse`, and it justifies every new object in writing.

- **B-1 (the ADR edit)** moves to 642b, the closing part. That matches #640's split: its part 1 merged with no ADR edit, and
  part 3 carries the ADR-0021 rows (640-brief.md:22).
- **B-2 (exact tmux targets)** is 642b's code, pinned by AC 11a-11c and 19a.

Eight warns remain. The most consequential is W-1: this brief and #644's give #649 opposite instructions on the resolver
precondition, and `tui_session` (642b) looks impossible to meet during launch as the ports stand.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn | Decision 14 (brief:1036-1039) and the #644 Forward-compat row (brief:974): "#644 passes the session name (and directory) explicitly from its plan". The `Resolver` doc (brief:506-509). | contract shape (forward-compat); the seam between two in-flight plans | #644's brief (at 7195993) rejects this reading. Its C-9: "The frozen `HarnessPort` methods take only `(PaneName, port)` or `(PaneId, ...)`: there is no channel to pass them. #644 calls the ports as frozen; #649 must make its resolvers answer before the record exists" (644-brief.md:100-103). Its #649 row says the same: "from the `PaneName` and `PaneId` the engine passes" (644-brief.md:2105). So #649 gets opposite instructions from the two briefs. This brief's own `Resolver` doc ("Meeting it is wiring's job (#649)") already agrees with #644; decision 14 and the row do not. **642a (`workdir`):** only `workdir` is live in 642a, and wiring can meet it alone. #644 runs `host.ensure_session(&name, &spec.host.cwd)` (A2) before `harness.serve(&name, port)` (A4) (644-brief.md:1614-1616). A wiring-level `HostPort` wrapper that records the `cwd` it forwarded therefore answers `workdir(name)`, with no verb change and no tmux call outside an adapter. **642b (`tui_session`):** `tui_session(PaneId)` at A7 and O1 (644-brief.md:1619-1620) has no such source. No port call links a Herdr pane to its tmux session (#644 C-8, 644-brief.md:96-99), and `HerdrSpec` carries no `PaneName`. | Rewrite decision 14 and the #644 row to match #644: #644 passes nothing beyond the port arguments, and the precondition is #649's alone. Add the `workdir` route above as the worked example. Before 642b's F phase, say how `tui_session` is met during launch, or make C-8's contract follow-up a named dependency of #649. One in-crate option for 642b: within one process, `serve(name, port)` has already told the adapter which pane name owns the port. `attach_tui(pane, port, ...)` can map `pane` to that name and keep the mapping for the `shown_session` that follows, while SHOWN itself stays observed from tmux (I6). The public API does not change either way. |
| W-2 | warn | `OpenCodeConfig.env: ProcessEnv`, adapter-wide and "final" in 642a (split table, brief:69; API, brief:511-518 and 534). The #644 Forward-compat row (brief:974), which says "yes". | contract shape (forward-compat) | #644 hands this crate a job that this brief never mentions. Its decision 7: "Model, effort, env names and ceilings are recorded, not applied ... applying them to the OpenCode process is a follow-up for #642/#649 (for example the adapter's serve environment from the record)" (644-brief.md:1988-1990; follow-up at 2127; item (c) of the #649 row at 2105). Nothing in 642a has to change. ADR-0021 §13 says env values "stay in the operator's own environment" (ADR-0021.md:482), and `ProcessEnv::Inherit` meets that. Neither ADR-0021 nor the epic says `Pane.env` restricts what a harness sees. But "the config is final" and "#644: yes" leave #649 with no named owner for the job. | Add one sentence to Forward-compat and to decision 10. As built, the adapter applies no per-pane env names, model or effort, and under `Inherit` the server and TUI see the operator's environment (ADR-0021 §13). If #649 or a follow-up wants per-pane application, the shape consistent with this crate is another per-pane `Resolver<PaneName, _>` beside `workdir`. Add it with its first caller, not in 642a, where it would have none. Answer #644's follow-up line so both briefs name the same owner. |
| W-3 | warn | The split: decisions 2, 13, 14 (the `workdir` half) and 15(a) ship in 642a's code, while AC 25 (the PR-body record) and AC 26 (the ADR note) are 642b's (brief:69-74, 714-735). | ADRs; the documented contract during the split | Not a block. Deferring the ADR-0021 edit to the closing part matches #640's split (part 3 carries "the ADR-0021 §9/§10 rows", 640-brief.md:22). ADR-0021.md:533 ("`HerdrPort` and `HarnessPort` in their final form: #636 and #635, then #640 and #642") also stays true while #642 is open. Between the 642a and 642b merges, though, main runs a `serve` that never adopts a server, while the fake re-serves its own pane (harness.rs:115-117). Main also lists top-level sessions only and relies on a `workdir` precondition. No 642a AC requires the crate docs to state any of this, and no AC covers the 642a PR body. | Add to the 642a list: (1) `lib.rs`'s crate docs state decisions 2, 13, 14 (for `workdir`) and 15(a), and that `attach_tui`, `select_session` and `shown_session` answer `not-implemented` until 642b; (2) the 642a PR body lists divergence 15(a), cites #695, and says the ADR-0021 note lands with 642b. S checks both. |
| W-4 | warn | Decision 2, "a follow-up issue for it (no number yet)" (brief:997-998). Decision 11, the Reuse map's `exec.rs` row and the third follow-up (brief:960-963, 1065-1066). Decision 15's test-kit follow-up (brief:1063-1064). The two `kill` spellings (brief:615, 656). | duplication (follow-up tracking); pattern consistency | (a) The stop owner is **#695**, which is open ("stop the harness server that serve started"); #644 already cites it (644-brief.md:2112). (b) The consolidation is **#696**, also open ("one bounded subprocess runner shared by the adapters and run_probe"); #663 points to it too (663-brief.md:1528, 1664). Filing the brief's third follow-up would duplicate #696. (c) No `FakeHarness` parity issue exists (I searched open and closed issues), yet divergence 15(a) lands in 642a. (d) Behaviour writes `kill -KILL -- -<pgid>` and the `exec.rs` paragraph writes `kill -s KILL -- -<pgid>`. #641 and #663 use `kill -s <SIG> -- -<pgid>` through the `kill` binary (663-brief.md:1328-1336). (e) Stopping "by the recorded pid" has no rule that the pid must still be that server (pid reuse). #641 checks the same for its own processes: `@holler-pid` must equal the live `#{pane_pid}` (641-brief.md:513-515). | Cite #695 in decision 2, Out of scope and Follow-ups. Cite #696 in decision 11, the Reuse map and Follow-ups, and drop the third follow-up. File the `FakeHarness` parity follow-up before 642a's PR. Use one form, `kill -s KILL -- -<pgid>`, so #696 stays a move. Add the pid-reuse check to #695 as a comment; nothing changes in #642. |
| W-5 | warn | AC 23 and AC 24 as they read in 642a's list (brief:728-729, 870-875). | spec coherence across the split | AC 23 is in 642a's list in full. It requires the CHANGELOG entry to say "the real-OpenCode tests are opt-in", but 642a adds none. The precedent for a part-1 entry is #640's: "No I/O yet: the socket adapter follows in part 2, so nothing a user runs changes" (CHANGELOG.md, #699). AC 24, extended by "the rig clause applies to whatever test code this run adds", says no test reads "a port outside 48100-48199". The hermetic stub binds `127.0.0.1:0` (brief:738), an OS-assigned port. AC 1 and AC 6 need a refused port, which the repo's convention also takes from `127.0.0.1:0`, bound and then dropped (attach_cli_test/fake_server.rs:93-102). Read literally, S would fail 642a's stub. | **AC 23:** 642a's entry says this is part 1 (the server side), that the TUI half follows, and that nothing a user runs changes yet (#649 wires the adapter). 642b's entry adds the opt-in clause. **AC 24:** scope the port rule to tests that start real OpenCode (642b's rig). Hermetic tests use OS-assigned loopback ports only. |
| W-6 | warn | 642a's `exec.rs`, "runner and `kill` only" (brief:69), against the full runner spec (brief:610-616). The manifest's dev-dependencies (brief:931-933). | pattern consistency (the workspace's lint and dependency rules) | The workspace denies `dead_code` ("forbids a helper landing without a caller", Cargo.toml:13-18 and 30), and CI fails on unused dependencies (ci.yml:314-316). In 642a, `exec.rs` has one caller: the `kill` on `serve`'s deadline. The runner as specified drains and returns stdout and stderr's first line, and only 642b's tmux calls read those. The manifest lists `tempfile`, which only the rig uses, and the rig is 642b's file. Unread fields fail the build or invite an `#[allow(dead_code)]`, which the workspace uses only for justified forward declarations (holler-hub/src/serve.rs:64). An unused `tempfile` breaks "declare only what is consumed". | In 642a, `exec.rs` carries only what the kill path reads. 642b adds stdout capture and the tmux stderr classification together with their callers. 642a declares only the dev-dependencies that `hermetic_test.rs` uses (`holler-pane-testkit` for the `HarnessOp` pin; `tempfile` only if it is used), and 642b adds the rest. |
| W-7 | warn | The Reuse map's reason for the stub (brief:964-966): the existing fake OpenCode servers are "async on `tokio`". | duplication (test helpers); accuracy of the Reuse map | This is half true. `holler-cli/tests/attach_cli_test/fake_server.rs` is synchronous `std::net` with `std::thread`s (lines 14-20). It has per-route replies (`set_bare`, `set_api`), a request record (`requests()`), `Connection: close`, and `unreachable_endpoint()` (lines 93-102). Its header states the same rule this stub relies on: a file under another crate's `tests/` cannot be imported (lines 7-17). The new stub is still justified, because it cannot be imported and it needs a frozen mode and the raw request line. But it is a near-cousin of that file, not of the tokio one. | Correct the Reuse map line. T models the stub on `attach_cli_test/fake_server.rs`: the same conventions, `Connection: close`, and bind-then-drop for a refused port. Its module doc names that file as its cousin, the way that file names `holler-body`'s. A-dup checks the stub against it. |
| W-8 | warn | `pub mod http` with `pub fn request(...)`, "public so the tests reuse it" (brief:502, 546-557). | layering (the public surface of an adapter) | This makes a raw OpenCode client part of the adapter's API, without the port's per-call deadline, reply-shape checks or error mapping. #640 part 1 publishes only pure modules (holler-adapter-herdr/src/lib.rs:24-30), and the epic reaches OpenCode only through the port (I1; ADR-0021 §5). `holler-cli` is the crate's only consumer, so the exposure is small. | The crate docs and `http`'s module doc say `http` is the adapter's own transport, public for this crate's tests (and 642b's raw pins), and that code outside the crate reaches OpenCode only through `HarnessPort`. Optionally mark it `#[doc(hidden)]`. |

### Checked and consistent with existing patterns (no finding)

- **Object choice and dependencies.** `HarnessPort`, `PaneError`, the closed codes, `run_harness_conformance` and `HarnessRig`
  are reused unchanged. The crate depends only on `holler-pane`, `serde_json` and `httparse`, and on no other adapter, the hub
  or the body (ADR-0021 §5). The `op` strings are this crate's own constants, pinned equal to `HarnessOp::as_str` through a
  dev-dependency, as #640 part 1 pins `GRID_UNREACHABLE` and `SUPPORTED_VERSIONS` (holler-adapter-herdr/Cargo.toml).
- **The partial implementation.** In 642a, `NotImplemented` from three methods fits the skeleton convention. Nothing can
  wire the adapter before #642 closes (#649 depends on it), and the PR says `Part of #642`.
- **Server behaviour against the fake.**
  - `health` is never `Err`; the fake answers `Ok(state == Running)` (harness.rs:287-290).
  - A frozen port answers `timeout` with the method's `op`, and an unbound port is `unavailable`.
  - `abort` checks `GET /session/<id>` first, as the suite's binding rules require (conformance/harness.rs:12-21).
  - The only intended divergence is 15(a).
- **Reply shapes and top-level sessions.** The 200-HTML guard (AC 11d) and the `parentID` filter (AC 11e) are in 642a's
  list, so `abort` and `list_sessions` land with them.
- **The HTTP client.** The hand-rolled blocking client over the workspace's `httparse` is still justified, unchanged since the
  first review. The async `reqwest` client lives in `holler-body`, and turning on `blocking` would unify features across
  crates.
- **The `serve` process model.** The server gets a process group of its own with no `unsafe`, its pgid is the returned pid,
  and it outlives the CLI. `Child` ownership is stated, and only health polls are sent before the first healthy answer (spike
  Recommendation 1).
- **The mirror of #641.** `TmuxSocket { Default, Name, Path }` still matches #641's current brief (641-brief.md:480-481 at
  95e2260). Because the two are distinct types, #649 maps one to the other, variant for variant.
- **I5's bound.** `holler-pane` states "default 10 s" only in doc comments; there is no constant to reuse, so
  `Timeouts::default().call` is a value, not a parallel definition.
- **Evidence drift.** The brief cites 9d61c9f, and the branch now includes 3bdd129. `git diff 9d61c9f 3bdd129` is empty for
  `holler-pane`, the harness fake and its suite, and this crate.
- **Public repository.** I found no personal infrastructure names in the brief or in this run's tracked handoff files.

## Notes for O

Not required for a PASS. For your follow-up decisions:

1. The MO must decide W-1 before 642b's F phase. W-4 (c) needs an issue filed before 642a's PR.
2. Out of scope, but noticed: the brief names `deepseek-v4-pro` as the outside model (brief:5), and the usage files of all
   three review rounds show it. The repo's `CLAUDE.md` names `glm-5.3-flash`. If the operator chose `deepseek-v4-pro` for
   #642, nothing is needed. Otherwise, confirm which model's gate counts for this run's journal.

## Patterns referenced

- `crates/holler-pane-testkit/src/harness.rs` (`FakeHarness`) and `crates/holler-pane-testkit/src/conformance/harness.rs`
  (the suite's binding rules).
- `crates/holler-adapter-herdr/{Cargo.toml,src/lib.rs}` and its `CHANGELOG.md` entry (#640 part 1, PR #699), plus
  `.claude/worktrees/0640-herdr-adapter/docs/handoffs/640-brief.md:22` (the ADR edits in part 3).
- `crates/holler-cli/tests/attach_cli_test/fake_server.rs` (the synchronous fake OpenCode server and the cross-crate-copy
  rule).
- `.claude/worktrees/0644-launch-relaunch/docs/handoffs/644-brief.md` at 7195993 (C-8, C-9, decision 7, the act order, the
  #649 row), `0641-host-adapter/.../641-brief.md` at 95e2260, `0663-profile-scope-probe/.../663-brief.md` at ec3a214, and
  issues #695 and #696.
- `Cargo.toml` (the workspace lints, `dead_code = "deny"`), `.github/workflows/ci.yml` (`cargo machete`), and
  `docs/adr/ADR-0021.md` (§5, §13 line 482, line 533).
