# Handoff-A: #831 adopt - adopting the fleet's existing Claude Code panes  (up-front plan review, third pass)

**Date:** 2026-10-12
**Branch:** `issue-831-implementation` at `7f69f8c` (`origin/main` is `ef881fc`)
**Brief reviewed:** `docs/handoffs/831-adopt-brief.md` (the brief amended twice)   **Reuse map:** the brief's "Reuse map" paragraph   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS: zero `block` findings, 5 `warn`. The second pass's one block (a flag-off Claude pane turned `import-blocked` by the port-less `source-missing`) is resolved: the brief now emits that finding on the OpenCode path only, keeps `Outcome::claude_code` for `Adoption::NotAsked`, pins it with a named green-before-and-after test, and says what the flag-on path does with `files.problems`. All six warns of the second pass are folded in (three-state `Adopt`, reworded flag-off text, failed-record equality check in the undo, the new bound test file, the three skipped OpenCode helpers, all-or-nothing `ps`). I re-checked the cited tree: `assess_all` (`assess.rs:135`), `assess` starting from `files.problems.clone()`, `checked` (`launcher.rs:115`), `write`/`undo`/`reconcile` (`writes.rs`), the stores keep a put verbatim apart from `generation` (so the equality check is sound), the membership rule only needs the profile to exist (so records-first in an extension is legal), the 12 `HostPort for` grep hits (10 sites + 2 doctests), `fake_tmux_test.rs` at 822, `ports.rs` at 750. The choke point (`send_prompt`) is untouched; no wire code changes except additive finding codes with the docs and tests updated in the same commit.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Claude record builder "in `adopt.rs`" ("the env rule of D3, `HarnessInfo { kind: Claude ... }`") | pattern consistency, duplication (a Phase 7 candidate) | `record()` (`assess.rs:390-435`) is a ~45-line `Pane` literal (host, `last_observed`, `hold`, `profile`, `probe`, `context`, `env`). A second literal in `adopt.rs` differing in `harness`, `session_of_record`, `model`, `opencode_agent` and `command` is a near-copy; the next field added to `Pane` must then be changed in two places. The brief justifies a new module for file size, not a parallel record builder. | Have both paths go through one constructor: split `record()` so it takes the `HarnessInfo`, the session, the `ModelSpec` and the agent from its caller (the OpenCode caller passes today's values), or make `adopt.rs` call a shared `pane_from(...)`. `adopt.rs` then holds only the Claude-specific choices (D3, pid, login check). A's Phase 7 will reject a second `Pane { .. }` literal. |
| 2 | warn | `assess_adopted` "starts from `files.problems.clone()` exactly as `assess` does" | duplication | The tail of `assess` (role, `pos`, `blocked`, `PaneRow`, `Outcome`) is another ~25 lines the adopt join would repeat. | Extract the shared tail (`finish(context, files, herdr, findings, record_builder)`) rather than copying it. The same goes for the label match of `Adopt::On(PaneId)`: reuse `labelled` (`assess.rs`), do not re-implement the snapshot filter. |
| 3 | warn | "Launcher": relative cwd and port 0 for a port-less entry; "each `source-missing` / launcher problems and block" | naming, test accuracy | In the tree, port 0 and a relative cwd are `source-unreadable` (`launcher.rs:138-145`), not `source-missing`; the brief's wording lets T pin the wrong code. Also, with `(None, Some(cwd))` now `Ok`, the relative-cwd check must still run for it (the brief says "keeps every other check", but the order in `checked` puts the port-0/cwd faults after the match). | State the codes: no entry, no cwd, no port-and-no-cwd are `source-missing`; port 0 and a relative cwd are `source-unreadable`. Add a T case: no port and a relative cwd is `source-unreadable`, flag on and off (OpenCode). |
| 4 | warn | Tests placement: "a new `adopt.rs` submodule if `harness.rs` grows past its file's budget" | size | `harness.rs` is 344 lines and the story adds well over 25 named tests (adopt, three flag states, each AC9 problem, D3, `spec-kind-differs`, the empty-`added` extension, undo cases in `writes.rs` at 348). Leaving the split conditional invites a late file over 600 (warn) or 900 (fail). | Decide it now: all flag-on tests in `crates/holler-cli/tests/pane_verbs/import/adopt.rs` (+ `adopt_undo.rs` for the AC10 cases), and `harness.rs` / `writes.rs` keep only their flag-off and replaced cases. |
| 5 | warn | Hub/real-store reality of the extension undo | state and persistence | The undo and the equality check are verified against the testkit store, which stores a put verbatim; the real stores (`crates/holler-hub/src/panes`, `HubPaneStore`) are not exercised by the fakes. | One sentence in the brief and the guide that the equality check relies on the store keeping a record verbatim apart from `generation`, and a note for T that the planned record contains no field a store fills (`last_observed.at` is 0 in the plan). Not a gate. |

## Notes for O

No amendment is required for the verdict. Fold findings 1 to 4 into the brief's Files section before F starts (they are one-line additions); finding 1 is what Phase 7 will look at first.

## Patterns referenced

- `crates/holler-pane/src/import/{assess,observe,files,findings}.rs`, `crates/holler-pane/src/import.rs` (`plan`, `assemble`, `Plan::refusal`)
- `crates/holler-cli/src/pane/import/{writes,launcher}.rs`
- `crates/holler-pane-testkit/src/pane_store.rs` (`put`, `check_membership`, `concurrent_put`), `crates/holler-hub/src/panes/mod.rs`, `crates/holler-hub/src/profile/mod.rs` (`check_membership`)
- `crates/holler-pane/src/pane.rs`, `crates/holler-pane/src/profile_diff.rs` (`is_member`), `scripts/lint.sh`
