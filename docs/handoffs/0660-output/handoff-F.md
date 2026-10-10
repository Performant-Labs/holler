# Handoff-F: Phase 5 - Implement

**Date:** 2026-10-09
**Branch / worktree:** `issue-0660-output` / `.claude/worktrees/0660-output` (base `519947a`, branch head `2f122b9`)
**Contract:** `handoff-T-red.md` — VERDICT: GREEN-on-contact (8 new + 1 extended test, all passing against current `output.rs`)

## What I changed

**Nothing in production code. This is a deliberate no-op.**

| File | Change | Why |
|---|---|---|
| `crates/holler-cli/src/output.rs` (my only allowed production file, per the brief's boundaries) | none | No test faults against it. There is no RED to turn green, so any edit would be speculative. |

How I reached that decision:

- **The contract has no RED in it.** T's conformance suite covers all six acceptance bullets: goldens through `check_envelope`, NDJSON through `check_ndjson`, the forced diagnostic in JSON mode, `ALL_CODES` exit parity checked by the checker, the `GridPos` row-first golden, and the #637 signature pins. All of them pass against `output.rs` at `519947a`. Brief §RED policy and decisions.md decision 4 apply: a suite that is green on contact gives F nothing honest to implement.
- **The operator chose a test-only continuation.** decisions.md (~11:05 PM entry) records that decision with F's contract as a no-op. The one RED the plugin recorded at the t-red crossing came from `tests/remote_admin_test.rs`, which predates this run and failed transiently. O could not reproduce it, and `output.rs`, the only file I may change, cannot affect that target. It is not F's contract, and my own run below did not reproduce it either.
- **I did not act on T's advisories:**
  - `one_line()` turns whitespace-only text into `""`, which would break checker rule 12. No real `PaneError` reaches that path.
  - A zero-item `emit_stream` produces an empty stream, which would hit `check_ndjson`'s `EmptyStream`. `pane watch` already prints nothing when nothing is owed.

  Both are outside #660's acceptance bullets and no test pins either one. Changing `output.rs` for them would be exactly the speculative, unpinned production change this role must not make. Making them RED is option (b) in the decisions journal: a scope change that only the operator can approve.

The worktree diff is still T's two test files plus this handoff directory (`git status`: `M output_api.rs`, `M process/stub.rs`, `?? docs/handoffs/0660-output/`). Nothing under `src/**` changed: `git diff 519947a -- crates/holler-cli/src` is empty.

## Self-check

I ran these myself in the run worktree, one cargo process at a time. This is the Rust equivalent of the template's `npm test`:

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 134 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.04s

$ cargo test -p holler-cli --test pane_cli_process
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

$ cargo test -p holler-cli --test pane_verbs -- output_api
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 106 filtered out
  (includes all 8 new tests and the extended
   every_closed_code_exits_by_its_class_with_the_same_code_in_both_formats)

$ cargo test -p holler-cli            # the brief's narrow verification
exit=0 — 59 test binaries, all "test result: ok"; 619 passed, 0 failed
  (this includes remote_admin_test, which passed this time; the earlier flake did not recur)

$ cargo clippy -p holler-cli --test pane_verbs --test pane_cli_process
Finished — zero warnings

$ rustfmt --check --edition 2021 crates/holler-cli/src/output.rs
exit 0 (output.rs is fmt-clean; untouched)
```

I did **not** run the full workspace command (`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`). The brief schedules it for pre-merge, and the plugin runs its own suite at T-green regardless.

As the role requires: the authoritative GREEN is the plugin's exit code at T-green, not this self-check.

## Tests I think are wrong (if any)

None are wrong. I checked each new or extended test against the acceptance bullet it cites, and the assertions match the brief, including ADR-0021 §9's 0/1/2/3 exit codes (decision 3). Three non-blocking notes for T, none of which needs a test repair for this run:

1. **The `Format` `Copy` claim isn't actually checked at compile time.** In `the_fixed_signatures_and_types_compile_unchanged`, the comment says the code proves `Format` is `Copy`, but `let copy = Format::Text; assert_eq!(copy, Format::Text);` would compile without `Copy`, because `assert_eq!` only borrows. `Format` does derive `Copy` (`output.rs:41`), so the test is correct today. It just wouldn't catch `Copy` being removed. Something like `let a = Format::Text; let b = a; assert_eq!(a, b);` would pin it. This is optional and outside what F can touch.
2. **The process-tier diagnostic test will break once every `pane` verb is live.** `a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope` runs against the first `pane` stub it finds in `STUBS`. When the last one goes live, the `.expect` panics. T already flagged this in its own notes; I'm repeating it so T-green sees it.
3. **The `--debug noisy` banner may not be what the issue meant by a diagnostic.** T recorded this as an assumption. I think it's a reasonable reading of "forces a diagnostic in JSON mode", but it is an interpretation the operator may want to confirm.

## Ready for T(green)

Ready. Since the RED contract was green on contact, I made no production change. The suite as it stands is the suite T-green should run. The plugin should now run it: GREEN advances the loop to t-green → a-dup → S. If it comes back RED, the failure is not in `output.rs`. Most likely it would be a recurrence of the earlier `remote_admin_test` flake, and that should go back to O as a pre-existing stability issue, not be routed to F as rework, since F's boundary cannot reach it.
