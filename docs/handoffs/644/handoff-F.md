# Handoff-F: #644 part 2 - the OpenCode agent through `launch` and `relaunch`

**Date:** 2026-10-11
**Branch:** `issue-644-implementation` (on `9eb9978`, T's RED; merge base `origin/main` `7284001`, #802 merged)
**Issue:** #644

Implemented against T's RED (`handoff-T-red.md`) and A's PASS (`handoff-A.md`, 0 block, 6 warn). No test file was
touched.

## What was done

- `crates/holler-pane/src/profile_diff.rs`: the 16th field.
  - `SpecField::OpencodeAgent`, path `opencode_agent`, placed in `ALL` after `Effort` and before `Role` (`ALL` is now
    `[SpecField; 16]`).
  - `value()` reads `spec.opencode_agent` into the new `FieldValue::Key(Option<String>)`. With the enum's
    `#[serde(untagged)]`, it serializes as the key or `null`.
  - `Display`: `Key(Some)` shares `Text`'s arm, so it goes through `write_escaped`, the file's one escaper.
    `Key(None)` prints `(default)`. `differs` gets no new arm, so the field is compared by plain equality.
- `crates/holler-cli/src/pane/launch.rs`: the result.
  - `Report` gains `agent: Option<AgentKey>` after `spec_only`. `Report::new` reads it from `launched.pane` (the
    record the CAS write returned), as it reads `spec_only`.
  - `Report::text` appends `, agent KEY` after the harness port and the `(pid P)` clause, and before `; profile ...`.
    The key goes through `list::text_value`. A pane with no agent prints exactly the old line.
  - The `PaneLaunch` doc comment (the verb's `--help`) lists `agent` among the data keys.
- `crates/holler-cli/src/pane/relaunch.rs`: the `PaneRelaunch` doc comment lists `agent`.
- `crates/holler-cli/src/pane/args.rs`: the `--agent` help now says "Without it, a new spec has none (the server's
  default agent) and an existing spec keeps its own."
- `docs/adr/ADR-0021.md`: the five places of Acceptance 8.
  - (a) Section 8 gains the agent's path, the `verify_serving` refusal and the output.
  - (b) The section 9 row gains `agent-key-invalid` and the harness's open code from `verify_serving`.
  - (c) The deferred bullet becomes "decided, #644".
  - (d) Decisions taken gains items 15 to 20.
  - (e) Section 3 says `profile_diff` compares `opencode_agent`. Section 3's spec-flag table also gains the
    `--agent KEY` row, and the `--from-current` copy list gains `opencode_agent` (A's warn 1).
- `docs/adr/ADR-0003.md`: the SPEC FLAGS bullet gains `--agent KEY` (A's warn 1; see Deviations).
- `CHANGELOG.md`: an `## [Unreleased]` Enhancements entry linking #644.

## Design decisions

- **`Report.agent` is `Option<AgentKey>`, not `Option<String>`.** It is the record's own type. `AgentKey` is
  `#[serde(transparent)]`, so the JSON is the key or `null` (evidence.md). No conversion means nothing can drift.
- **The text clause reads `self.agent`, not `pane.opencode_agent`.** Both are equal by construction, but reading the
  same field the JSON prints gives the two forms one source.
- **The key goes through `text_value` (A's warn 2), with no second escaper.** A key that passes its guard prints
  unchanged. The one exception is `-`: it is a valid key and also the line's empty value, so it prints `"-"`.
- **`FieldValue::Key(Option<String>)` follows the brief exactly.** `Option<AgentKey>` was considered and rejected. The
  brief names `String`, and the file's other text variants hold strings (`role` and `kind` too). Sharing `Text`'s
  match arm (`Text(text) | Key(Some(text))`) makes the single escaper visible in the code.
- **`(default)` is an inline literal, like `Argv(None)`'s `none`.** The `Display` doc gives the reason: `none` is a
  valid key, and no key can be `(default)`.
- **The `{"verb", ...}` code span sits on its own doc line in both verbs.** rustdoc and clap's paragraph join then
  both keep it contiguous. The rendered `--help` was checked by eye (below).
- **The `(pid P)` clause is kept, as the brief decided.** A noted that it cannot print since #802. See Known issues.

## Reuse / extend-vs-new

Everything extends an existing object, with no new production module, type or helper:

- `Report` is the one result type both verbs print, through `emit_outcome`.
- `SpecField` and `FieldValue` are the one comparison table, which `profile show` already prints and `diff_spec`
  already walks.
- `list::text_value` is the line's one rule for stored strings, and `profile_diff::write_escaped` is that file's one
  escaper.

`launch/rig.rs` and every test file are untouched by F.

## Architecture notes for A

- **Public API of `holler-pane`, additive but visible.**
  - `SpecField` gains a variant and `SpecField::ALL` becomes `[SpecField; 16]`.
  - `FieldValue` gains `Key`.
  - Neither enum is `#[non_exhaustive]`, so an exhaustive `match` outside the crate would need the new arm. There is
    none on this branch: `profile/show.rs` only iterates `ALL` and builds `FieldValue::Text`.
  - The same holds on the sibling branches #642, #647, #649, #650 and #664 (`git grep` for `SpecField::`,
    `FieldValue::` and `[SpecField; N]`). Their `; 15]` arrays are unrelated conformance-case tables.
  - Neither type derives `Deserialize`, so no wire, protocol or file format changes.
- **CLI JSON contract.** `pane launch` and `pane relaunch` `data` gains the additive key `agent`, last after
  `spec_only`. Every earlier key keeps its order, and `schema_version` stays 1 (ADR-0021:693).
- **Layers.** The only CLI change is to the view (`Report`) and help text. The engine, ports, test kit and adapters
  are untouched. The `verify_serving` refusal and its rollback are #802's engine behavior, used as they are (Scope 3).
- **No new dependency.** No `#[allow]` was added. File sizes: `launch.rs` 457, `relaunch.rs` 89, `args.rs` 170,
  `profile_diff.rs` 347.

## Deviations from spec / wireframe

1. **`docs/adr/ADR-0003.md` was edited, and it is outside the brief's Blast radius.** The edit is one token,
   `--agent KEY`, in the SPEC FLAGS bullet (line 93).
   - A's warn 1 recommended adding exactly this line to the Blast radius. O had not folded A's fresh-run warns into
     the brief before T ran.
   - It documents the flag of the two verbs this story owns, and the stack rule asks for `docs/` updates when the CLI
     surface changes.
   - #664's branch edits ADR-0003 only at lines 69-71 (its own row), so the hunks do not touch.
   - Reverting it is a one-line `git checkout` if O prefers it as a follow-up.
2. **ADR-0021 section 3 got more than Acceptance 8(e)'s one sentence.** The spec-flag table gains its missing
   `--agent KEY` row, and the `--from-current` copy list gains `opencode_agent`. Both are A's warn 1, made in the same
   section of a Blast-radius file.
3. **The ADR text follows A's warns 3 and 4, not the brief's wording.**
   - The brief says "code and message unchanged". The ADR says the code is unchanged and the message is the harness's
     message followed once by the reconcile step.
   - The brief says "rolls back and records nothing". The ADR says that is so for a launch, and that a refused relaunch
     leaves the pane with no server and its record unchanged.
   - T's tests pin the same behavior.
4. **`CHANGELOG.md` placement.** The entry goes after #802's, at the end of Unreleased Enhancements. #664 adds its
   entry at the top of the section, so the two do not share a hunk.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-pane --test profile_diff_test
test result: ok. 11 passed; 0 failed      (RED was 8 passed; 3 failed)

$ cargo test -p holler-cli --test pane_verbs --test profile_verbs
test result: ok. 275 passed; 0 failed     (pane_verbs; RED was 266 passed; 9 failed)
test result: ok. 45 passed; 0 failed      (profile_verbs; RED was 44 passed; 1 failed)

$ cargo clippy --workspace --all-targets -- -D warnings      -> 0 warnings, 0 errors
$ bash scripts/lint.sh                                        -> exit 0 (warn-only lines are pre-existing files over 600)
$ bash scripts/changelog-check.sh                             -> changelog-check: ok
$ rustfmt --edition 2021 --check <the 4 changed .rs files>    -> clean

$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
exit 0: 148 "test result: ok" lines, 0 not ok; 2127 passed, 0 failed, 25 ignored (the opt-in tests)
```

Every one of T's 13 RED tests now passes. The two refusal-table tests (`a_launch_the_server_refuses_records_nothing`,
`a_relaunch_the_server_refuses_leaves_the_record_unchanged`) were green before F, as T recorded. They pin #802's engine
behavior, which needed no change.

Rendered help (`target/debug/holler pane launch --help`; `relaunch` prints the same `--agent` line and the same key
list):

```
`--format=json` prints one envelope whose data is `{"verb", "pane", "profile", "spec_only", "agent"}`: the stored
record, the profile's name, slug and generation, or null, and the OpenCode agent the record names, or null.
      --agent <KEY>
          The OpenCode agent the pane's messages run as: letters, digits, - and _. Without it, a new spec has none (the
          server's default agent) and an existing spec keeps its own
```

## Evidence appendix

`docs/handoffs/644/evidence.md` holds 18 entries, each with a `file:line` source and a verbatim excerpt. A script
checked every excerpt against its cited lines, and all 18 match. They cover:

- `Launched.pane` is the CAS result, and `None` with `--spec-only`.
- The record's agent is the spec's.
- `AgentKey`'s serde and its guard.
- `NO_VALUE` and `text_value`.
- `ProfileSpec.opencode_agent`'s `skip_serializing_if`, and `FieldValue`'s `untagged`.
- `spec_from_pane`'s copy of the agent.
- `profile show`'s `push_spec` and `difference_line`.
- `relaunch` → `emit_outcome`, and `failure_body`'s single reconcile step.
- The engine's `verify_serving` after the health wait, and the rollback of the act.
- ADR-0021's `schema_version` rule.

## Tests that look wrong (for T)

None. One note: `a_key_that_reads_as_the_empty_value_is_quoted` pins A's warn 2, which the brief's text does not carry
yet. It is met through `text_value`, with no second escaper, as T's journal asked.

## Known issues

- **The `(pid P)` clause in `Report::text` cannot print.**
  - Since #802 the engine writes `pid: None` in every record, and `Report` reads the record this run stored.
  - The brief kept the clause, so it stays. Removing it is O's or the operator's call (A's warn 2 detail) and is
    inside this file.
- **A's warn 6 (coordination) is not F work.** Recording decision 4 on #664's issue and decision 1 on #642's issue is
  for O or the MO.
- **Merge order with the parallel runs.** #664's branch also edits ADR-0021 (around lines 548 and 620), ADR-0003 (line
  70) and `CHANGELOG.md` (top of Enhancements). None of those hunks touches this diff's hunks today. #642 part 3 and
  #649 have no edits to these files yet. Whichever run merges second merges `main` and keeps both texts.

## Files changed

- `crates/holler-pane/src/profile_diff.rs`
- `crates/holler-cli/src/pane/launch.rs`
- `crates/holler-cli/src/pane/relaunch.rs`
- `crates/holler-cli/src/pane/args.rs`
- `docs/adr/ADR-0021.md`
- `docs/adr/ADR-0003.md`
- `CHANGELOG.md`

Handoff artifacts (not production): `docs/handoffs/644/handoff-F.md`, `docs/handoffs/644/evidence.md`, and the
`docs/handoffs/644/decisions.md` entry.
