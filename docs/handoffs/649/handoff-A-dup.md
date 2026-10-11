# Handoff-A-dup: Phase 8 - #649 hub wiring and the scratch integration scenario  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`)
**Diff base:** `5d366b9` (the merge base with `origin/main`)   **Diff head:** `c13400a`
**Reuse map:** no separate file. As in every A pass of this run, the brief's D11, D24, D31 and D32, with Evidence E1 to E70 (`docs/handoffs/649-brief.md`).
**Verdict:** PASS

`origin/main` has moved to `4366fb8` (#830's research spike). It touches only `docs/research/` and `scripts/spikes/`, and no file of this diff.

The head commit `c13400a` changes one line of the brief. `git diff 4d7f58a c13400a -- crates/` is empty, so the tree under review is the one T-green verified.

## Summary

**PASS. F extended the objects the map named, and built no parallel path.**

- **Production.** #670's `Wiring` is filled in place. Its `connect`/`ports` signatures are kept, and `Unwired` stays.
  - Every port is built from a reused piece: `StoreScope`, `SystemProber`, `transport::call(None, ..)`, `holler_pane::reply`, and the adapters' own constructors.
  - The new objects are D24's names, plus two. `Wired<P>` generalises D24's `WiredHarness` (reviewed at A's fifth pass), and `Lookup` is an alias.
  - For the store clients, the environment readers, the bindings and the resolver, I searched for an analogue to extend and found none.
- **Tests.** D11's three rigs are included by `#[path]` and generalised in place.
  - `support`'s `Hub`, `StateDir` (extended by `StateDir::at`) and `wait_for` are used and never re-declared.
  - There are still three `LinkedHost` copies, and no fourth.
- **The rework cycles added no drift.**
  - F's second cycle changed `connect`'s lookup closure and two private messages; its third added a comment.
  - T's cycles changed tests only.

**Four low warns, none binding on this run.** Three are small test-helper copies. One notes that the client's watch iterator has two private twins in the hub.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-1 | warn (low, T; not binding) | `crates/holler-cli/tests/pane_integration/scratch/mod.rs:84-91` | **`alive(pid)` is a third copy.** It runs `kill -s 0 -- <pid>`, as `holler-adapter-host/tests/real_tmux_test.rs:53-60` and `tests/common/mod.rs:221-230` do; those two also set `LC_ALL=C`.<br>`real_tmux/mod.rs`, which this story made includable, would be its shared home. But D11 limits edits to the adapter test directories to the named generalisations. So the copy follows from the brief's own boundary; it is not drift. | Move `alive` into `real_tmux/mod.rs` when the `LinkedHost` consolidation is done (Notes for O). `real_tmux_test.rs` and the scenario then share one. |
| W-2 | warn (low, T; not binding) | `crates/holler-adapter-host/tests/real_tmux/mod.rs:129-136` | **`tmux_on_path` repeats the rig's `on_path`** (`real_opencode/rig.rs:95-101`), which D32 lists among what must not be copied.<ul><li>It serves only `isolated`, which only #649 calls (`real_tmux_test.rs:14-16`).</li><li>`Scratch::start` already holds `tmux_bin` from `rig::binaries()` (`scratch/mod.rs:207`).</li></ul>Not a block: it was in the blessed trial `046026f`, and the host adapter's tests cannot reach the OpenCode rig. So it is a crate-boundary helper, not a parallel path. | Give `isolated` a `tmux: &Path` parameter, pass `rig::binaries().1`, and delete `tmux_on_path`. |
| W-3 | warn (nit, T; not binding) | `crates/holler-cli/tests/pane_integration/wired.rs:27-41`; `scratch/mod.rs:293-297` | **Two small copies inside the new target.**<ul><li>(a) `wired.rs`'s `trap_dir` rebuilds the OpenCode trap of `no_hub.rs:56-72` (`Traps::new`): the same `hlr649-` temp dir, and an `opencode` script that touches `opencode-ran`, mode 0755.</li><li>(b) `Scratch::harness`'s `tui_session` closure is `rig::sessions_for(&[])` (`rig.rs:548-560`, already `pub`).</li></ul> | (a) One trap helper in `main.rs`, used by both files.<br>(b) Call `rig::sessions_for(&[])`. |
| W-4 | warn (low, O; optional follow-up) | `crates/holler-cli/src/pane/wiring/store.rs:290-336` | **The client's `Feed<E>` is the third instance of one iterator shape:** each reply's batch yielded one event at a time, an empty batch as `Ok(None)`, and the first error ending the stream. The other two are the hub's private twins, `holler-hub/src/panes/store.rs:295` and `profile/store.rs:347` (the second is commented "Twin of `panes::store::Feed`").<br>Not a block:<ul><li>the map does not name them;</li><li>they are private to `holler-hub`, which is outside the Blast radius (D16), and bound to `Arc<Store>::poll`;</li><li>the client's version adds D4's eager first request.</li></ul>It is generic over the event, so one copy serves both clients and adds no twin of its own. | If O wants it: a generic poll feed in `holler-pane` over `FnMut(Cursor) -> Result<WatchReply<E>, PaneError>`, used by the hub's two stores and by the client. No change in this run. |
| W-O | warn (carried from Phase 3; checked) | `crates/holler-cli/tests/pane_integration/bind.rs:422` | **Still holds.** There are three `struct LinkedHost`s: `pane_integration/bind.rs:422`, `pane_verbs/launch/rig.rs:663` and `profile_verbs/apply/rig.rs:636`.<ul><li>There is no fourth under any name. The only new `HostPort` that passes `run` to `FakeHarness::start_server` is #649's own.</li><li>The consolidation is still not filed. `gh issue list --search LinkedHost` finds only #649 and the closed #802.</li></ul> | O files it (Notes for O). |

### Checked and clean

**Production (F)**

- **`Wiring` is filled in place.**
  - `connect() -> Result<Wiring, PaneError>` and `ports()` are unchanged, so `main.rs` and `say_cmd.rs` are untouched.
  - `Wiring` holds no `Unwired` field, and `Unwired` itself stays.
- **Reused, not rebuilt:**
  - `StoreScope::new` (E19), with `const ACTOR` and `Actor::parse`, the pattern of `profile/create.rs` (E20);
  - `SystemProber` (E21);
  - `crate::transport::call(None, ..)`. `control::run`'s only caller in `holler-cli/src` is still `transport.rs:35`;
  - `ControlCall::status()`, used by `reach_hub`;
  - the params structs of `holler_pane::reply` and `PaneReply::into_result` (E10 to E13, E62);
  - `WATCH_WAIT` (E57) and `holler_hub::state::resolve_state_dir`;
  - `HerdrAdapter::connect`, `HerdrConfig::new`, `TmuxHost::new`, `OpenCodeHarness::new` and `Timeouts::default()`.
- **One mapping.** `store.rs`'s `call`, `timed_out` and `hub_unavailable` are the only `ControlError` to `PaneError` mapping in `holler-cli/src`. `call` holds its only `ControlCall {` literal (E70: the fields are public for this).
- **No analogue to extend.** I searched for each:
  - **A store client.** No other production `PaneStore` or `ProfileStore` client exists. Outside the hub and `holler-pane`, the only `PaneReply` user in `src` is `store.rs`.
  - **The environment readers.**
    - `holler-adapter-herdr` has no `Extent` parser and no Herdr socket resolver.
    - No `src` has a shared XDG helper.
    - The one other `PATH` search in `src` is `holler-body/src/query.rs:288` (`resolvable`). It is private, answers a bool and checks `is_file`, a different contract: `opencode_bin` must return an absolute executable through the injected `Lookup`.
  - **The no-hub text.** It is inlined in seven `*_cmd.rs` files, and there is no shared helper. `hub_unavailable` follows that pattern.
  - **The resolver's join on `(session, pane_id)`.**
    - `tx_launch.rs:779`'s `same_pane` is private and compares two `HerdrPane`s.
    - `tx_apply/observe.rs:171` inlines the same check, so inline is the dominant pattern.
    - Import's `labelled` (`import/assess.rs:164`) goes the other way, from a name to its panes.
  - **`LazyHerdr` beside `Wired<P>`.** They are two shapes by design: D2's lazy ping, and W-E's port-or-error. Both are D24's names, and A's fifth pass reviewed them.
- **D32's production items, all clean:**
  - no `TRIAL` header and no `eprintln!` in the wiring files;
  - no `Unwired` field, and no read of `HERDR_SESSION`;
  - no fallback to the default tmux socket, a value that is not UTF-8 included (W-T);
  - D4's eager first request (`store.rs:285`);
  - the manifest comment naming #649 (`Cargo.toml:402-404`);
  - no `UnixStream` or `UnixListener` in `holler-cli/src`.
- **Blast radius.** Every changed file is inside it. `holler-hub`, `pane/mod.rs`, `main.rs` and `say_cmd.rs` are untouched.

**Tests (T)**

- **D11.** The three rigs are included by `#[path]` (`scratch/mod.rs:38-45`), and each is generalised in place:
  - `scratch_herdr` gains `Flavor`. `HERDR_640` is the default of the unsuffixed functions, which delegate to the new `_as` and `_in` variants.
  - `rig.rs`'s items become `pub`. `lay_out` and `isolated_vars` are extracted from the rig's own constructor, which now calls them.
  - The private tmux server moves to `real_tmux/mod.rs`, and `real_tmux_test.rs` uses it, with no copy left behind. Its one `TmuxHost` helper, `private`, stays in place for AC 9's source guard.
- **The scenario copies none of the rigs.**
  - It uses the rig's dead-end config, model guard, port picker, `refused`, `raw` and `signal_group`.
  - Its name and path guards delegate to `scratch_herdr`'s: `check_session` to `check_name_as`, `check_inside` to `check_socket`.
- **`support`.**
  - `Hub`, `StateDir`, `wait_for`, `holler_bin` and `holler_cmd` are used.
  - `StateDir::at` (D22) is the one constructor added.
  - None of the overlay's helpers (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`) is re-declared, and no polling loop bypasses `wait_for`.
  - `LiveHub` wraps `Hub::start` (D22).
- **`FakeHub` is D13's sanctioned new object.** The CLI's tests have no fake control socket to extend. It reuses `StateDir`, `control_sock_path` and `holler_proto`'s own frame encoders.
- **Test-local types with no shared analogue:**
  - `Recording`, a call log. The test kit's `FakeHarness` keeps none, and the rig's `Recording` is a launcher, not a log.
  - `Resolving`, which resolves before `attach_tui`. The launch rig's `HookedHarness` fires after a method, and lives in another target.
  - `Out` and `output_of`, the root type of this target, as `pane_verbs/process` has its own. The scenario must check envelopes with #638's `check_envelope` (E55).

## Notes for F

None. The verdict is PASS, and no warn binds F.

## Notes for O

- **File the test-glue consolidation** (W-O; #664's gate asked for it). It is still unfiled.
  - It moves `LinkedHost` (three copies) to the test kit.
  - It can also absorb W-1 (`alive`, now in three test crates), W-2 (`isolated` takes the tmux binary) and W-3.
- **W-4 is optional.** Open an issue only if a shared poll feed is wanted for the hub's two stores and the client.
- **Carried, still open:**
  - comment on #825 that `profile apply` logs as `holler pane` (W-Q (a));
  - #653 and #828: the automated `pfleet o reset` caller;
  - #803 does not list #649.
- **For Aftersight** (pipeline bugs go there):
  - W-U, the diff gate's findings lost on the `archChanged` route;
  - F's two diff-gate input gaps: the 12,000-byte evidence cap, and no `--handoff`.

## Patterns referenced

- **The analogues searched:**
  - `crates/holler-hub/src/panes/store.rs` (`Feed`, 292-322) and `crates/holler-hub/src/profile/store.rs` (`Feed`, 344-374);
  - `crates/holler-pane/src/tx_launch.rs:779` and `crates/holler-pane/src/import/assess.rs:163-171`;
  - `crates/holler-body/src/query.rs:286-296`;
  - the no-hub text in `crates/holler-cli/src/{say,hold,roster,wait,answer,interrupt,hub}_cmd.rs`.
- **The included rigs:**
  - `crates/holler-adapter-opencode/tests/real_opencode/rig.rs`;
  - `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs`;
  - `crates/holler-adapter-host/tests/real_tmux/mod.rs`.
- **The CLI's test support:**
  - `crates/holler-cli/tests/support/mod.rs` (`StateDir`, `Hub`, `wait_for`);
  - `crates/holler-pane-testkit/src/harness.rs` (`FakeHarness`'s public API).
- **The three `LinkedHost`s:** `crates/holler-cli/tests/{pane_integration/bind.rs:422, pane_verbs/launch/rig.rs:663, profile_verbs/apply/rig.rs:636}`.
