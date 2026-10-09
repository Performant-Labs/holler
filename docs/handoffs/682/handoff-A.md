# Handoff-A: Phase 3 - #682 part 1, `FakeProfileStore` and the `ProfileStore` conformance suite  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-682-implementation (at f3e8781, on origin/main e410e9d)
**Brief reviewed:** docs/handoffs/682-brief.md   **Reuse map:** docs/handoffs/682-brief.md, sections "Evidence", "Reuse refactors" and "Extend vs new" (this run has no separate survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS. The plan fills slice a's stubs by extending what slice a built for this purpose: the fault switch, the generic change
feed (whose own doc already names `ProfileEvent` as its second user, `feed.rs:6-8`), the case runner and the watch helpers.
`Writer` and `lock` move into `feed.rs` so the two fakes share one of each, no manifest, `lib.rs` or `conformance/mod.rs`
changes, and the test kit still depends on `holler-pane` alone. All six findings are `warn`: two log rules #661 inherits
(W-1), a sibling-module import the no-edit rule forces (W-2), three items for F (W-3 to W-5), and a PR that would close #682
before part 2 is built (W-6).

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn | Decisions 4 and 5; cases 11, 14 and 15; AC4 `the_clock_stamps_created_updated_and_at` | ADRs | The suite fixes two log rules that ADR-0021 does not state, and #661 must pass them (its acceptance is "passes the `ProfileStore` conformance cases from #638"). **(a)** Decision 4: a `Deleted` entry carries the deleted generation + 1. The frozen field doc says `generation` is "the generation the profile had after the write" (`holler-pane/src/profile.rs:298`), and ADR-0021:268 says "a record that does not exist is at generation 0". Read together, those give 0 for a delete. The brief reads "each applied write adds one" (ADR-0021:269) instead. Both readings are defensible, so this is not a block. Under g + 1, `Deleted` is the one entry kind whose generation is not what the next write must name: a re-create names 0, not g + 1. Under 0, every entry's generation is what the next write names. **(b)** Decision 5: one log per slug, continuing across a delete and a re-create. | Settle (a) before T writes cases 11 and 14 and AC4. Keep g + 1, or switch to 0, which changes only those three expectations and one line of the fake. Either way, state both rules and their reasons in the module doc of `conformance/profile_store.rs`, which is the contract #661 reads, and record the choice in decisions.md. No ADR edit in this run: the ADR is silent on this, not contradicted, and it leaves the registry's log layout to #661 (ADR-0021 §7). #661 can add a line to §7 if it wants the rule ratified. |
| W-2 | warn | "Reuse refactors": five helpers in `conformance/pane_store.rs` become `pub(super)`, four of them generic over `crate::feed::Change` | dependency direction | The profile suite and its `watch` child will import from the sibling pane suite (`profile_name`, `expect_change`, `changes`, `cursors`, `increasing`), and the pane suite gets its first import from `crate::feed`. The shared seam for suite helpers is `conformance/mod.rs`, which already holds `run_cases`, `drain` and `next_item`. The brief justifies the placement in writing: issue #682 keeps `mod.rs` out of every slice's diff. The `Change` bound keeps the suite neutral, because the trait is implemented on the port's event types and not on any store, so the hub's `ProfileEvent`s go through the same impl. Accepted, not drift. | Build as planned, with each shared helper's doc saying the profile suite uses it (the brief already requires this). O opens a small follow-up to move the five helpers into `conformance/mod.rs` once slices b, d and e have merged and the no-edit rule has done its job, so that a third suite does not also import from `pane_store`. |
| W-3 | warn | `Writer` moves from `pane_store.rs:81-100` into `feed.rs` | naming, cohesion | `feed.rs` is the right home: it is the one module both fakes already share, and leaving `Writer` in `pane_store.rs` would make one fake import from the other. But `Writer`'s doc names "the membership rule" (`pane_store.rs:84-88`), a pane-only rule, and `feed.rs`'s module doc (lines 1-8) describes only the feed. | Generalize `Writer`'s doc to "the store's port-only rule (membership for panes, the name rule for profiles)". Add a line to `feed.rs`'s module doc naming `Writer` and `lock` as the write-side helpers the fake stores share. `lib.rs`'s one-line summary of `feed` goes slightly stale, which is acceptable under the no-edit rule. |
| W-4 | warn | `FakeProfileStore.history: Mutex<…>`, documented as "Locked only inside a `feed.write` (feed lock first)" | concurrency, naming | A second lock beside the feed is right, because `ProfileEvent` is frozen and cannot carry the log. But the stated rule does not fit `log()`, a read that takes this lock outside any `feed.write`. The rule that actually prevents a deadlock is "never take the feed lock while holding this one". Separately, `history` already names the feed's own event list (`feed::Log::history`, `feed.rs:61`). In the write path the feed's `Log` is also in scope, so two different "history" fields there make the code harder to read. | Document the lock order as: the feed lock first, then this one, never the reverse. `log()` may take this lock alone or inside `feed.read`. Name the field from the port's vocabulary, for example `change_logs`. The field is private, so no test or AC changes. |
| W-5 | warn | `fixture.rs`: `sample_spec` with "the sample pane's model and context", cwd `/srv/demo` | duplication | `sample_pane` keeps the model, the context ceilings and the cwd as inline literals (`fixture.rs:37` and `55-64`). If `sample_spec` restates them, the crate has two fixtures that are meant to agree but can drift apart. That matters because #662's `profile_snapshot` maps a pane to a spec. | Add private helpers (`sample_model()`, `sample_context()` and a `SAMPLE_CWD` const) and have both `sample_pane` and `sample_spec` call them. Reuse `SCRATCH` for the workspace. The change is behaviour-neutral for `sample_pane`, and AC5 guards it. Update `fixture.rs`'s module doc to name the new fixtures. A checks this at Phase 7. |
| W-6 | warn | Brief line 35: "This PR says 'Part of #682', not 'Closes #682'. See 'Needs operator'." | process | The brief has no "Needs operator" section. The Workflow script opens the PR with the body `Closes #682.` (`$WORKFLOW_ROOT/workflow/coding-pipeline.workflow.mjs:4822`). Merged as is, that closes #682 while part 2 (`FakeProfileScope`, which #663 needs) is still unbuilt. | O adds the missing section or removes the pointer, and records the split in decisions.md. After the script opens the PR, the run's agent changes `Closes #682.` to `Part of #682.` with `gh pr edit`, in the same edit that adds the CONTRIBUTING.md AI disclosure. After the merge, it checks that #682 is still open. |

Apart from these, the plan is consistent with existing patterns. I verified:

- **Naming mirrors slice a one for one:** `FakeProfileStore`, `ProfileStoreOp`, `run_profile_store_conformance`,
  `profile_store_cases` and `sample_profile`. The op strings are `profile_store.<method>`, the case ids run parallel to the
  pane suite's, and the mutants follow `tests/pane_store_conformance_test.rs:153-263`.
- **The layout follows the workspace's dominant pattern.** `conformance/profile_store.rs` plus
  `conformance/profile_store/watch.rs` is the `foo.rs` + `foo/` layout used by `lockout`, `holds` and `circuit` in
  `holler-hub`, and in `holler-body` and `holler-proto`. As a child module, `watch.rs` reaches its parent's private helpers
  without widening them.
- **The suite stays neutral on how #661 files records.** It compares events by `name.slug()` and sorts `list` before
  comparing. The hub's `RegistryEntry::name()` returns `&str`, so #661 may file by display name, and no case has two names
  with one slug in a single event stream.
- **Every refusal is a closed `PaneError` variant.** `next_generation` is the only compare-and-swap rule and `EnvVarName` the
  only env guard, and AC7's greps enforce one `enum Writer` and one `Condvar`.
- **Sizes stay under the limits.** The largest new file is about 480 lines, against the 600-line warn, and the brief names a
  split (`profile_store/log.rs`). The slice-a files it touches stay at or below about 530 lines.
- **No missed reuse candidate.** The hub's `Clock` trait (`holler-hub/src/lockout.rs:572`) is out of reach, because the kit
  cannot depend on the hub, and `holler-pane/src/profile_snapshot.rs` is still an empty stub.

## Notes for O

PASS, so nothing blocks T. Settle W-1 (a) before T writes cases 11 and 14 and AC4, and handle W-6 before the merge. W-3 to
W-5 are for F.

- **Stale citations:** the brief's Evidence cites `src/lib.rs:51-62` (actually 30-41) and `src/fault.rs:81-85` and `103-170`
  (actually 19-23 and 41-114). The quoted content is right, so nothing needs to change.
- **What Phase 7 will check:**
  - AC7's three greps.
  - `fn lock` exists only in `feed.rs` and as `FaultSwitch`'s private method.
  - `pane_store.rs` keeps no `Writer`.
  - `impl Change for ProfileEvent` sits in `profile_store.rs`.
  - The profile suite and `watch.rs` call the shared helpers and keep no local copy of `expect_change`, `changes`,
    `cursors`, `increasing`, `profile_name`, `drain`, `next_item`, `run_cases`, `succeeds`, `expect_code` or `expect_eq`.
  - `sample_spec` and `sample_pane` share their values (W-5).

  Accepted typed duplicates: `open`, `last_cursor`, `put`, `shown`, `unchanged`, `sample` and `revised` over `ProfileStore`
  (a different trait), and the two impl blocks (the brief's "Accepted near-duplicate").

## Patterns referenced

- `crates/holler-pane-testkit/src/{feed.rs, fault.rs, pane_store.rs, fixture.rs}`, `src/conformance/{mod.rs, pane_store.rs}`
  and `tests/pane_store_conformance_test.rs` (slice a, at e410e9d)
- `crates/holler-pane/src/profile.rs:202-371` and `src/generation.rs` (the frozen port and the compare-and-swap rule)
- `docs/adr/ADR-0021.md` §5 (lines 184-192), §7 (237-262), §8 (264-302), and "Decisions taken" items 1 and 4
- `crates/holler-hub/src/panes/{mod.rs, feed.rs}`, `src/profile/mod.rs`, and issue #661 (the suite's other consumer)
- `$WORKFLOW_ROOT/workflow/coding-pipeline.workflow.mjs:4815-4824` (the PR body the script writes)
