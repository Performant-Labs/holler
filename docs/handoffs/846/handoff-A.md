# Handoff-A: #846 linkedhost-fakes  (up-front plan review, second pass after the amended brief)

**Date:** 2026-10-12
**Branch:** issue-846-implementation
**Brief reviewed:** docs/handoffs/846-brief.md (amended in `e1eae6a`)   **Reuse map:** the brief's "Files" / "Reuse map" paragraph (no separate survey.md)   **Wireframe:** N/A
**Verdict:** PASS

## Summary
The amended brief closes every block from the first pass (git `b59e745`) and I re-checked each fix against the tree at `6eff9cc`.
The design extends the right home (`tests/verb_harness/`, already `#[path]`-included by all four targets and not built by
`pane_cli_process`), adds no testkit, production or `Cargo.toml` change, and keeps the propagate/swallow difference as a named
mode. Only warnings remain, none of which changes the design.

## Verification of the first-pass blocks
| First-pass block | State in the amended brief | Checked against the tree |
|---|---|---|
| 1 AC 1 grep | now `HostPort for LinkedHost` and `struct LinkedHost`; `LostChain` in Out of scope | `grep -rnE "HostPort for LinkedHost"` and `"struct LinkedHost"` each print 4 lines today (apply `rig.rs:644/640`, launch `:694/688`, claude `:424/420`, `bind.rs:524/519`); `LostChain` (`import/harness.rs:307`) no longer matches |
| 2 `sessions_for` / W-3b | dropped from scope and AC 5 | not in `holler-cli/tests`; confirmed |
| 3 `profile_apply_scenario` | added to the AC 2 command; `scratch/`, `out.rs`, `scenario.rs` not edited, and nothing there may import from `pane_integration`'s shared code | `profile_apply_scenario/main.rs:33-37` path-includes `out.rs` and `scratch/mod.rs`; they use `crate::support` and `crate::{output_of, Out}` only |
| 4 no-hub text | W-6 rewritten to `no_hub.rs` + `fake_hub.rs` (four assertions), `scenario.rs` removed, both existing assertion strengths kept | `no_hub.rs:48`, `fake_hub.rs:349` (full string with state dir), `fake_hub.rs:423` (phrase only); confirmed |

Also confirmed: two `opencode-ran` writers (`wired.rs:36`, `no_hub.rs:67`) with the same `hlr649-` /tmp prefix and the same script body (W-3);
`observe.rs:27-55` and `bind.rs:293-316` are the duplicated conformance runner and the `LazyHerdr` closures are duplicated (W-7).
The four `LinkedHost`s match the evidence block: the host is an owned `FakeHost` in apply, launch (inside `Fakes`) and claude, and is
built inside an `Arc` in `bind.rs:630` alone. `Borrow<FakeHost>` serves `&FakeHost` and `FakeHost`; `Arc<FakeHarness>` serves `bind.rs`. The
`RIG_PORTS` consts (`[u16; 8]` in apply, `[u16; 3]` in launch) promote to `&'static [u16]`.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | AC 2 / Test plan wording | checkable criterion | AC 2 says "the four targets" and the Test plan says "the same four runs", but the command names five targets (`pane_verbs`, `profile_verbs`, `pane_claude`, `pane_integration`, `profile_apply_scenario`). A checker counting four could skip the one that proves `scratch`/`out` still build. | T records five `test result:` lines. Say "five" in both places. |
| 2 | warn | `verb_harness/mod.rs` module doc | naming / docs | Its doc says "Included by `pane_verbs` and `profile_verbs`"; it is in fact included by four targets and will now hold a second module (`linked_host`). | F updates the one line of the module doc when it adds `pub mod linked_host;`. |
| 3 | warn | Helper placement in `pane_integration` (`CallLog`, suite runner in `observe.rs`, imported by `bind.rs`) | dependency direction | `bind.rs` and `observe.rs` are sibling topic modules. Moving shared helpers into one and importing from the other is fine only if the dependency runs one way. | Prefer a neutral module (`traps.rs` for the trap builder and no-hub text, and the shared `CallLog`/runner/builder in one module that neither `bind.rs` nor `observe.rs` is a cycle with). If `common.rs` is used, its doc names what it holds. |
| 4 | warn | `FakeHarnessLink { on_start, kill_on_stop, ports }` | abstraction level | Three knobs (propagate/swallow mode, kill flag, port list) on one link, plus a private launch link, is as much as one type should carry. `pane_integration` passes "kill nothing" with an empty `ports`, which is redundant with `kill_on_stop: false`. | Keep two modes only as named enums, not bare bools; do not add a fourth knob. Phase 7 checks `kill_served` is defined once and called from `FakeHarnessLink::stopped` and launch's link. |
| 5 | warn | Size | file size | `launch/rig.rs` is 852 lines (fail line 900) and gains a ~15-line link while losing ~50; `bind.rs` is 693 and `profile_verbs/apply/rig.rs` is 776. | Keep AC 6's `wc -l` before/after for all four rigs; the new `linked_host.rs` stays well under 300 lines. |
| 6 | warn | Out of scope list | docs accuracy | The brief writes `profile_verbs/rig.rs:252` for a direct `start_server` call; that path exists (`profile_verbs/rig.rs:252`), distinct from `profile_verbs/apply/rig.rs`. Correct, but easy to confuse with the apply rig. | No action beyond keeping the full path. |
| 7 | warn | D2 (testkit home) | cross-cutting | `holler-pane-testkit/src/host.rs:12-17` prescribes this wrapper; folding into the testkit is the cleaner long-term home. Deferral is right for scope (production-crate, public API). | The module doc names the testkit prescription and the follow-up issue number; O files the issue before F finishes. |

## Notes for O
None required (PASS). Optional: file the D2 follow-up issue so the module doc can cite a number, and fix the "four runs" wording (finding 1) when convenient. Merge order for siblings is as in the brief (#845 before #846 if both are ready; #848 before the #846 PR).

## Patterns referenced
- `crates/holler-pane-testkit/src/host.rs:12-17` (the documented `HostPort` wrapper pattern) and `crates/holler-pane/src/ports.rs:371` (`HostPort: Send + Sync`, five methods)
- `crates/holler-cli/tests/{profile_verbs/apply,pane_verbs/launch,pane_claude}/rig.rs` and `pane_integration/bind.rs` (the four `LinkedHost`s)
- `crates/holler-cli/tests/verb_harness/mod.rs` and `pane_verbs/process/main.rs` (who builds `verb_harness`)
- `crates/holler-cli/tests/profile_apply_scenario/main.rs` and `pane_integration/{main.rs,scratch/mod.rs}` (path-included files and their imports)
- `scripts/lint.sh:43-50` (size gate)
