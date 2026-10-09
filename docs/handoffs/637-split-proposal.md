# #637 split proposal (draft, not posted)

Why: three architecture reviews of the single-story brief drew BLOCK (6, 2, 3 blocks). Each block was a gap between two
parts of a ~40-path design that no compiler had checked. The implementer role's own scope cap (more than ~6 files or more than
one system surface: propose a split) also applies. Brief revision 4 (`182a69f`) is the source text for all three slices.

| Slice | Delivers | Brief parts it takes | Depends on |
|---|---|---|---|
| **637a** `holler-pane` crate | `holler-pane` (error codes, `Pane`/`Profile`/`ProfileSpec`, `GridPos`, `Argv`, `EnvVarName`, `PaneName`/`ProfileName`, generation/CAS, ports and `Ports`, `Prober`, `PaneReply` and params, empty module stubs); the four empty crates (`holler-adapter-herdr/-host/-opencode`, `holler-pane-testkit`); `PANE_METHODS`/`PROFILE_METHODS` in `holler-proto`; CHANGELOG entry | Decisions 2, 3, 7 (ports half), 8 (names and reply type); AC 6, 7, 8, 9, 10 (ports half), 12 | none |
| **637b** hub plumbing | `holler-hub`: forwarding arm, `PaneState`/`ProfileState` (`Arc`, `load`), `panes`/`profile` stub `dispatch`, `check_membership`, `pane_dispatch.rs`, empty `profile/rename.rs`, `pane_wiring.rs`; `holler-hub` Cargo deps; the hub dispatch test | Decision 4; AC 11 | 637a |
| **637c** CLI surface | `cli.rs`, `main.rs` (extraction, `try_parse`, format resolver), `output.rs`, `prompt_target.rs`, `--pane`/`--profile` shapes and guards on say/interrupt/answer/roster, `pane/**` and `profile/**` stub verbs, `wiring.rs`, `profile_scope.rs`, ADR 0003, CLI fixture, `pane_verbs`/`profile_verbs`/placeholder test targets, `verb_harness`, `holler-cli` Cargo deps | Decisions 1, 5, 6, 7 (VerbCtx/output half), 9, 10; AC 1-5, 10 (seam test), 13 | 637a (not 637b) |

637b and 637c run in parallel after 637a. 637c is still large (about 25 new files plus `main.rs`); if its first review blocks
on size, split again into 637c1 (`main.rs`, `output.rs`, format, usage errors) and 637c2 (clap verbs, ADR, fixture).

Changes this forces on the epic text (draft, to post only with the operator's go-ahead):
- #637 becomes 637a; 637b and 637c are new issues under epic #633 with the dependencies above.
- The epic's ownership table: the hub files move to 637b, the CLI files to 637c; every sibling's "blocked by #637" becomes
  "blocked by 637a, plus 637b (hub stories) or 637c (CLI verb stories)".
- Everything listed in the brief's "Blast-radius amendments" (decision 0 contradictions, per-verb test files, `main.rs` edits for
  #648, `send_prompt` path for #646, #634's method placement, the rustfmt reading) goes into the same edit.
