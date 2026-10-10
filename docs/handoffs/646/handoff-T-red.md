# Handoff-T-red: Phase 4 - #646 part 1 of 3 (646a) `holler pane park` and `holler pane unpark`

**Date:** 2026-10-09
**Branch:** issue-646-implementation (worktree `.claude/worktrees/0646-park-close-routing`, head `ec2b6b3`)
**Brief / wireframe reviewed:** `docs/handoffs/646-brief.md`; `docs/handoffs/646/handoff-A.md`; no wireframe (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan, with six warns and no block (`handoff-A.md`). Warns 5 and 6 are taken up in the
tests (see "How A's warns landed in the tests" below).

## What T changed

The brief's test plan has T land the argument structs with the tests, so that RED fails on an assertion and not on a
compile error. That is the only production change, and `run` still answers `not_implemented(646)` in both files:

- `crates/holler-cli/src/pane/park.rs`: `PanePark` gains `pane: Option<String>` (`PANE`), `reason: String` (`--reason
  TEXT`, required) and `release_when: String` (`--release-when WHEN`, required). Plain `String`s, no `value_parser`
  (Decision 5: the guards are the verb's).
- `crates/holler-cli/src/pane/unpark.rs`: `PaneUnpark` gains `pane: Option<String>`.

Because the required flags change what parses, the surface rows had to change in the same step (AC 11):

- `docs/adr/ADR-0003.md` rows 54-55 now read `holler pane park [PANE] --reason TEXT --release-when WHEN [--profile NAME]  #646`
  (two spaces) and `holler pane unpark [PANE] [--profile NAME]` padded so `#646` stays in its column. Row 56 is untouched.
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: the four `# #646` park/unpark lines are now AC 11b's lines.
- `crates/holler-cli/tests/pane_verbs/process/stub.rs`: the `park` and `unpark` entries are deleted. `// #646` and the
  `close` entry stay. This is required, not optional: `holler pane park` now exits 2 (clap), so it would fail
  `stub_verb_not_implemented`.

Left to F: the verb itself (engine, guards, renderer), the ADR-0021 paragraph (AC 12) and the CHANGELOG entry (AC 13).
No test asserts the ADR-0021 paragraph or the CHANGELOG entry. Both are grep and script checks that S runs.

## Tests authored

All of them are in-process tests (`run_verb_with` over fakes), at the cheapest tier that reaches the verb's routing,
envelope and exit code. No subprocess is used: the process-level surface is already covered by `pane_cli_process`.

**Rig:** `crates/holler-cli/tests/pane_verbs/park/rig.rs` (407 lines), declared `pub(crate) mod rig;` from `park.rs`
and reached by `unpark.rs` as `crate::park::rig`.

- It has a `FakePaneStore`, a `FakeProfileStore`, a `FakeProfileScope` over the two (behind `Arc`s), and `FakeHerdr`,
  `FakeHost`, `FakeHarness` and `FakeProber`.
- The names follow #662's `profile_verbs/rig.rs` (A warn 5): `Rig::new`, `ports`, `run`, `run_both`, `Both`,
  `assert_failure`, `pane`, `member`.
- Added for park:
  - `Rig::wrapped` hands the verb and the scope a wrapper of the fake (AC 8c).
  - `record`, `seeded`, `assert_all_unchanged` and `cas_puts`.
  - `assert_no_live_call_and_no_profile_write` is the AC 9 check. It counts profile `CasPut`/`Delete`/`Rename` as
    writes and allows `Get`/`List`.
  - `assert_no_call_at_all` is the AC 7 check.
  - `profile_world` is the seed of AC 4 and 5.
  - The `Verb` enum (`Park`/`Unpark`) lets one check body serve both verbs: argv, the suffix word, the start and
    "already" holds, `is_done`, and the text lines.
- `run_both` runs the text and JSON formats on fresh rigs. It asserts equal exit codes, an empty JSON `err` and
  `check_envelope(&out, code)` (AC 10), and it runs the AC 9 check on both rigs. Every refusal and failure case goes
  through it.

**The tests:**

| Test (file) | Pins | Tier / why |
|---|---|---|
| `park::park_then_unpark_round_trips_one_pane` (`park.rs`) | AC 1: exact text lines; `hold == Parked{R, W, since}` with `t0 <= since <= t1`; generations 2 then 3; every other field equal to the seed; AC 9 | in-process: the verb's write and output |
| `park::park_and_unpark_json_pass_the_envelope_helper` (`park.rs`) | AC 2: `check_envelope(&out, 0)`; exact `data` for park (with the stored `since`) and unpark; empty `err` | in-process |
| `park::park_and_unpark_leave_a_pane_already_in_that_state` (`park.rs`) | AC 3a-c: already parked, not parked, drained (both verbs). Exact text line; JSON `data` is `changed: false`, generation 1 and the stored hold; no `CasPut`; record as seeded | in-process, both formats |
| `park::park_and_unpark_with_a_profile_take_every_member_in_name_order` (`park.rs`) | AC 4: members in name order (seeded in reverse); both at generation 2 with one `since` (Decision 6); non-members unchanged; unpark on the same rig gives generation 3; JSON names in order; the empty profile gives `no panes in profile "Demo Empty"` and `{"panes":[]}` with no `CasPut` | in-process |
| `park::park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile` (`park.rs`) | AC 5: `pane-not-in-profile` (c3, no profile; c4, other profile) and `profile-not-found`, exit 3, both verbs and both formats, nothing written. A named member with `--profile` parks only c1 (one `CasPut`) | in-process |
| `unpark::unpark_of_a_member_named_with_its_profile_changes_only_that_pane` (`unpark.rs`) | AC 5, unpark's half: the named member alone is unparked | in-process |
| `park::park_and_unpark_refuse_a_pane_with_no_record` (`park.rs`) | AC 6: `pane-not-found`, exit 3, text `err` exactly `error: pane not found: demo-c9r9\n`, JSON message equal | in-process |
| `park::park_and_unpark_usage_errors_touch_no_store` (`park.rs`) | AC 7a-e, listed below this table | in-process; 7e calls `Cli::try_parse_from` directly (the harness panics on a clap error) |
| `park::failures::park_failures_name_the_pane_and_stop` (`park/failures.rs`) | AC 8a-c, listed below this table | in-process with a test-local `PaneStore` wrapper (Decision 11: no production test switch) |
| `unpark::unpark_failures_name_the_pane_and_stop` (`unpark.rs`) | AC 8d: the same checks with `Verb::Unpark` (words `unparked by this run before it`; the seeds flipped) | same |

AC 7 in detail:

- (a) Neither PANE nor `--profile`: the exact message for park and for unpark.
- (b) Pane `a/b`: `PaneName::parse`'s own message, compared by equality with `PaneName::parse("a/b").unwrap_err()`.
- (c) `--profile "   "`: `ProfileName::parse`'s own message, compared the same way.
- (d) The guards, each run on `--reason` and on `--release-when` with the exact Decision 5 message:
  - `""` and `"   "` are blank;
  - `"a\nb"` and `"a\u{1b}b"` hold a control character;
  - 201 `x` and 201 `é` are too long.
  - Decision 5's check order, first failure only: neither, then the pane name, then the profile name, then `--reason`,
    then `--release-when`.
  - Accepted: 200 `x` on each flag, 200 `é`, and `"  disk full  "` (stored trimmed).
- Every refusal case runs in both formats with exit 2, the exact message in text `err` and JSON `error.message`, and
  **every** call log empty, the two stores' logs included.
- (e) A missing `--reason` or `--release-when` is clap's `MissingRequiredArgument`.

AC 8 in detail:

- (a) `fail_next(CasPut, Conflict)`: exactly `demo-c1r1: <conflict text>`, exit 1, the record unchanged.
- (b) A standing `Fault::Fail(Unavailable{"pane store"})`: exit 1, code `unavailable`.
- (c) `FailNthCasPut` fails the 1st, 2nd or 3rd write. Each case checks:
  - the exact message with the suffix;
  - exactly which records changed (generation 2) and that the others are as seeded;
  - that the wrapper saw exactly N `cas_put`s, so nothing is written after the failure.
- After the 2nd-write failure, a rerun with no fault exits 0 and prints `demo-c1r1: already parked ...` (unpark:
  `not parked`) and then the two changed lines. `demo-c1r1` stays at generation 2.
- Edges: with c1 already in the asked state and the first write failing, the message is
  `demo-c2r1: ...; parked by this run before it: none; not reached: demo-c3r1`. A one-member profile gets the bare
  message.

AC 9 is asserted in every test, through `run_both` or by a direct call at its end. AC 10 is asserted in every AC 5-8
case, through `run_both`. AC 11d (`pane_cli_process`, `cli_surface_test`, `docs_cli_test`, and `pane_verbs` with the
`close` stub case) is green now, because the surface edits landed with the structs.

How A's warns landed in the tests:

- Warn 5: the rig mirrors #662's names (above).
- Warn 6: AC 8 and its wrapper are in `park/failures.rs` (269 lines). `park.rs` is 413 lines, `rig.rs` 407 and
  `unpark.rs` 41, all under lint's 600-line warning.
- Warns 1-4 are for F (the named cap constant, ADR wording, scoping arms).

## RED confirmation

`cargo test -p holler-cli --test pane_verbs` over the stub structs gives `93 passed; 10 failed`. Exactly the 10 new
tests fail, and every pre-existing case passes, including `close`'s stub case. Each failure is the stub's
`not-implemented` answer meeting the assertion about the missing behavior:

| Test | Failing assertion (exact) |
|---|---|
| `park_then_unpark_round_trips_one_pane` | `park.rs:52` `left: 1 right: 0`, `Outcome { code: 1, out: "", err: "error: not implemented (story #646)\n" }` |
| `park_and_unpark_json_pass_the_envelope_helper` | `park.rs:98` `check_envelope`: `ok does not match the exit code (true at exit 0, false at any other exit code)`, out is the `not-implemented` envelope |
| `park_and_unpark_leave_a_pane_already_in_that_state` | `park.rs:126` `left: 1 right: 0` (the stub's exit) |
| `park_and_unpark_with_a_profile_take_every_member_in_name_order` | `park.rs:197` `left: 1 right: 0` |
| `park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile` | `rig.rs:246` (`assert_failure`) `left: 1 right: 3` |
| `park_and_unpark_refuse_a_pane_with_no_record` | `rig.rs:246` `left: 1 right: 3` |
| `park_and_unpark_usage_errors_touch_no_store` | `rig.rs:246` `left: 1 right: 2` |
| `park::failures::park_failures_name_the_pane_and_stop` | `rig.rs:258` `left: "not-implemented" right: "generation-conflict"` (the exit codes match at 1, so the envelope's code is what fails) |
| `unpark_failures_name_the_pane_and_stop` | `rig.rs:258` `left: "not-implemented" right: "generation-conflict"` |
| `unpark_of_a_member_named_with_its_profile_changes_only_that_pane` | `unpark.rs:32` `left: 1 right: 0` |

There is no compile error, no missing `[[test]]` (the files are modules of the existing `pane_verbs` target, which
needs no manifest entry), and no timeout.

**Check that the tests are not wrong.** I wrote a throwaway prototype of the verb in `park.rs`/`unpark.rs`, ran the suite
against it, and restored the stub. The prototype was never staged and the stub is back: no `PROTOTYPE` marker remains in
`src/`. Against the prototype, all 10 pass (`10 passed; 0 failed`), so the expected strings, JSON shapes and fake
interactions are self-consistent and reachable. Four mutations of it were each caught:

| Mutation | Caught by |
|---|---|
| Store the guard value untrimmed | `usage_errors` |
| Park overwrites any hold | AC 3 and AC 8 |
| Continue after a failed write | AC 8 for both verbs |
| Add the suffix on a one-pane scope | AC 8 for both verbs |

**Other checks, all on the RED tree:**

| Check | Result |
|---|---|
| `pane_cli_process` | 34 passed |
| `cli_surface_test` | 3 passed |
| `docs_cli_test` | 3 passed |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` | clean, after splitting two functions that hit `too_many_lines` and `cognitive_complexity` (no `allow` added) |
| `rustfmt --check --edition 2021` on the six new or rewritten `.rs` files | clean |
| `bash scripts/lint.sh` | exit 0 |
| `git diff --stat HEAD -- '*Cargo.toml'` | empty |

## Notes for F and S

- **AC 13's manifest check.** `git diff --stat origin/main -- '*Cargo.toml'` is **not** empty on this branch. That is
  not this story's doing: `origin/main` has moved past the base, and #702/#705 changed
  `crates/holler-adapter-{herdr,opencode}/Cargo.toml`. Check against the merge base, or rebase first.
  `git diff --stat HEAD -- '*Cargo.toml'` is empty.
- **The partial-failure message is built from `PaneError::Conflict`'s `Display`.** The tests compare by equality with
  the brief's text, which is `error.rs:651-653` verbatim.
- **AC 3's JSON `data`.** The tests pin `generation` 1 and the stored hold for an unchanged pane. That is Decision 8
  ("`generation` the record's after the run"; `hold` is the record's own `Hold`).
- **Test-name placement.** The brief's `park_and_unpark_*` test names each cover both verbs and live in `park.rs`.
  `unpark.rs` holds AC 8d and the AC 5 named-member case for unpark. The brief's Files section puts "unpark's cases of
  AC 3-10" in `unpark.rs`; they are in `park.rs` under the brief's own test names, which cover both verbs.

## Ready for F

RED is valid: F may implement against these tests. Staged by explicit path, and nothing is committed.
