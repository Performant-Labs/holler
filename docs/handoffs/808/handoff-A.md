# Handoff-A: #808 - A Herdr snapshot pane carries its label  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-808-implementation` (base `main` at `92fbc6e`; the branch's three commits touch only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/808-brief.md` (as amended in round 2; outside brief gate round 3: PASS)   **Reuse map:** the brief's own survey, sections E1 to E15 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS: 0 `block`, 7 `warn`. The plan extends objects that already exist and builds no parallel ones:

- The label is an optional field on the contract's `HerdrPane`, in the record format's established optional-field form.
- The adapter reads the label from `PaneInfo` objects in replies it already receives, so no method is added and neither the allow-list nor I5 changes.
- `FakeHerdr` and `WireHerdr` get the label hook in the same shape as their existing outside-world hooks.
- O2's whole-value `contains` becomes an explicit four-field comparison beside `same_pane`.

I re-checked the survey's load-bearing claims in source:

- O2 (`tx_launch.rs:685`) is the only whole-value `HerdrPane` comparison in production code.
- `Plan::record` (`tx_launch.rs:327`) is the only production writer of a record's `herdr`.
- Neither the hub nor `holler-proto` references the type, and no golden file or `docs/protocol/v2.md` describes its JSON.
- `holler-cli/src` only reads `HerdrPane` fields and never destructures one, so it needs no change.
- `serve` already takes an `Arc<WireHerdr>`, so E12's hook can reach the wire fake.

Clarification 6's choice (a field on `HerdrPane`, not a separate snapshot-entry type) therefore rests on true evidence, and I do not reopen it.

