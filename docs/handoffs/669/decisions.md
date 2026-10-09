# Decisions — #669 hub plumbing for `pane/*` and `profile/*` (skeleton slice b)

## A (Phase 3, up-front plan review) — 2026-10-09T06:12:39-06:00
- **Decided:** PASS on docs/handoffs/669-brief.md (Revision 1) at e33dab2, with 8 warns and no blocks; see handoff-A.md.
  - The plan extends the existing seams: one arm in `dispatch_control` that delegates to a new module (the `control_hold.rs` pattern); `encode_response` and `encode_error`; state loaded in `build_shared_state` beside `Holds::load`; non-`Clone` types shared as `Arc` the way `Lockout` is.
  - It leaves `send_prompt`, `holler-proto`, `CATALOG`, the golden files and the ADRs untouched.
  - It names where new code goes for the two files above 800 lines.
  - The dependency direction is right: `holler-hub` depends on `holler-pane`, and `holler-pane` depends on neither the hub nor any runtime.
- **Assumed:**
  - The issue's signatures are shorthand that F may meet through deref coercion, so a call site of `&deps.panes` satisfies `&PaneState` (W1).
  - #661's `pane-in-other-profile` rule is meant to run in the hub's `cas_put` hook, as #661's issue says, and not on the CLI side (W2).
  - The `mod.rs` exception recorded in #670's brief reflects the same reasoning that applies to this slice (W5).
- **Hedged:**
  - W1: the hub has no single pattern for blocking store I/O. `token.rs` uses `spawn_blocking`; `holds.rs` writes on the caller's thread. So this is a warn, not a block.
  - W2, W6 and W7 change the issue text or another story's radius, so O or the operator decides. A raised them as warns rather than amending the brief.
  - W3: AC 3's test as worded cannot observe the dispatcher's arguments from `tests/`. A recommends testing at the bundle seam and adding a `compile_fail` doctest, rather than adding new surface to `serve.rs`.
- **Evidence:**
  - Code: `control_server.rs` (whole file), `serve.rs` (whole file), `state.rs`, `lib.rs`, `control_hold.rs`, `control.rs:55-140, 419-518`, `live.rs:395-470`, `holds.rs:1-120, 196-240`, `roster.rs:70-110, 240-300`, `lockout.rs:337-345`, `token.rs:715-799`, `circuit/admin.rs:1-80, 255-300`, `circuit/auth.rs:38-62`.
  - `holler-pane`: `lib.rs`, `reply.rs`, `ports.rs:1-120`, `error.rs:295-460`, `pane.rs:1-80`, `profile.rs:160-200, 328-370`. `holler-pane-testkit/src/lib.rs`. `holler-proto/src/methods.rs`.
  - Repo docs and config: `scripts/lint.sh`, `clippy.toml`, `docs/testing.md` (layout and rules), `docs/adr/README.md`, `ADR-0006.md`, `ADR-0020.md`.
  - Issues #669, #633 (epic, including the Skeleton split), #639, #649 and #661 via `gh issue view`. `docs/handoffs/670-brief.md` in the #670 worktree.
  - Searches: `find crates -path '*/src/*' -name mod.rs` returned none. `grep -rn UnixStream::pair crates` returned none. Line counts are from `wc -l` on every file the plan touches.
