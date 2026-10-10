# Handoff-A: Phase 3 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (up-front plan review, round 2)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (at 50bcc83, T-green's PASS; base 3bdd129, still `origin/main`)
**Brief reviewed:** docs/handoffs/643-brief.md (sha256 62fce781..., unchanged since round 1)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" (lines 986-1001)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

Round 1 of this review (PASS, 8 warns, on 2e7f221) is in git: `git show fafd138:docs/handoffs/643/handoff-A.md`. This
file replaces it. Round 1's open warns are carried forward below, with their status.

## Why this pass ran

The outside diff gate BLOCKed round 1 of the diff (`docs/handoffs/643-diff-result-r1.md`, gitignored, 4 BLOCKs). F had
reported `archChanged` for the seven `pub` helpers it added to `list.rs` beyond the four the brief names. So the run came
back to this phase instead of going to F. The brief is unchanged and the code exists, so this pass checks three things:
the plan, the architecture F built, and the gate's findings.

## Summary

PASS, with no blocks and nine open warns, three of them new. F's code keeps the plan's architecture. It touches
`holler-cli` only, in the three verb files. It reads through the frozen ports and prints through `output.rs`, and it
edits no frozen file. The sibling dependency runs one way (`get` and `watch` use `list`). The extra helpers sit in the
file and layer that Decision 2 chose. The gate's three real BLOCKs (B-1 to B-3) are not defects: the code, the codebase's
own patterns and a rustc check all show it. None of them needs a production change. The real problem is W-9: the brief no
longer describes the code on four points, and the outside gate reads the brief. The next F should put the facts below
into `evidence.md`, which the gate also reads.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-9 | warn (new; act on it this cycle) | Decision 2, four `pub` items (brief:1015-1018); Decision 6, bare `serde_json::to_string` (brief:1053-1056); Decision 11's trigger set (brief:1090-1097); Risks, bidi "passed through ... Accepted" (brief:1209-1211) | plan vs code (the spec the gates read) | The brief no longer describes the code on four points. (1) `list.rs` has eleven `pub` items, not four. The extra seven are `COLUMNS`, `NO_VALUE`, `json_text`, `profile_name`, `optional_text`, `health_word` and `hold_word`, plus the methods `PaneRow::cells` and `SessionSync::text`. (2) The JSON fragments in text mode go through `json_text` (list.rs:279-299), not bare `serde_json`. (3) `text_value` also quotes a stored `-` and any character `{:?}` writes as `\u{..}` (list.rs:251-272, 301-307). (4) So bidi characters are escaped, not passed through. All four go the way round 1's W-8 asked, in the same file and layer, and handoff-F.md lists them as Deviations 1-3. They are not drift. But the gate's prompt holds only the brief, the diff and `evidence.md`, and S audits against the brief. So round 2 will read the same contradictions; gate findings B-2 and B-3 came from them. | This cycle: F adds one `evidence.md` entry per point, with the triage facts below (file:line and the rustc check). If the operator wants the letter of the brief instead, amend Decisions 2, 6 and 11 and the Risks bullet. A brief edit needs a fresh run, so evidence is the cheaper fix for this run. |
| W-10 | warn (new, low) | `profile_name(&ProfileOpt)` (list.rs:132-136), used by get.rs:18-21 and watch.rs:17 | layering; duplication across stories | `profile_name` is the typed guard of a shared flag group, but it lives in a verb file. The repo's home for such a guard is `args.rs`, next to `SpecFlags::validate` (args.rs:110-140). `args.rs` is frozen, though (#670, epic ruling 2). The sibling stories type `--profile` inline: #647's `doctor.rs:48-53` (code, unmerged) and #644's plan (644-brief.md:1577). It is one line, so the copies are cheap. But a later `--profile` rule would have several homes. | Keep it. Name `pane::list::profile_name` as the one guard until a `ProfileOpt::validate` lands in `args.rs` through the frozen-file amend channel, and have #644 to #647 use it. |
| W-11 | warn (new) | `json_text` (list.rs:279-299); JSON mode, `write_envelope` (output.rs:302-314, #660) | cross-cutting (terminal safety); abstraction level | JSON mode writes DEL, the C1 controls (U+009B is a one-character CSI on some terminals) and bidi characters raw, because `write_envelope` is plain `serde_json::to_string`. F saw this on the fakes (handoff-F.md, "Known issues"). `json_text` fixes it only for the JSON fragments in text mode. That is a verb-local fix for a concern whose choke point, for every verb, is `write_envelope`. | O raises it on #660: one terminal-safe JSON encoding in `write_envelope`, built from `json_text`'s and `acts_on_terminal`'s logic. `json_text` then calls it instead of being a second encoder. It is outside this blast radius. |
| W-1 | warn (open; still the most consequential) | Decision 3, `SessionSync::of` (list.rs:211-221); AC 3 | contract shape across layers (one rule, two readings) | One side is now code. #647's F commit (9500955, unmerged) compares SHOWN with `session_of_record` (`reconcile/observe.rs:224-262`, finding `shown-driven-mismatch`). Its `record` writes only `last_observed.shown` (observe.rs:354-372). #644's re-amended plan writes `driven: None` and says writing it "would need an amend-first change" (644-brief.md:121, 2077-2079). So nothing writes `last_observed.driven` before #649. #643's SYNC would then read `-` on every real pane while `pane doctor` reports mismatches. Every verb runs over `Unwired` until #649 (brief C7), so the deadline is #649, not this merge. ADR-0021 still supports both readings: I2 (:161), `last_observed` "written by reconcile, never inferred" (:45), and §9's "SHOWN differing from DRIVEN" (:344). With no dominant reading, this stays a warn. | The MO decides before #649. Either #649 writes `last_observed.driven`, or `SessionSync::of` switches to SHOWN vs `session_of_record` (one match, plus AC 3's tests and one help sentence). Record the rule once, in ADR-0021 §1. |
| W-2 | warn (open) | `text_value`, `json_text`, `acts_on_terminal` (list.rs:251-307) | duplication across stories (terminal safety) | One side is now code. #647 adds `pub fn quoted` (`error::excerpt`: `{:?}`, cut to 64) and `embedded` (controls only) in `holler-pane/src/findings.rs:328-352`, unmerged. `pane doctor` and `pane get` would then quote the same session id two ways, and `quoted` does not escape bidi characters. #662's plan has a third style (round 1). | The MO names one helper before the second of #643, #647 and #662 merges. If both crates need it, it belongs in `holler-pane`, which both depend on, added amend-first. |
| W-3 | warn (open) | the `Rig` (tests/pane_verbs/list.rs:39-98) | duplication (test harness) | #647's `tests/pane_verbs/doctor/rig.rs` (409 lines, unmerged) sits in the same test binary. It does a different job: it builds a live world through the ports, while #643's `Rig` is store-only. They overlap only in composing the six fakes into a `Ports`. | No change here. Follow-up: one `Ports`-over-fakes builder in `tests/verb_harness/`. |
| W-4 | warn (open; last cheap moment) | `PaneChange.pane` (watch.rs:95-103) vs `PaneDetail.pane` (get.rs:64-73) | contract shape; naming | Unchanged, and AC 3, 5 and 13 now pin it. `data.pane` is a `PaneRow` in `watch` but the full record in `get` and in the port's `PaneEvent`. After merge, renaming it breaks the JSON contract (ADR-0021:408-409). | The MO decides now or accepts it: rename it `row` in `PaneChange` (three lines plus the tests), or keep it. |
| W-5, W-6 | warn (open, unchanged) | Decision 7's empty stream and AC 16; Decision 16 and C5 | test-kit contract; ADRs | As in round 1. An empty `watch --until-idle` stream fails `check_ndjson` (`EmptyStream`), and I5 is read per port call for `watch`, with no ADR line. | As in round 1: a test-kit follow-up that names #649, and one ADR-0021 §12 line through the amend channel. |
| W-7, W-8 | closed | | | W-7: T landed only the fields and one-line docs, and left `run` untouched (decisions.md, T entry). W-8: adopted (`acts_on_terminal`, list.rs:305-307). | |

## Diff gate, round 1, triaged

For F, T and the round-2 gate. "No change" means no production change is needed.

| Gate finding | Verdict here | Why (evidence) |
|---|---|---|
| B-1, `{:?}` in the `what` of `pane-not-in-profile` (get.rs:104-106) | not a defect; no change | This is the established form. It is byte-identical to the only other construction, `FakeProfileScope::resolve` (`crates/holler-pane-testkit/src/profile_scope.rs:122-123`), and to what #663's real scope plans (663-brief.md:1657). A profile name can hold spaces ("Demo Alpha", `fake_profile_scope_test.rs:470-471`), so the quotes carry meaning. Changing the form in the verb alone would make one code read two ways. |
| B-2, a stored `-` prints as `"-"` | not a defect; no change | The quotes keep a stored `-` apart from the empty value `-`, as both help texts say (list.rs:50-52, get.rs:37-39). No profile or pane name can be `-`. `slugify("-")` is empty, which `ProfileName::parse` refuses (profile.rs:50), and a pane name must start and end with `[0-9a-z]` (vocab.rs:209). The brief's Decision 11 leaves `-` out (W-9). |
| B-3, `acts_on_terminal` is "heuristic" (list.rs:305-307) | not a defect; no change | Every character the gate says is missed is caught. A scratch program on rustc 1.98.1 used the predicate as written. `is_control` flags NUL, TAB, DEL, NEL and U+009B. `escape_debug` flags ZWSP, ZWJ, LRM, RLO, LRI, PDI, U+061C, BOM, U+2028, NBSP, the soft hyphen, combining marks and VS16. `é`, `naïve` and CJK text pass unchanged. The predicate reads the same table `{:?}` uses, so whatever it flags, `text_value` escapes. T-green pinned C1 and bidi (`text_output_escapes_c1_and_bidi_characters`). |
| B-4 | no defect (the gate's own words) | |
| NV-6, and the gate's W-1, "over-escapes `é`" | false | The same check: `é`, `naïve` and CJK text are unchanged. |
| The gate's W-3, "a pane named `-`" | false | Not a valid `PaneName` (vocab.rs:209). |
| NV-1, and the gate's W-2, `json_text`'s `unwrap_or_default` | cannot be reached; no change | `Argv`, `Vec<String>` and `ProfileSpec` hold no map and no fallible `Serialize` (profile.rs:178-200). `emit`'s text closure cannot return an error, so `output.rs`'s encode-failure path (302-314) cannot be used from here. |
| NV-5, `self.watch.next()?` (watch.rs:161) | cannot be reached today; no change | Both existing `Watch` iterators return `None` only after an error has ended the stream: the test kit's (`feed.rs:248-286`) and the hub's (`panes/store.rs:296-326`). `emit_stream` stops at that error. #649's client must keep this rule. |
| NV-2, column width in `char`s | as specified; no change | Decision 4 says so. A wide cell misaligns but stays one cell. |
| The gate's W-4, W-5 and NITs | test-level | T's call. |

## Checked and consistent with existing patterns (no finding)

- **Layer and seams.** Unchanged from the plan. The code is in `holler-cli` only, in the three verb files. The only port
  calls are `PaneStore::{get,list,watch}`, `ProfileStore::get` and `ProfileScope::resolve`, and AC 17's grep prints
  nothing. Every result goes through `emit`, `emit_stream` or `emit_error`. The verbs raise only the closed `PaneNotFound`
  and `PaneNotInProfile`. No frozen file, manifest, ADR-0021, hub, proto or golden file is touched.
- **Dependency direction.** It runs one way: `get.rs` and `watch.rs` import from `list.rs`, which imports neither, and
  nothing imports `get.rs` or `watch.rs` (Decision 2).
- **One table definition.** `COLUMNS` and `PaneRow::cells()` build both the `list` table and the `watch` line, so the two
  cannot drift, and #648's roster can call `cells()`. The roster's own `render_table` (roster_cmd.rs:65-95) is a
  fixed-width renderer over JSON values, so this is not a copy of it.
- **Words by exhaustive `match`.** Role, harness kind, health and hold are matched exhaustively, so a new variant is a
  build error, not a stale word. That is the same guarantee as `args.rs`'s parse by serde names (args.rs:142-150).
- **Scope.** `get --profile` uses the pane that `resolve` returned, with no second read. `watch --profile` compares
  membership by slug (watch.rs:222-226), as the hub and the fake scope do.
- **Test helpers.** `ok_envelope` and `ok_stream` build on the kit's `check_envelope` and `check_ndjson`, as the ACs
  require. They do not copy `verb_harness::one_envelope`.
- **Size and hygiene.** Production files are 316, 275 and 227 lines; the test files are 536, 280 and 311. All are under
  800. There is no new dependency, no `unsafe`, nothing persisted and nothing logged, and only neutral names are used.

## Notes for O

1. **No re-plan is needed.** F's `archChanged` covers the seven extra `pub` helpers, all in the file and layer that
   Decision 2 chose.
2. **The gate's findings have only one carrier.** When F reports `archChanged`, the driver skips its rework classifier, so
   no rework note brings the gate's findings to T-red or F. The round-1 result is gitignored and sits outside
   `docs/handoffs/643/`. This handoff is the only thing that carries it.
3. **For F, this cycle.** Make no production change for B-1 to B-3. Append the triage facts above to `evidence.md` (file
   and line, plus the rustc check), and add one entry per W-9 point under "Deviations from the brief". The gate reads
   `evidence.md`. When the prompt lacks the source to check a claim, the gate has to file it as needs-verification, not
   as a BLOCK.
4. **For T-red.** These findings add no behaviour, so nothing can be RED this cycle. A test that pins current behaviour
   passes at once, and here that is expected, not an invalid RED. Examples: a stored `-` prints `"-"`, `é` is unchanged,
   and the `pane-not-in-profile` text equals the scope's.
5. **For the MO.** W-1 (deadline #649); W-2 and W-10 (one helper each, before the second sibling merges); W-3; W-4 (the
   last cheap moment); W-11 (#660).

## Patterns referenced

- `crates/holler-cli/src/output.rs`, `crates/holler-cli/src/pane/args.rs` and `crates/holler-cli/src/roster_cmd.rs`.
- `crates/holler-pane-testkit/src/{profile_scope,feed}.rs` and `crates/holler-hub/src/panes/store.rs` (the `what` form,
  and how each `Watch` ends).
- `crates/holler-pane/src/{pane,profile}.rs` and `crates/holler-proto/src/vocab.rs` (the name grammars).
- `docs/adr/ADR-0021.md` §§1, 3, 9 and 12.
- The parallel worktrees, as read at about 18:05 MDT: `.claude/worktrees/0647-reconcile-doctor` (code at 9500955) and
  `.claude/worktrees/0644-launch-relaunch` (brief at 7195993). Both are unmerged; they are evidence of plans, not merged
  code.
