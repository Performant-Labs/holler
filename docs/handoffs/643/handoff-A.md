# Handoff-A: Phase 3 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (at 2e7f221, the brief commit on 3bdd129)
**Brief reviewed:** docs/handoffs/643-brief.md (sha256 62fce781...)   **Reuse map:** docs/handoffs/643-brief.md, "Reuse map (extend, do not duplicate)" (lines 986-1001)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with no blocks and eight warns. The plan uses the right objects and puts them in the right layers. The three verbs
read only through the frozen ports (`PaneStore::{get,list,watch}`, `ProfileStore::get`, `ProfileScope::resolve`). They
print only through `output::{emit, emit_stream, emit_error}`, raise only closed codes, and type their names in `run` the
way `SpecFlags::validate` does. They reuse the record's own serde forms and `GridPos`, and edit no frozen file. The new
shared view code (`PaneRow`, `SessionSync`, `text_value`, `observed_at`) has to live in `list.rs`, because the frozen
`pane/mod.rs` cannot gain a module, and the brief says so in writing.

The warns are almost all seams with the wave-3 briefs being planned in parallel (#644, #646, #647, #662). Those plans
already define their own copies of three things this brief calls "one copy": the SHOWN/DRIVEN rule, the terminal-safe
text helper, and the fakes-backed test rig. W-1 matters most. #647, and #644 as amended after its own A, read "SHOWN
versus DRIVEN" as SHOWN versus `session_of_record` and keep `last_observed.driven` empty until #649. Under those plans,
this brief's SYNC column would read `-` for every real pane while `pane doctor` reports mismatches. #643 cannot fix the
other plans inside its own blast radius, and the documented contract supports both readings. The MO should settle W-1 to
W-3 before the second of these stories merges, and W-1 before T pins AC 3 if possible (see "Notes for O").

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn (the most consequential) | Decision 3 (`SessionSync`, brief:1019-1025); Decision 2's "No copy of any of them elsewhere" (brief:1015-1018); forward-compat rows #648 and #647 (brief:1132, 1136); AC 3, `{shown: None, driven: "ses-a"}` → `-` / `"unobserved"` (brief:837-842) | dependency direction; contract shape (one rule, three layers, two readings) | Three layers need the SHOWN/DRIVEN rule, and only the CLI can import a `holler-cli` type. (1) #646's refusal "when the shown and driven sessions differ" belongs at `send_prompt` in `holler-hub` (pane.rs:160-164; issue #646, "`send_prompt` has no path to pane state"; ADR-0021:344). `holler-hub` cannot depend on `holler-cli`. (2) #647's `shown-driven-mismatch` finding lives in `holler-pane` (`findings.rs`, `reconcile.rs`, ADR-0021 §5:178-181). So "No copy elsewhere" cannot hold, and the #647 row is true only for `doctor.rs`. (3) Two sibling plans have already settled on a different reading. #647 compares SHOWN with `session_of_record` ("which the hub drives under I2"), counts a home-screen `None` as a mismatch, and keeps `last_observed.driven` as stored (647-brief.md:951-953, 975, 997). #644 was amended in its working tree after its own A BLOCK (uncommitted when read). It now writes `last_observed.driven = None` at launch, keeps it on relaunch, and adopts "SHOWN == `session_of_record` ... the reading #647's plan uses" (644-brief.md:118-122, 1721-1722; its committed e7064e9 wrote `driven: Some(sid)`). #647 also plans to write its reading into ADR-0021 §11 (647-brief.md:950-953), so whichever story merges second will contradict the ADR unless the plans are aligned. If both land as planned, no record carries a DRIVEN until #649 wires it (ADR-0021 §11:452-453). `pane list`/`get`/`watch` would then print DRIVEN `-` and SYNC `-` for every real pane, while `pane doctor` reports `shown-driven-mismatch` for the same panes, and #648's roster, if it reuses `SessionSync`, stays silent too. The home-screen case diverges even once DRIVEN is set. This is the epic's headline signal ("a SHOWN/DRIVEN mismatch is loud"). The documented contract supports both readings: #643's issue says DRIVEN comes from `last_observed`, while I2 makes `session_of_record` the session the hub drives. On `None`, the frozen port says "if it can tell" (ports.rs:198), and the merged fake says "on its home screen or with no TUI" (testkit harness.rs:135-136). There is no dominant pattern, so this is a warn, not a block. | The MO picks one reading for the epic, ideally before T pins AC 3. (a) Keep #643's: SYNC compares `last_observed.shown` with `.driven` and is `-` until #649. Then `--help` must say that DRIVEN and SYNC read `-` until the hub's DRIVEN is wired, and #647 should name its finding for what it compares. (b) Take #644 and #647's: SHOWN vs `session_of_record` under I2. Then Decision 3, AC 3 and the help text change. Either way, settle what `shown: None` means. Record the rule once, for example as one sentence in ADR-0021 §1's `last_observed` row through the amend channel. Give it one home: a pure function in `holler-pane` (for example `LastObserved::sync()`), added amend-first, since the hub, the engine and the CLI all need it. In this brief now: correct Decision 2 and the #647 row (only `doctor.rs` can reuse `SessionSync`), add #644 and #646 rows, and journal the chosen reading as an assumption. |
| W-2 | warn | Decision 11 (`text_value`, brief:1090-1097); Risks, "`text_value` is the one choke point" (brief:1206-1211); Reuse map (brief:986-1001); `get`'s `probe-last` form (Decision 5, AC 7) | duplication; cross-cutting (terminal safety) | The Reuse map does not name the existing helpers for the same job. None of them can be reused as is: `hub_cmd::printable` (hub_cmd.rs:129-134, private, replaces non-ASCII with `?`), `holler_proto::log::escape_field_value` (log.rs:474-497, private, escapes control characters only, no quoting), `holler_hub::holds::sanitize_reason` (holds.rs:213-218, drops characters), `LockoutKey`'s `Display` (lockout.rs:273-281, `?`), and `holler_pane::error::excerpt` (error.rs:686-695, `pub(crate)`, always `{:?}`, cut to 64). So a new helper is justified, but the brief must say why. Its `{:?}` style matches `excerpt`'s. The parallel briefs then add two more helpers. #647 has its own sanitizer in `findings.rs` and a renderer that `{:?}`-quotes (647-brief.md:1016-1019), the same style as #643's in a separate copy. #662's `FieldValue` `Display` uses `char::escape_default` with no quoting (662-brief.md:1519-1521), a different style. The plans also disagree on the probe-result text: #662 prints `failed (missing "a", "b")` (662-brief.md:1638-1640), and #643 prints `failed missing=["ok"]`. Across `pane get`, `pane doctor` and `profile show`, the operator would get three helpers, two escaping styles and two probe-result forms. This is a cross-cutting concern of the shared output module ("one module decides what the verbs print", output.rs:1-2; open #660). Kept in `pane/list.rs`, it means `profile/show.rs` must import a pane verb file. | Add the five existing helpers to the Reuse map, each with one clause on why it does not fit. Add #662 and #647 forward-compat rows that name `text_value` (and the argv-as-JSON and probe-last forms) as the shared ones. O raises it on the epic or on #660: one terminal-safe text helper, ideally in `output.rs` (owner #660), or `pane::list::text_value` designated until then, settled before the second of #643/#647/#662 merges. |
| W-3 | warn | Decision 10 (`pub(crate) struct Rig` in `tests/pane_verbs/list.rs`, brief:1081-1089) | duplication (test harness) | The parallel briefs each build their own fakes-backed ports rig. #644 has `pub(crate) mod rig` in `tests/pane_verbs/launch.rs` or `launch_rig.rs` (644-brief.md:58, 1806). #647 has `tests/pane_verbs/doctor/rig.rs` (647-brief.md:911). #662 has `pub(crate) mod rig` in `tests/profile_verbs/list.rs`, with `ports()`, `run(argv, format)` and `assert_no_adapter_call()` (662-brief.md:1756-1760), nearly the same API as this `Rig` (`ports`, `run`, `assert_nothing_observed`). Three of these would sit in the one `pane_verbs` test binary. #643's `Rig` copies nothing on `main` (the only `Ports` builder is `verb_harness::unwired_ports`, verb_harness/mod.rs:25-36), so this is not a block for #643. | Add a forward-compat row: "#644-#647 `pane_verbs` tests reuse `crate::list::Rig`". O tells the parallel stories to reuse whichever rig merges first, and their A-dup gates reject a second copy. A follow-up can move the one rig into `tests/verb_harness/`, so `profile_verbs` shares it too. |
| W-4 | warn | Decision 7, `watch` JSON `data` = `{cursor, name, change, pane: PaneRow \| null}` (brief:1060-1063); Decision 5, `get` JSON `data.pane` = the `Pane` record (brief:1035); AC 3 (`data.pane.sync`), AC 5, AC 13 | contract shape; naming | The key `pane` has two shapes in sibling verbs. In `watch` it is a `PaneRow` (summary: `health`, `sync`, `pos` at its top). In `get` it is the full record (`harness.health`, `herdr.grid`). The port's own `PaneEvent.pane` (pane.rs:265-267) and the `pane/get` and `pane/list` data (ADR-0021:229-230) also use `pane`/`panes` for records. A script written against `get`'s `data.pane` breaks on a `watch` line. Once merged, a rename is breaking (ADR-0021:408-409). Precedent is mixed: `hub token list --json` uses `tokens` for summaries (token_cmd.rs:121-135). Hence a warn. | Decide before AC 3, 5 and 13 pin it. Either name the summary `row` in `PaneChange` (`{cursor, name, change, row}`; `list`'s `panes` can stay, as `tokens` allows), or carry the record under `pane`, as `PaneEvent` does, with `sync` beside it, as `get` does. |
| W-5 | warn | Decision 7, "A watch that owes nothing prints nothing, in JSON mode too" (brief:1071-1073); AC 16 (brief:909-910) | contract with the shared test-kit checker | `check_ndjson` treats an empty stream as `EmptyStream` (envelope.rs:35-37, 144-145, 247-249). The issue says "every `--format=json` output ... passes #638's envelope helper", and #649's scenario checks that "every verb's `--format=json` output parses against the envelope" (epic #633, integration criteria). The carve-out is documented, and the port's idle `Ok(None)` carries no cursor to print, so it is the right local choice. Still, the shared checker and the verb now disagree, and an idle `watch --until-idle` in #649's scenario fails the checker. | Journal it as a known divergence. O opens a test-kit follow-up that names #649: for example, accept an empty stream at exit 0, or add a `check_ndjson` variant for `--until-idle`. |
| W-6 | warn | Decision 16 and C5, I5 read per port call for `watch` (brief:1123-1126, 1158-1160) | ADRs | I5 says "A verb returns in bounded time" (ADR-0021:164). `watch` without `--until-idle` does not. §12 ("every port call is bounded by I5", ADR-0021:457-460) and §9's NDJSON stream (ADR-0021:363) support the brief's reading, and ADR-0021 defers nothing to #643, so no ADR edit is required here (unlike #642's B-1). But the stack rule is that a decision extending an ADR's reading updates the ADR. | Add one line to ADR-0021 §12 through the epic's amend channel, for example: "`pane watch` without `--until-idle` is the one stream verb; each `next()` is bounded by I5". Or record it on #634. Journal it either way. |
| W-7 | warn | Test plan, "T first lands the **surface**": the three `Args` structs' fields in `src/pane/{list,get,watch}.rs` (brief:1182-1186) | process; pattern consistency | Here T writes production code before RED, which tester.md forbids (tester.md:8, 23, 208). The repo's established way to stage CLI surface that is not implemented yet is `cli-surface.pending.txt` (cli_surface_test.rs:1-15, 66-79; #508's T-red used it, 508/handoff-T-red.md:21). The brief's reason is sound: without the fields, the in-process cases panic in `run_verb_with`'s parse, and the overlay does not accept that as RED. The parallel #644 ("T-red (Args)", "T-red (stubs)", 644-brief.md:41-43) and #647 ("T first lands the type declarations", 647-brief.md:1146) briefs do the same, so this is not a block. | Keep the plan, but put the exception explicitly in T's task: fields and one-line docs only, `run` untouched, no behaviour. The MO adopts one rule for the epic's verb stories. |
| W-8 | warn (low) | Decision 11's trigger set (control, whitespace, `"`, `\`, `=`); Risks, bidi characters accepted (brief:1209-1211) | cross-cutting (security) | A value whose only unsafe characters are Unicode format characters passes unquoted, for example bidi overrides U+202A-U+202E and U+2066-U+2069. Such a value can reorder how a line displays. | Optional and cheap: also quote when the value differs from its own escaped form, that is, when `format!("{s:?}")` minus its quotes differs from `s`. Rust's `{:?}` already escapes these characters (checked with rustc: U+202E, U+2066, U+200B and U+FEFF come out escaped, while `é` passes through), so this needs no new table. |

### Checked and consistent with existing patterns (no finding)

- **Ports only, nothing observed.** The verbs call only `pane_store.get/list/watch`, `profile_store.get` and `scope.resolve`
  (ruling 1; ADR-0021 §5). AC 17 checks this both by grep and by the fakes' call logs. No adapter, hub, proto or golden file
  changes, and nothing is persisted.
- **Output and exit codes.** Every result goes through `emit`, `emit_stream` or `emit_error`, and exit codes come from
  `class_of` (output.rs:201-243, 336-340). Only the closed `PaneNotFound` and `PaneNotInProfile` are raised; no code is
  declared (ruling 3). The failure modes match ADR-0021 §9's table (lines 336-337).
- **Arguments.** Positionals and `--profile` stay strings at clap time and are typed in `run` with `PaneName::parse` and
  `ProfileName::parse`, so a bad name is an emitted `usage`. This is the `SpecFlags::validate` pattern (args.rs:1-13,
  110-150). `ProfileOpt` stays flattened. The verb-specific positional and flags live in the verb's own file (ruling 2), and
  the ADR 0003 rows, fixture lines and `STUBS` rows are this story's (ADR-0003:92; stub.rs:12-16). `flags.rs` already
  counts "only a required positional is missing" as accepted, so `get PANE` breaks no process test.
- **Reuse of serde forms.** `get` prints the record verbatim (`Pane`, `ProfileSpec`). Positions use `GridPos` `Display` and
  `Serialize` (grid.rs:68-73, 129-138). Argv in text is `serde_json` of `Argv` (transparent array, argv.rs:20-27). Time
  formatting uses `format_epoch(ms / 1000)`, as attach_cmd.rs:189 does. The ` UTC` suffix is new, since other callers print
  no zone; it is harmless.
- **Shared view placement.** `list.rs` is the only home that edits no frozen file. A child module (`pane/list/view.rs`,
  declared from `list.rs`, the way #647 nests its test rig) would also avoid `pane/mod.rs`, but it is not needed at about 290
  lines. `pub` items in a `pub mod` raise no `dead_code`.
- **JSON wrapper.** `list` puts its rows under `{"panes": [...]}`, the same shape as `hub token list --json`'s
  `{"tokens": [...]}` and the roster's `{rows: [...]}`, and ADR-0021 §9's rule that fields may only be added.
- **Membership.** Profiles are compared inline by slug, as the hub's `refuse_profile_move` (panes/store.rs:338-357) and the
  fake scope (profile_scope.rs:238) do. The spec lookup `entry.pane == name` matches the fake's (profile_scope.rs:244, 282).
- **`--since` ahead of the head is `usage`.** The verb leaves this check to the store, and the hub's real feed refuses it
  the same way (holler-hub/src/panes/feed.rs:22, 86-95). So the fake and the hub agree, which answers the outside
  reviewer's W-4.
- **Size.** About 290, 210 and 210 lines of production code and at most about 360 lines per test file, far below 800 and
  900, with a stated overflow plan. No new dependency, no `unsafe`, and only neutral placeholders (public repository).

## Notes for O (non-blocking)

- W-1 to W-3 are seams between parallel plans. #643 can make its half right (correct the forward-compat rows, name the
  existing helpers), but the decisions belong to the MO. Settle them before the second of #643/#644/#647/#662 merges:
  1. One SHOWN/DRIVEN rule (W-1): which pair is compared before #649 wires DRIVEN, and what `shown: None` means. If #643
     keeps its reading, its `--help` must say that DRIVEN and SYNC read `-` until then.
  2. One terminal-safe text helper and probe-result form, with an owner (W-2).
  3. One `pane_verbs` rig (W-3).
- W-4 is the one choice this story controls that becomes permanent public JSON. It costs nothing to settle now and needs a
  `schema_version` bump later.
- I used the parallel briefs (#644, #646's issue, #647, #662) as evidence of what is being planned, not as merged code. None
  of them has passed its own A, and #644's brief was being amended while I read it: its `driven: None` change is
  uncommitted in `.claude/worktrees/0644-launch-relaunch`. Their details and line numbers, as read at about 17:15 MDT on
  2026-10-09, may still change. The authorities are the issue (#643), the epic (#633), ADR-0021, ADR 0003 and the merged
  code.

## Patterns referenced

- `crates/holler-cli/src/output.rs` (the one output module) and `crates/holler-cli/src/pane/args.rs` (strings at clap time,
  typed by guards).
- `crates/holler-pane/src/{pane,ports,profile}.rs` (the records, `PaneEvent`, the `shown_session` contract) and
  `docs/adr/ADR-0021.md` §§1-3, 5, 6, 9, 12.
- `crates/holler-pane-testkit/src/{envelope,harness,profile_scope}.rs` and `crates/holler-cli/tests/verb_harness/mod.rs`.
- `crates/holler-cli/src/{hub_cmd,roster_cmd,token_cmd,time_fmt}.rs`, `crates/holler-proto/src/log.rs:474-497` and
  `crates/holler-pane/src/error.rs:686-695` (the existing text, table and escaping practice).
- The parallel briefs: `.claude/worktrees/0644-launch-relaunch/docs/handoffs/644-brief.md`,
  `.claude/worktrees/0647-reconcile-doctor/docs/handoffs/647-brief.md`, `.claude/worktrees/0662-profile-verbs/docs/handoffs/662-brief.md`.
