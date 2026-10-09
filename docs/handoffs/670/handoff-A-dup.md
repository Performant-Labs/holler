# Handoff-A-dup: Phase 7 - #670 the pane/profile CLI skeleton (skeleton slice c)  (anti-duplication gate, pass 2)

**Date:** 2026-10-09
**Branch:** issue-670-implementation
**Diff base:** f2602ba (merge base with origin/main)   **Diff head:** 56883d6
**This cycle:** e4365c4..56883d6. This is T's test-only rework after S's REWORK, one commit (56883d6). It changes no file under `src/`, and not the manifest, the fixture, ADR 0003, `Cargo.lock` or `CHANGELOG.md`. Pass 1 of this gate is the version of this file at e4365c4.
**Reuse map:** docs/handoffs/670-brief.md, the "Reuse map" paragraph under Files (this run has no survey.md)
**Verdict:** PASS

## Summary

PASS. The rework extends the shared harness and does not build a second one beside it. It also settles the two main warns from pass 1:

- **Harness ports (pass-1 W-2).** `run_verb_with(argv, format, ports)` is the "over given ports" entry the brief asked for. `run_verb` runs over the existing `Unwired`, so the code still has one not-implemented port set. No test refers to `Wiring` any more.
- **Shared files pinning siblings (pass-1 W-1).** The frozen roots and the shared process files no longer pin another story's stub. Only two places name a story's refusal line: `STUBS` with the two refusal constants in `process/stub.rs`, and the parameterised `assert_stub_routes`.
- **Parsing.** The new `verb_harness/parse.rs` uses `output::resolve_format` and does not copy any grammar.

Three warns remain. W-1 matters most. In two places the new layout still makes two wave-3 stories conflict on rebase when they land in parallel. Each fix is one or two lines.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `tests/pane_verbs/process/stub.rs:9-45`; `tests/pane_verbs/process/legacy_verbs.rs:10` | **Two places in shared files still conflict when neighbouring stories land in parallel.**<br>(a) The `STUBS` doc says a story "deletes exactly its own group". Each group's only separator is its own `// #NNN` line. If #643 and #644 each delete their whole group, header included, the two deletions are adjacent and git reports a conflict. I checked this in a scratch repo: deleting two adjacent whole groups conflicted, and deleting only the entries (keeping the `// #NNN` lines) merged cleanly. The ADR and the fixture do not have this problem, because there a story edits its rows in place and never removes a separator.<br>(b) `legacy_verbs.rs:10` imports `PANE_FORM_REFUSAL` (#646) and `ROSTER_PROFILE_REFUSAL` (#648) on one `use` line. When a story deletes its refusal test it must also remove its name from that line, or `unused_imports` fails clippy `-D warnings`. So #646 and #648 both edit the same line. | (a) Change the `STUBS` doc to say: delete your entries and leave your `// #NNN` line (an empty group does no harm).<br>(b) Give each constant its own `use` line with an unchanged line between the two, or write the full path where each constant is used.<br>Optional: assert that the story number of each `STUBS` entry equals `story_of(namespace, verb)` from `docs_rows::STORY_GROUPS`. Today each of the two story tables is checked against a different source (the binary's output, the fixture headers), and nothing checks them against each other. |
| 2 | warn | `tests/verb_harness/parse.rs:16-26,132-137`; `tests/verb_harness/mod.rs:55-57`; `tests/pane_verbs/target_flags.rs:15-19`; `process/main.rs:78-101`; `process/usage.rs:72-77`; `process/stub.rs:101-121` | **Small copies of test helpers, some of them new in the rework.**<br>(a) The `pane_verbs` binary now has three copies of "prepend `holler`, then call `Cli::try_parse_from`": `parse::try_parse`, inline in `run_verb_with` (same module tree), and `target_flags::parse`.<br>(b) `assert_spec_flags_accepted` repeats `assert_no_failures` word for word, because `parse.rs` is built into both binaries and cannot reach `process/main.rs`.<br>(c) Some of the converted tests repeat each other. `usage.rs::spec_only_with_a_profile_parses` makes exactly the check `flags.rs:42-59` makes (`accepted([... "--spec-only", "--profile", "demo"])`). Its `--format=text` cases overlap `flags.rs::format_is_a_global_flag`, and `each_argv_form_alone_parses` overlaps `SPEC_FLAG_SETS`.<br>(d) Carried over from pass-1 W-3: the one-envelope check is still in three places (`verb_harness::one_envelope`, `process::Out::envelope`, and inline in `stub.rs`). | (a) Make `run_verb_with` and `target_flags` call `parse::try_parse`.<br>(b) Move `assert_no_failures` into `parse.rs` and re-export it from `process/main.rs`.<br>(c) Delete the exact repeat. Whether to keep or merge the overlapping cases is T's call.<br>(d) The rework already includes `verb_harness/parse.rs` in `pane_cli_process` with `#[path]`. Including `verb_harness/mod.rs` the same way lets `Out::envelope` and `stub.rs` call `one_envelope`, so #638's conformance helper only has to replace one function. |
| 3 | warn | `tests/pane_verbs/process/main.rs:9-13`; `crates/holler-cli/Cargo.toml:477-478,487-492` | **Comments in frozen files no longer describe the test targets.**<br>- `process/main.rs` still lists the flag matrix among the things "only the binary can show". `flags.rs` and three `usage.rs` cases now check it in-process, and the positive matrix moved to `pane_verbs/{launch,relaunch}.rs`.<br>- The manifest says the in-process targets run over "the stub wiring's `Ports`". They now take the ports they are given, `Unwired` by default.<br>- The manifest says `pane_cli_process` is a separate target "so that it builds without the in-process API". It now builds `parse.rs`, which imports `holler_cli::output::{resolve_format, Format}`.<br>Sibling stories never edit the manifest, so these sentences stay wrong unless #670 fixes them. | Update the three comments now. Moving the in-process parse checks into `pane_verbs` would match the stated split, but that also means sharing the verb lists. Fixing the comments is enough. |

