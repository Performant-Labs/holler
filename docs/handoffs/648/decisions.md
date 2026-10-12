# Decisions: #648, the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ

Run handoff directory `docs/handoffs/648` · branch `issue-648-implementation` · uiSurface false (D and U are N/A).
Append-only; every phase adds its own entry.

## A (Phase 3, up-front plan review): 2026-10-11T19:49-06:00

- **Decided:** PASS on `docs/handoffs/648-brief.md` at `1c81e2d` (sha256 `1f12c6140e9e6508...`). There are no blocks, and twelve warns, eight of them binding. The detail is in `handoff-A.md`.
  - **The objects and the layers are right.**
    - `roster_cmd.rs` keeps the hub call and joins the hub's rows to the registry CLI-side, reusing `pane::list::PaneRow` and `output::emit`/`emit_error`.
    - The registry is read through `Wiring::connect`'s ports and `ProfileScope`.
    - The hub, the wire and `holler-pane` are untouched, and nothing persists.
  - **The binding warns:**
    - **W-1:** the seam. The hub call comes first. A deferred source of ports, which `main.rs` builds around `Wiring::connect`, is called only after a reply and never under `--server`. One pure function takes the registry as ports, error or "not read". Under `--profile`, a failed `connect` is R4's error, and `scope.resolve` is the only read.
    - **W-2:** the pair rule is one named predicate, pinned to `say_cmd::resolve_pane_target` by a test. Moving it into `pane::list` is the approved alternative.
    - **W-3:** ADR 0003 lines 91 and 99 and the help at `cli.rs:63` and `:71-75` get a roster carve-out. Exception approved.
    - **W-4:** three output modes. Every roster `usage` is a stderr line. Other errors go through `emit_error`. Hub errors are `unavailable`, except `RemotePolicyRefused`, which gets a roster-owned open refusal code (exit 3).
    - **W-5:** the six cells are taken by position in `pane::list::COLUMNS`, pinned by a test.
    - **W-6:** an in-process hub call runs only under the state-dir lock, with a scratch `StateDir`. Golden rows carry no wall-clock cell.
    - **W-7:** `render_table`'s fixed-width cadence is extended, and the legend is `--help` text.
    - **W-8:** three ADR-0021 sentences are added (parked panes; the wiring's users; the open code). Exception approved.
  - **Not binding:**
    - W-9: `note:` against `warning:`, and a JSON signal for a partial read (for O);
    - W-10: one join function, and a key-collision test;
    - W-11: the stale line 133;
    - W-12: spec observations for T and S.
- **Assumed:**
  - Clap derive uses the doc comment of `Command::Roster` as the long help of `holler roster --help` (not probed).
  - The deferred source in W-1 compiles in the shape described (a closure over an `Option` or `OnceCell` holding the `Wiring`). F may pick another shape with the same order and ownership.
  - `RefusalCode::from_static` accepts a new roster-owned code as it does `say_cmd.rs`'s three. Its name is F's.
- **Hedged:**
  - **Warns instead of a BLOCK.** W-1 to W-8 are binding warns, not a BLOCK (the #649 precedent). None changes an acceptance criterion, and a BLOCK on an unchanged brief would stop the run for an amendment the warns supply.
  - **Overriding R10.** W-3 and W-8 approve Blast radius exceptions that override R10's "nothing else in that file changes". That clause was decided about line 94 alone, and the project rule requires the ADR to be updated in the same change. The operator can overrule.
  - **The alternative for W-4.** R3's form for `RemotePolicyRefused` (no envelope) is left to O.
- **Evidence:**
  - **Read in full:** the brief's refresh, scope, acceptance, blast radius, decisions and E11; `roster_cmd.rs`, `main.rs`, `output.rs`, `pane/list.rs`, `say_cmd.rs`, `pane/wiring.rs`.
  - **Read in part:** `wiring/store.rs`, `transport.rs`, `cli.rs`, the hub's `roster.rs` and `control_server.rs`, `profile.rs`, `ports.rs`, `profile_scope.rs` (both), `vocab.rs`.
  - **ADRs:** ADR-0003 lines 88-110; ADR-0021 lines 245-275, 316-330, 685-720, 760-830, 862-895.
  - **Tests:** `roster_cli_test.rs`, `hold_cli_test.rs`, `grant_cli_test.rs`, `pane_verbs/{main,list}.rs`, `pane_verbs/process/{main,docs_rows}.rs`, `pane_integration/main.rs`, `verb_harness/mod.rs`, `docs_pane_control_test.rs`; and the `Cargo.toml` test targets.
  - **State checked:** `git fetch` (`origin/main` is still `bda1069`); `gh issue view 839` and `648` (both open).
