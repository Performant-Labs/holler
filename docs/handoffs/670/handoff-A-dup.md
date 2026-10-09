# Handoff-A-dup: Phase 7 - #670 the pane/profile CLI skeleton (skeleton slice c)  (anti-duplication gate, pass 3)

**Date:** 2026-10-09
**Branch:** issue-670-implementation
**Diff base:** f2602ba (merge base with origin/main)   **Diff head:** 4876a4c
**This cycle:** 51f3bed..4876a4c. It has three commits:
- S pass 2's REWORK (7939b54), handoffs only;
- F's rework 1 (02a3655): `src/prompt_target.rs`, +15 / -2;
- T's rework 2 (4876a4c): four test files and two comments in the manifest.

No other `src/` file changed, and neither did the fixture, ADR 0003, `Cargo.lock` or `CHANGELOG.md`. Pass 2 of this gate is this file at 51f3bed, and pass 1 is this file at e4365c4.
**Reuse map:** docs/handoffs/670-brief.md, the "Reuse map" paragraph under Files (this run has no survey.md)
**Verdict:** PASS

## Summary

PASS. F extended the object the map named and did not build a parallel path:
- **Production.** `resolve_tail` in `prompt_target.rs` now refuses a third positional in the SESSION form, using the existing `cli::Usage`. The check was added in place, next to the check the `--pane` arm already had. There is no new function, type or error type, no signature changed, and the change stays in the layer the brief gave the accessors.
- **Tests.** T's proving tests use what was already there:
  - the two in-process tests call the existing helpers in `target_flags.rs`: `parse`, `resolve`, `expect` and `session`;
  - the two binary cases are rows added to the existing malformed-form table in `legacy_verbs.rs`, and they run through the existing `holler()` helper.

From pass 2, W-1(a) (the `STUBS` layout) and W-3 (the out-of-date comments) are settled.

One warn remains. Pass-2 W-1(b), which is S pass 2's item 3, is not settled. The two refusal imports are now two `use` lines, but the lines are adjacent, so git reports a conflict when #646 and #648 each delete their own line. I reproduced this in a scratch repo. The fix is two comment lines in a test file.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:10-11`; `docs/handoffs/670/decisions.md` (T's rework-2 entry, "Assumed") | **S item 3 (pass-2 W-1(b)) is not settled: #646 and #648 still conflict on the import lines.**<br>- **Each story must delete its own `use` line.** Once a story deletes its constant (the `stub.rs` doc says to), the import cannot resolve. Once it deletes the last test that uses the constant, the import fails clippy `-D warnings` as `unused_imports`.<br>- **The two lines are adjacent.** Git treats changes on adjacent lines as one conflict region.<br>- **Reproduced.** In a scratch repo with a copy of this file, #646 deleted line 10 and #648 deleted line 11. The merge reported `CONFLICT (content)`, with the two `use` lines on opposite sides.<br>- **T's two premises are contradicted.** (1) Adjacent lines do conflict. (2) rustfmt does keep a line between the imports: `rustfmt --check --edition 2021` is clean both with a blank line between the two `use` lines and with a `// #NNN` line above each one. I also checked the second layout inside the repo, which has no rustfmt config. | - Put `// #646` above line 10 and `// #648` above line 11, the same convention `stub.rs` uses. Each story deletes its `use` line and keeps its comment line.<br>- I checked this layout in the same scratch repo: it is rustfmt-clean, and the two parallel deletions merge cleanly. A blank line between the two `use` lines also merges cleanly.<br>- Correct the T rework-2 entry in decisions.md.<br>- Owner: T. It is two lines. |

### Checked: no duplication