The most important warn: the plan has the adapter's `ensure_pane` report the label but does not require the same of `FakeHerdr`'s, and the new conformance case checks only `snapshot()`. The two implementations of the port can therefore disagree, and E9's relaunch test can pass without exercising `Plan::record`'s clearing (finding 1).

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Scope item 3, E9, E12: `FakeHerdr` and the label conformance case | pattern consistency | The amended contract has `ensure_pane` report the label, but the plan requires it only of the adapter. `FakeHerdr`'s `ensure_pane` answers from an id-only `place()`, and the conformance case checks `snapshot()` only. | `FakeHerdr` builds its `ensure_pane` answer and its snapshot entries with one private constructor. The label case also runs `ensure_pane` on the labelled cell and expects the snapshot's entry, label included. |
| 2 | warn | Scope item 4 vs clarification 5: the ADR-0021 edits | ADR | Section 1's records table lists `herdr` as `HerdrPane { session, workspace, pane_id, grid: GridPos }` under "Type (merged)". Clarification 5's four places leave that row out, so after merge it misstates the merged type. | Add `label?` to that row, with a note that the label is observed and never recorded (the #700 precedent). Say in the PR body that this is the one ADR edit beyond clarification 5's list. |
| 3 | warn | E10 item 1: the parsers read `label` | pattern consistency | Three new inline optional reads, each against the parser rule in `protocol.rs:17-19` (a missing or wrong-typed field is `unavailable`). `Object` has no optional accessor. | Read the label through one helper whose doc states the exception, and name it in the module's Parsers bullet. Do not name the `SessionState` lookup `label_of`. |
| 4 | warn | Acceptance 1, the fake test: the `""` → `None` mapping in `FakeHerdr` | naming / anti-duplication | `FakeHerdr` cannot call `label_of`, so it needs its own copy of the rule. The project's established form for such a copy is a private copy cross-referenced from the adapter's doc (`last_lines`). | Put the fake's copy in one private place, and have `label_of`'s doc name it the way `last_lines`'s doc names its copy. Add the label hook to `FakeHerdr`'s list of outside-world hooks. |
| 5 | warn | Clarification 3, E12: the suite's first optional hook | pattern consistency | The suites' stated policy is that events which cannot be caused through the port are not driven by the suite. The label case is a deliberate exception, and with `None` it "is not run". | Document the exception and the skip in the Herdr suite's module doc and its runner example. Keep the one `CASES` table, with `herdr_cases()` still listing the label case. |
| 6 | warn | E9, E11, E14, clarification 4: where new code and tests go | file structure / size | `tx_launch.rs` is 787 lines and `launch/rig.rs` is 718. Four more test files that gain tests are already past lint's 600-line warning. The brief names no file for the record-format tests. | Build the E11 double in the shape of the rig's own per-run wrappers, in a sibling module if the rig would pass about 800 lines. Put the serde round-trips in `records_test.rs` and keep clarification 4's literal in one file. |
| 7 | warn | Clarification 6, E14: one type that is both persisted and observed | cross-cutting (persistence) | "A record never carries a label" holds only at `Plan::record`. The stores accept a labelled record. The derived `PartialEq` makes any future `==` between a record and a snapshot entry label-sensitive. | Accepted here: the issue rules out a hub change. Follow-up for #650: when the importer becomes the second writer, move the clearing and the four-field comparison to one named place in `holler-pane`. |

### Details

**1. `FakeHerdr`'s `ensure_pane` and the label conformance case.**

- `FakeHerdr::ensure_pane` builds its answer from the spec and the id that `Workspace::place` returns (`crates/holler-pane-testkit/src/herdr.rs:246-257`). For an occupied cell, `place` returns only the occupant's id (`:402-411`). So unless the plan says otherwise, the fake answers `label: None` for a labelled pane while its `snapshot()` reports `Some("x")`.
- That breaks a property the suite already pins. Cases 3 and 4 require `ensure_pane`'s answer for an existing pane to equal that pane's snapshot entry as a whole value (`crates/holler-pane-testkit/src/conformance/herdr.rs:217-244`). Those cases label nothing, and the new case checks only `snapshot()`, so no case would notice.
- **The consequence:** E9's acceptance test runs over the launch rig, which uses `FakeHerdr`. That test is "a relaunch in place of a pane whose `ensure_pane` answer carries a label writes a record with `herdr.label: None`". If the fake's answer carries no label, the test passes whether or not `Plan::record` clears the label, so the guard on constraints (a) and (c) goes untested. In production the adapter does answer the label (the setup wizard labels panes). A record that kept the label would put a `label` key in `panes.json`, and a build without the field refuses to load that file (`crates/holler-hub/src/panes/persist.rs:30-37`).
- **Fix for T:** in the label case, run `ensure_pane` on the labelled cell (and on the `""` cell) and expect the `HerdrPane` the snapshot lists, label included. F then cannot satisfy the suite with a fake that answers `None`.
- **Fix for F:** give `FakeHerdr` one private constructor that turns a `LivePane` into a `HerdrPane`, used by both `ensure_pane` and `listed()`. That is clarification 2's "the two can never disagree" applied to the fake.
- This stays within scope items 2 and 3. No brief amendment is needed.

**2. ADR-0021 section 1's records table.**

- `docs/adr/ADR-0021.md:39` lists `herdr` as `HerdrPane { session, workspace, pane_id, grid: GridPos }` in the "Type (merged)" column.
- Scope item 4 names "sections 1 and 2 (the `HerdrPane`/`HerdrSnapshot` types ...)". Clarification 5 lists four places and says "the PR edits exactly these places", and none of the four is in section 1. Followed to the letter, it leaves section 1 misstating the merged type while the new section-2 sentence names `HerdrPane.label`.
- **Precedent:** #700 (`c6485b0`) added `Pane.opencode_agent` with "one dated row in ADR-0021 section 1". The `host` row already lists its optional `herdr_api_version?` and says who writes it.
- **Fix:** F changes the cell to `HerdrPane { session, workspace, pane_id, grid: GridPos, label? }` and adds to its Notes: "`label` is observed, never recorded: launch and relaunch write none (section 2; amended 2026-10-11, #808)."
  - This adds no prose to the four places clarification 5 pins. It keeps a ratified table true, which the stack rule requires (an ADR is updated in the same change).
  - Name it in the PR body as the one ADR edit beyond clarification 5's list.
- **Merge note:** #802 is still open and edits the same ADR. If it lands first and takes "Decisions taken" item 10, renumber this item when merging `main` in before the squash.

**3. Reading the label in `protocol.rs`.**

- The module's parser rule is to refuse "a field it reads that is missing or of the wrong type, as `unavailable`" (`crates/holler-adapter-herdr/src/protocol.rs:17-19`).
- `Object` has only required-field accessors: `get`, `string`, `object` and `list` (`:532-558`). The one optional read today, `parse_pong`'s `protocol`, reads `.fields` inline and documents its exception at the function (`:305-317`).
- E10 item 1 adds three more such reads, in `pane_ref`, `parse_pane_info` and `parse_workspace_created`.
- **Fix:** read the label through one private helper, either an `Object` method beside `string` (for example `optional_string`) or a `pane_label(Object<'_>) -> Option<String>`. Its doc states the exception once, and the Parsers bullet names it so the module's stated rule stays true.
- Do not reuse `pane_ref` for the split and create replies. It requires `workspace_id` and `tab_id`, which would make those two parsers stricter than they are today.
- Give the lookup that E10 item 3 suggests a name other than `label_of`, which clarification 2 reserves for the coercion. For example `SessionState::pane_label`, beside `SessionState::workspace` (`:358-384`).

**4. The `""` → `None` mapping in `FakeHerdr`.**

- Acceptance 1's fake test needs `FakeHerdr` to map `""` to `None`. The test kit does not depend on the adapter, so the fake cannot call `label_of` and needs its own copy of the rule.
- The codebase's established form for such a copy is a private copy, cross-referenced from the adapter's doc. `last_lines` exists in both `protocol.rs:589-595` and `herdr.rs:520-525`, and the adapter's doc says: "The test kit's `FakeHerdr` reads a screen the same way; its copy is private and dev-only, so it is not shared."
- **Fix:** the fake's copy lives in one place (finding 1's constructor), and `label_of`'s doc names it. `FakeHerdr`'s struct doc lists its outside-world hooks (`herdr.rs:129-132`, "`vanish` and `print` ... bypass the faults and the call log"); add the label hook to that list.
- This also keeps the copy from reading as a near-copy at the Phase 7 gate.

