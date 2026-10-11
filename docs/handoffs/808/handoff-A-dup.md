# Handoff-A-dup: #808 - A Herdr snapshot pane carries its label  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** `issue-808-implementation`
**Diff base:** `92fbc6e4186acc717f4aa2b6465c55a266b9043c` (`main` at the branch's base)   **Diff head:** `797b5bbd29a195da9b19ac54cdb35c911f5705ff` (T-green)
**Reuse map:** the brief's survey, `docs/handoffs/808-brief.md` E1-E15 (this run has no separate `survey.md`), as reviewed in `docs/handoffs/808/handoff-A.md`
**Verdict:** PASS

## Summary

PASS: 0 `block`, 2 `warn`. F extended every object the map named and built no parallel path.

- **The contract type.** The label is an optional field on `HerdrPane`, not a new snapshot-entry type (clarification 6).
- **The adapter.** It extends `PaneRef`, the two `PaneInfo` parsers, `SessionState` (with `pane_label`, beside `workspace`) and `WorkspaceGrid`.
- **The fake.** `FakeHerdr::rename` sits beside `vanish` and `print`.
- **The suite.** `run_herdr_conformance` keeps its one `CASES` table.
- **O2.** It compares through `same_place`, which reuses `same_pane` beside it.

The diff also removes duplication that was already there:

- The adapter's two `HerdrPane` literals collapse into one constructor, `HerdrAdapter::pane`. It is the only `label_of` call in the adapter.
- The fake's `ensure_pane` now answers its own snapshot entry through `State::listed()`. So each implementation builds a `HerdrPane` in exactly one place.

Each new object has a written justification in the brief or in A's Phase 3 review:

| New object | Justified by |
|---|---|
| `label_of` | clarification 2 |
| `sent_label` | A warn 3 |
| `SessionState::pane_label` | E10 item 3, A warn 3 |
| `same_place` | E11 |
| The test kit's private `label_of` copy | A warn 4, the `last_lines` precedent |
| `DisplacingHerdr` | E11 test 3, A warn 6 |

The two warns concern what comes next:

1. The label clearing and the place comparison are private to `tx_launch.rs`, and #650 does not yet know it must reuse them.
2. `adapter_test.rs` has passed the ~800-line mark, and `tx_launch.rs` is just under it.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/tx_launch.rs:328`, `:756`, `:762` | A's Phase 3 warn 7 is still open, and nothing records it for #650. Two rules exist only in `tx_launch.rs`: "a record never carries a label" (inline in `Plan::record`) and the place comparison (`same_pane`, `same_place`, both private `fn`s). #650's importer joins a name to a snapshot entry by `label == name` and writes a record from that entry, so it is the second writer. #650's issue body (read 2026-10-11) mentions neither rule. | Before #650's brief is written, O adds this to #650 (a comment or brief input): in #650's own PR, move the clearing and the two comparisons to one named place in `holler-pane`. The recommended home is methods on `HerdrPane` in `pane.rs` (281 lines). `Plan::record`, O2 and the importer then share them. This is a refactor-to-extend that the brief names, not drift. |
| 2 | warn | `crates/holler-adapter-herdr/tests/adapter_test.rs` (810 lines), `crates/holler-pane/src/tx_launch.rs` (798 lines) | This stack flags touched files above ~800 lines. `lint.sh` warns on both and fails at 900. A's Phase 3 put `adapter_test.rs` at 645 lines, and #808's four label tests brought it to 810. `tx_launch.rs` grew by 11 lines, as planned. | Put the next adapter tests in a new `tests/*.rs` file, following the `adapter_messages_test.rs` precedent. Move helpers they share (`connect`, `spec`, `pane`, `sent_since`) into `tests/common/mod.rs` rather than copying them a third time. Add nothing more to `tx_launch.rs`. Finding 1's move takes code out of it. |

No duplication; the extension is clean. The two warns are follow-up input, not defects in this diff.

### Details

**1. The record invariant and the place comparison, for #650.**

- **Where the rules live now.**
  - `Plan::record` writes `herdr: HerdrPane { label: None, ..live.herdr }` (`tx_launch.rs:328-331`).
  - O2 checks `listed.iter().any(|pane| same_place(pane, herdr))` (`:690`).
  - `same_place` is `same_pane(a, b) && a.workspace == b.workspace && a.grid == b.grid` (`:762-764`).
  - All three are private to the launch engine.
- **The ADR now states the rule in general terms.** Section 1's `herdr` row says "`label` is observed, never recorded". Section 2 says "no comparison of a record with a snapshot entry looks at it".
- **The risk in #650.** The importer will build a `Pane` from the snapshot entry whose label equals the pane's name. Without a shared helper, its F faces two options, each wrong:
  - Copy the entry whole, `herdr: entry.clone()`. That persists the label: the record's JSON gains a `label` key, and a build without the field refuses to load `panes.json` (`crates/holler-hub/src/panes/persist.rs:30-37`).
  - Write a second clearing and a second comparison. That is a parallel path, which this gate would reject.
- **The fix belongs in #650, not here.** #808's blast radius and A's Phase 3 kept the rules where the brief put them (E11: "beside `same_pane`").

**2. File sizes.**

- `scripts/lint.sh` warns at 600 lines and fails at 900. It lists both files: `adapter_test.rs is 810 lines` and `tx_launch.rs is 798 lines`.
- `adapter_test.rs` keeps about ten file-local helpers (lines 33-129).
- The one adapter test file split out so far, `adapter_messages_test.rs`, re-declares its own `spec` and `config`. The shared `tests/common/mod.rs` already holds `id`, `cell` and the layout builders. A third copy of the connect and spec helpers would be the near-copy to avoid.

### Checked and clean (no finding)

- **The test kit's `label_of` (`crates/holler-pane-testkit/src/herdr.rs:546`)** duplicates `protocol::label_of` (`crates/holler-adapter-herdr/src/protocol.rs:480`). This is deliberate:
  - The test kit cannot depend on the adapter, because the adapter dev-depends on the test kit (`crates/holler-adapter-herdr/Cargo.toml:25`).
  - Clarification 2 puts the rule in `protocol.rs`.
  - A warn 4 named the `last_lines` form: `protocol.rs:630-636` and `herdr.rs:536-541`.
  - The two docs name each other, which is stronger than the precedent.
- **`DisplacingHerdr` (`crates/holler-cli/tests/pane_verbs/launch/displaced.rs:44`)** is the `HerdrPort` double that E11 test 3 called for.
  - The rig builds it per run in `with_ports` (`launch/rig.rs:283-309`), beside `LinkedHost` and `HookedHarness`.
  - It lives in a sibling module (A warn 6), so the rig is 739 lines.
  - No reusable `HerdrPort` wrapper exists in `holler-cli`'s tests. `GuardWatcher` and `Mutant` sit in another crate's tests, and `wiring.rs`'s `Unwired` refuses every call.
- **No remaining whole-value `HerdrPane` comparison in `crates/*/src`.**
  - O2 was the only one, and `grep` finds no `panes.contains` or `herdr ==` left.
  - The other place checks compare a `HerdrSpec` with a record (`tx_launch.rs:390`, `:456`, `:731`), so `same_place` does not duplicate them.
  - `profile_snapshot.rs:51-52` copies only `workspace` and `grid` from a record. The survey (E5) did not list it, and it is unaffected.
- **`sent_label` (`protocol.rs:468`)** reads the field the way `parse_pong` reads its optional `protocol`. It is the one documented exception in the Parsers bullet.
- **The lookup is not duplicated.** `pane_label` (`protocol.rs:394`) is the only reader of `SessionState.panes`. `snapshot()` (`adapter.rs:400`) and `WorkspaceGrid::occupant` (`adapter.rs:434`) both call it: "one lookup serves both" (E10 item 3).
- **The suite still has one table.** The runner filters `CASES` when the hook is `None`, and `herdr_cases()` still lists all 12. The `(fixture, guard)` repacking follows `run_harness_conformance` (`conformance/harness.rs:147-154`). The policy exception is documented in the suite's module doc (A warn 5).
- **Test-side label stores follow their fakes' own patterns.** `WireHerdr` adds a `labels` map beside `screens` and a `rename` beside `split` and `add_tab` (`wire_herdr/mod.rs:66`, `:160`). `Mutant` keeps the raw label itself, which is required: the fake folds `""` to `None`, and two of the label breaks need the raw value.
- **The `LaunchRequest` literal for `LAUNCH`'s spec** now appears three times: `launch.rs:537`, `relaunch.rs:343` and `record_bytes.rs:24`. The rig has no request helper, and the existing engine tests write it inline. The new copy follows the dominant pattern, so this is not drift.
- **No drive-by edits.** Every touched path is in the brief's blast radius, or in E9's and E15's additions to it:
  - Nothing in `holler-hub`, `holler-proto`, `holler-body` or `holler-cli/src` changed.
  - No golden file changed, and `docs/protocol/v2.md` is unchanged.
  - No method joined the Herdr allow-list.
- **The ADR is consistent.** ADR-0021 is the only doc that names `HerdrPane`, and its five edits agree with the code. No doc or test still counts 11 Herdr cases.
- **Public repository.** The added lines carry no personal infrastructure names. The commits' author is the GitHub no-reply address.

### Carried forward (already in `handoff-F.md` and `handoff-T-green.md`)

- **#802 is still OPEN** (checked 2026-10-11). If it merges first, renumber ADR-0021's "Decisions taken" item 10 when `main` is merged in. `main` has moved two wizard commits (#814, #815) past the base, and neither touches a file this branch changes.
- **The PR body must name the section-1 `herdr` row** as the one ADR-0021 edit beyond clarification 5's list.

## Notes for F

None. PASS.

## Patterns referenced

1. `crates/holler-pane/src/tx_launch.rs:756-764`: `same_pane` and `same_place`, the join key and its four-field extension.
2. `crates/holler-adapter-herdr/src/protocol.rs:630-636` and `crates/holler-pane-testkit/src/herdr.rs:536-549`: the documented private copy (`last_lines`, now `label_of`) across the adapter and test kit boundary.
3. `crates/holler-cli/tests/pane_verbs/launch/rig.rs:283-309`: the rig's per-run port wrappers (`LinkedHost`, `HookedHarness`, now `DisplacingHerdr`).
4. `crates/holler-pane-testkit/src/conformance/harness.rs:142-155` and `conformance/mod.rs:1-9`: the runner's repacking precedent and the one-table rule.
5. `crates/holler-pane-testkit/src/herdr.rs:227-249`: the fake's outside-world hooks (`vanish`, `print`, `rename`).