- **Production code:**
  - `resolve_tail` was extended in place. It reuses `cli::Usage` (`Usage::new` was already `pub(crate)`) and `Tail::forms()` for the message.
  - `route`, the three `*_cmd.rs` files and every signature are unchanged.
  - The SESSION arm's check for an extra positional sits beside the `--pane` arm's check. They are two short blocks with different messages inside one private function (F's design decision 2), not a second code path.
- **The limit of two positionals is stated twice:** in clap's `num_args` (`cli.rs:499, 536, 587`) and in `resolve_tail`. Both are needed:
  - clap still refuses the contiguous form `say io/alpha hello extra` itself, as decision 5 requires;
  - only the accessor sees the split form, where a flag separates the positionals.

  The doc of `resolve_tail` explains this, and F recorded the coupling in decisions.md. No sibling story adds a positional to these verbs. If only `num_args` were changed, the result is still safe, because the accessor refuses the extra positional. No action needed.
- **Tests:**
  - **No two tests repeat each other.** The new in-process test covers the SESSION arm, `an_extra_positional_after_pane_and_text_is_refused` covers the `--pane` arm, and `say_with_a_third_positional_is_still_refused_by_clap` covers the contiguous form.
  - **The new guard test is distinct.** `a_flag_between_session_and_text_still_resolves_to_both` puts a flag between SESSION and TEXT. `say_flags_after_the_tail_still_parse` puts the flags after both.
  - **The binary rows check what the in-process tests cannot:** exit 2, an empty stdout, no #646 refusal line and no hub line. S asked for tests at both levels.
  - **The `no live holler hub` text is matched the usual way.** Eleven existing CLI tests detect the hub path by this literal, and there is no shared constant to reuse.
- **Layout of the shared files (pass-2 W-1(a), settled).** The `STUBS` doc now says to keep the `// #NNN` line. In the scratch repo, both of these merged cleanly:
  - #643 and #644 deleting only their entries;
  - #646 and #648 each deleting its refusal constant and that constant's doc, while keeping `// #646`, `// #648` and the blank line between them. This is what the new doc on `PANE_FORM_REFUSAL` says to do.
- **Comments (pass-2 W-3, settled).** The module doc of `process/main.rs` and the manifest comments (`Cargo.toml:477-479, 488-494`) now match what the targets build:
  - the in-process targets take the ports a test gives them, `Unwired` by default;
  - `pane_cli_process` builds `support/mod.rs` and `verb_harness/parse.rs`, but not `verb_harness/mod.rs`.
- **Stack candidates.** Nothing copies `Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`, the token-store operations, `Lockout`, `Roster` or the `log(Severity, ...)` helper. The new binary rows go through `holler()`, which uses `support::StateDir` and `support::holler_cmd`.
- **Blast radius and size:**
  - All 82 paths in `git diff f2602ba...HEAD` are inside the blast radius.
  - The largest file this cycle touched is `target_flags.rs`, at 287 lines. `prompt_target.rs` is 202 lines.
  - The added lines use only `io/alpha`, `demo`, `demo-c1r1` and `ws://127.0.0.1:1`.
- **origin/main (1c48c30).** It changes nothing in `holler-cli`, and `git merge-tree` still conflicts in `CHANGELOG.md` only.

### Still open from earlier passes

None of these changed this cycle, and all are in handoff-S's advisory notes:
- **Pass-2 W-2 (optional):** small copies of test helpers:
  - three ways of writing "prepend `holler`, then call `try_parse_from`";
  - `assert_no_failures` repeated in `parse.rs`;
  - `usage.rs::spec_only_with_a_profile_parses` repeats a check from `flags.rs`;
  - the one-envelope check, in three places.

  This cycle added no new copy: the new tests use the helpers already in their files.
- **Pass-1 W-4:** `SpecValues` against `ProfileSpec`. #644 owns the one PROVIDER/ID parser and the one merge.
- **Pass-1 W-5:** small rules stated twice: the exit codes, which namespaces get the envelope, `flatten`/`one_line`, the stdio `Sink`, and the duplicate `interrupt | io/alpha` line in the fixture.
- **Note for #649:** `main.rs` calls `Wiring::connect()` before every verb.

## Notes for F

PASS, so nothing is required of F. Finding 1 is in T's test file: two comment lines, plus a correction to T's entry in decisions.md. It is the last open item of S pass 2's REWORK list (item 3), so S should check it before closing that item.