**5. The suite's first optional hook.**

- The suites' stated policy, in `conformance/harness.rs:24-26`, is that events which "cannot be caused through the port ... the suite does not drive them; the fake's own tests pin them". A label cannot be set through `HerdrPort`, so the label case, driven by a hook the implementation supplies, is a deliberate exception (clarification 3). E6 is right that no optional hook exists today.
- `conformance/mod.rs:7-8` says each suite's table and its `*_cases()` list "cannot drift". The brief says that with `None` the case "is not run".
- E12's repacking of `(fixture, guard)` into the subject is consistent with existing code: `run_harness_conformance` already hands `run_cases` the pair `((harness, rig), guard)` (`conformance/harness.rs:147-154`).
- **Fix:** list the label rule in the Herdr suite's module doc (`conformance/herdr.rs:1-24`), and say there that the case needs the hook and is skipped without one (the scratch run against real Herdr). Show the new parameter in `run_herdr_conformance`'s doc example (`:130-139`). Keep the one `CASES` table: do the skip in the runner or in the case, and keep `herdr_cases()` listing the case.

**6. Sizes, and where new code and tests go.**

- `scripts/lint.sh` warns at 600 lines and fails at 900.
- **`crates/holler-pane/src/tx_launch.rs`:** 787 lines today, about 795 after the four-field helper and the `Plan::record` line. That fits, but nothing else should land there.
- **`crates/holler-cli/tests/pane_verbs/launch/rig.rs`:** 718 lines. Build the E11 double per run in `with_ports`, the way the rig builds its own `LinkedHost` and `HookedHarness` (`rig.rs:265-288` and `:594-704`). That is the rig's established pattern, not a parallel path. `HookedHarness` is about 74 lines, so a seven-method `HerdrPort` plus its arming state would bring the rig to roughly 790-800 lines. If it would pass about 800, put the double in a sibling module of `launch/`, beside `guards.rs` and `profiles.rs`.
- **Test files already past the 600-line warning:** `launch.rs` (607), `adapter_test.rs` (645), `wire_herdr/mod.rs` (654) and `protocol_test.rs` (679). `relaunch.rs` (582) is close to it. None should reach 900 with the planned additions.
- **The record serde forms have an established home**, `crates/holler-pane/tests/records_test.rs` (#637 AC 4). #700 added `pane_opencode_agent_round_trips_set_and_absent` there (`:173-199`). The three forms are: a pre-change record reads; a record with `"label":"x"` reads; a `None` label writes no key.
- **The launch-engine byte test belongs with the launch tests.** They already drive the engine with their own `TxOptions` (`launch.rs:541-543`).
- Clarification 4's two tests share one literal, so keep it in a single file.

**7. The record invariant is held at the writer (follow-up for #650).**

- `HerdrPane` is both the persisted `Pane.herdr` (`pane.rs:232`) and the snapshot entry (`ports.rs:100`). After this change, "a record's `herdr.label` is `None`" holds only because `Plan::record` clears it (E14). I confirmed `tx_launch.rs:327` is the only production writer.
- The stores accept a labelled record. The hub's `cas_put` has a refusal seam, `refuse_profile_move` (`crates/holler-hub/src/panes/store.rs:170-187`), but it checks nothing about Herdr.
- The derived `PartialEq` also makes any future `==` or `contains` between a record's pane and a snapshot entry label-sensitive. That is the defect class E11 fixes in O2.
- Clarification 6 and the issue's "no hub change" accept this for #808.
- **Follow-up for #650:** the importer (`crates/holler-pane/src/import.rs`, a stub today) will be the second writer to build a record from a snapshot entry. When it lands, move the label clearing and the four-field comparison to one named place in `holler-pane` and share them, instead of writing them a second time. A store-side refusal of a labelled record is a possible later hardening, but it is a hub change and outside #808.

### The outside gate's round-3 items, checked in source

- **NV-1:** nothing in the adapter's `src` reads `SessionState.panes`. `grep` finds only the type and its parser (`protocol.rs:345`, `:355`, `:439-440`).
- **NV-2:** `tx_launch.rs:327` is the only production writer of a record's `herdr`.
- **NV-3:** `GuardWatcher` and `Mutant` each hold an `inner: FakeHerdr` (`herdr_conformance_test.rs:91`, `:202`).
- **NV-4:** quoted as shown above (`protocol.rs:305-317`, `:541-544`).
- **NV-5:** the adapter's `src` has no `#[cfg(test)]`, and `protocol` is `pub mod` (`lib.rs:40`).
- **W-1:** a pane id is `w<N>:p<M>`, which embeds the workspace and is never reused (spike lines 44 and 124). So the join by pane id inside one `session.snapshot` read is unambiguous.
- **W-2:** E10 item 4 already applies the `label: None` rule to `ensure_pane`'s answer for an occupied cell.

## Notes for O

None required for a PASS. Finding 7 is input for #650's brief. Finding 2 names the one ADR edit beyond clarification 5's list, which the PR body should state.

## Patterns referenced

1. `crates/holler-pane/src/pane.rs:4-9` and `:112-115`: the optional record field form (`#[serde(default, skip_serializing_if = "Option::is_none")]`, `HostInfo.herdr_api_version`).
2. `docs/adr/ADR-0021.md:35-52`, with commit `c6485b0` (#700): section 1's records table is updated when a record type gains a field.
3. `crates/holler-adapter-herdr/src/protocol.rs:17-19`, `:305-317`, `:358-384`, `:532-558` and `:589-595`: the parser rule, the one optional read, the `SessionState` lookup, the `Object` accessors and the documented copy shared with the fake.
4. `crates/holler-pane-testkit/src/conformance/{herdr.rs,harness.rs,mod.rs}` and `crates/holler-pane-testkit/src/herdr.rs:129-132`, `:223-236`: the suites' shape, the harness runner's repacking, the policy on events the port cannot cause, and the fake's outside-world hooks.
5. `crates/holler-cli/tests/pane_verbs/launch/rig.rs:265-288` and `:594-704`: the rig's per-run port wrappers, which E11's Herdr double follows.