**Still open from pass 1.** None of these changed this cycle (no `src/` change), and all are recorded in handoff-S's advisory notes:

- **W-4:** `SpecValues` against `ProfileSpec`. #644 owns one PROVIDER/ID parser and one merge.
- **W-5:** small rules stated twice: exit codes, which namespaces get the envelope, `flatten`/`one_line`, and the stdio `Sink`. `interrupt | io/alpha` also appears twice in the fixture, at lines 55 and 178.
- **Note for #649:** `main.rs` calls `Wiring::connect()` before every verb.

### Checked: no duplication

- **Ports:** `unwired_ports()` builds the bundle from the `Unwired` in `src/`, so there is no second not-implemented port set. Its seven-field literal repeats the stub body of `Wiring::ports()`, but only until #649 rewrites that body over the real adapters. `Ports` has no constructor, so each owner of a port set writing its own literal is expected. The doc at `wiring.rs:10-13` says the harness builds from `Unwired`, and that is now true.
- **Format and parse:** `resolved_format` calls `output::resolve_format`, so there is no second resolver. `accepted` and `unknown_argument` have no existing counterpart:
  - `cli_surface_test::try_parse` takes a `SurfaceLine` and is private to a file outside the blast radius.
  - `docs_cli_test` only tolerates the help error kinds.
- **Story tables:** `STORY_GROUPS` replaces the inline `groups` array that pass 1 flagged as a copy, and now drives both the ADR-adjacency check and the fixture-header check. It is kept separate from `STUBS` because it is permanent while `STUBS` shrinks, and decisions.md records that reason. `SPEC_FLAG_SETS` is defined once and shared by `launch.rs` and `relaunch.rs`.
- **Refusal lines:** `PANE_FORM_REFUSAL` and `ROSTER_PROFILE_REFUSAL` moved from `legacy_verbs.rs` into `stub.rs`, which removes a copy rather than adding one. A grep for `not implemented (story #` under `tests/` finds it only in `stub.rs` and the parameterised `assert_stub_routes`. A grep for `Wiring` under `tests/` finds nothing.
- **Stack candidates:** nothing copies `Hub`, `Body`, `mint_token`, `join`, `wait_for` or `StateDir`. `holler()` still uses `support::StateDir` and `support::holler_cmd`.
- **Blast radius:** all 82 paths in `git diff f2602ba...HEAD` are inside it. The largest touched test file is `docs_rows.rs`, at 256 lines.
- **#669 (on origin/main, not yet in this branch):**
  - The hub stubs answer `PaneReply::failure(&PaneError::NotImplemented)`.
  - Its empty `pane_wiring.rs` names `holler-cli`'s `pane/wiring.rs` as the place the adapters are wired.
  - Nothing there overlaps `Unwired` or the CLI stubs. Per handoff-S, the only rebase conflict is in `CHANGELOG.md`.

## Notes for F

PASS, so nothing is required. All three warns are in T's test files or in comments, and none needs a change under `src/`. W-1 is the one worth doing before merge: two lines that save #643/#644 and #646/#648 one rebase conflict each.
