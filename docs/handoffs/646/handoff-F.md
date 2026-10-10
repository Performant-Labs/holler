# Handoff-F: Phase 5 - #646 part 1 of 3 (646a) `holler pane park` and `holler pane unpark`

**Date:** 2026-10-09
**Branch:** issue-646-implementation (worktree `.claude/worktrees/0646-park-close-routing`, head `b996a8f` before F's
commit, base `ce12cdb`)
**Issue:** #646 (part 1 of 3; the PR says "Part of #646")

The Workflow script calls this run's F step "Phase 6" (its own numbering). In the role doc's numbering, used by the
handoffs and the journal of this run, it is Phase 5.

| Field | Value |
|-------|-------|
| GitHub issue | #646 (epic #633), part 1 of 3 (646a: park and unpark only) |
| Working branch | `issue-646-implementation` |
| Build plan phase | none: the issue and `docs/handoffs/646-brief.md` are the source of truth (repo `CLAUDE.md`) |
| Input documents read | the brief; `handoff-A.md`; `handoff-T-red.md`; `decisions.md`; ADR-0021 sections 1, 3, 8, 9 and 12; the stubs; `doctor.rs`, `output.rs`, `args.rs`, `mod.rs`, `profile/list.rs`; the `holler-pane` error, pane, ports, profile, findings and reconcile sources; the test kit's scope, store and envelope checker; T's tests and rig |
| Acceptance criteria count | 13 (AC 1-13); F's share: the verb (AC 1-10 behaviour), AC 12 (ADR paragraph) and AC 13's CHANGELOG |
| Handoff document path | `docs/handoffs/646/handoff-F.md` |

There is no human to confirm the table in the automated path, so F went ahead on it. The scope is under the role's cap:
two production files, one verb family, one surface (the CLI verb), plus the ADR paragraph and the CHANGELOG entry.

## What was done

- `crates/holler-cli/src/pane/park.rs` (rewritten from the stub; T's argument struct kept as T landed it):
  - `run`: types the arguments, then runs the shared engine and prints the report.
  - `park_request`: the typing step. It checks in Decision 5's order (the target, then `--reason`, then
    `--release-when`), before any port call, and takes the run's one `since`.
  - `checked_text` and `MAX_TEXT_CHARS`: the text guards. Trim, then refuse blank, then a control character, then more
    than 200 characters. The message is exactly `<flag> must not be blank`, `<flag> must not contain a control
    character` or `<flag> must be at most 200 characters`, and it never holds the value.
  - Shared with `unpark.rs` as `pub(super)`:
    - `HoldRequest { target, change }`;
    - `HoldTarget` (`Named(PaneName)` or `Profile { profile, pane }`) and its `parse`, which refuses neither PANE nor
      `--profile`, then a bad pane name, then a bad profile name;
    - `HoldChange` (`Park { reason, release_when, since }` or `Unpark`);
    - `run_hold_change`, the end of both verbs.
  - Private: the engine `change_holds`, the scope rule `in_scope`, the failure suffix `progress`/`listed`, the report
    types `HoldReport`/`PaneHold`, and the renderer `render`/`line`.
- `crates/holler-cli/src/pane/unpark.rs` (rewritten from the stub; struct as T landed it): `run` types the target and
  calls `run_hold_change` with `HoldChange::Unpark`.
- `docs/adr/ADR-0021.md`: the Decision 12 paragraph, verbatim, inserted after line 130 (the "Neither" bullet of section 3).
  10 lines added, none removed. Row 341 (now line 351) is unchanged.
- `CHANGELOG.md`: one entry under `## [Unreleased]` / `### Enhancements`, after #662's, ending "#646, part 1 of 3".
- `docs/handoffs/646/evidence.md` (new): the facts of unchanged code that the diff and the tests rely on, each quoted
  with its file and line, for the diff gate.
- `docs/handoffs/646/decisions.md`: F's journal entry.

## Design decisions

- **A closed target type.** `HoldTarget` is `Named(PaneName)` or `Profile { profile, pane }`.
  - Typing refuses "neither" (Decision 2) before a target exists, so no later code has an impossible `(None, None)` arm.
  - The alternative was doctor's pair of `Option`s, with a dead arm in the scope match. Rejected because the dead arm
    would be a silent empty run if it were ever reached.
  - `HoldTarget::parse` runs both parsers (they are pure), then reports in Decision 5's fixed order.
- **The change decides, the engine applies.** `HoldChange::next_hold(&Hold)` returns the hold to write, or `None` to
  leave the pane as it is (Decision 4):
  - park writes only a pane whose hold is `none`, and unpark only a parked one;
  - every other pair, drained included, is `None`. A future `Hold` variant would also be left unwritten, the safe
    default.
  - The engine reads the whole scope first (one `get`, or one `resolve`), sorts it by name, then makes one `cas_put`
    per pane at the generation it read, and stops at the first failure (Decisions 3 and 7).
- **The failure message is built where the failure is known.** The code is the failed write's
  (`ErrorCode::from(&error)`), and the message is `<pane>: <error>` plus `progress(..)`:
  - The suffix is empty for a scope of fewer than two panes. Otherwise both clauses are always present, an empty list
    being `none`.
  - "parked/unparked by this run before it" lists only the entries this run wrote (`changed: true`). "not reached" is
    every pane after the failed one.
  - It is an `ErrorBody` and not a `PaneError`, because `PaneError::Conflict` carries no text to extend. Decision 9
    already has the engine return an `ErrorBody`.
- **The report holds the record's own values.** `PaneHold { name, changed, generation, hold }`:
  - a written pane's values are what `cas_put` returned, a left one's are the record as read;
  - `hold` is the `Hold` value itself, so serde gives the JSON form, with no hand-built mirror (Decision 8).
- **The text line is derived, with no impossible branch.** `line` matches on the entry's hold:
  - a parked hold is `parked (...)` when `changed`, else `already parked (...)`;
  - `none` is `unparked` when `changed`, else `not parked`;
  - only `drained` needs the verb: park's `drained, left as it is`, unpark's `not parked` (AC 3c).
  - Every arm is truthful whatever state reaches it, so no `unreachable`-style fallback was needed. The rejected
    alternative was a separate "outcome" enum in each entry, kept out of the JSON. It would carry the same information
    twice.
- **The empty-profile text names the profile as typed.** `no panes in profile <q>` uses the parsed `--profile` value,
  not the stored display name (they are equal in the tests). This matches doctor, whose report echoes the requested
  scope, and keeps the engine free of the resolved profile.
- **`since` is taken in the typing step.** It is taken once, after the guards pass and before any store call, so every
  pane of a run carries one `since` (Decision 6, AC 4).
- **Shared as `pub(super)` from `park.rs`** (Decision 9): the frozen `pane/mod.rs` admits no new module. `unpark.rs` uses
  `run_hold_change`, `HoldRequest`, `HoldTarget` and `HoldChange`, and nothing else is visible outside `park.rs`.

## Reuse / extend-vs-new

- **Extended**, per the brief's Reuse map:
  - `PanePark` and `PaneUnpark` keep `#[command(flatten)] profile: ProfileOpt`; `args.rs` is untouched.
  - The verb shape is doctor's: type the arguments, run on `ctx.ports`, then `output::emit` with a text renderer.
    `ErrorBody::from` is used for every error answered as it is.
- **Called, not copied:**
  - `ProfileScope::resolve` (the only membership rule);
  - `PaneStore::get` and `cas_put`;
  - `PaneName::parse` and `ProfileName::parse` (their own messages);
  - `holler_pane::findings::quoted` (all quoting);
  - `holler_proto::clock::now_millis`;
  - `class_of`, through `output.rs`.
- **The one pattern copied:** `in_scope` copies the two arms of `holler_pane::reconcile`'s private `resolve`
  (A warn 4), down to the same `PaneError::PaneNotFound { what: name.to_string() }`. AC 6's `pane not found: demo-c9r9`
  therefore reads as doctor's does. It cannot be called: it is private, in a frozen crate. The shared "panes in scope"
  helper is a follow-up (below).
- **Unpark** calls park's engine and has no loop, scope logic or renderer of its own.
- **New objects:** only the `pub(super)` request, target and change types and the private report types of this verb.
  They are the engine Decision 9 calls for, and no existing object models a pane-hold change.
- **A warn 1:** the cap is `MAX_TEXT_CHARS`, whose doc cites `holler_hub::holds::MAX_REASON_CHARS` (same 200, opposite
  policy) and does not import it (ADR-0021 line 44 keeps the two holds apart).

## Architecture notes for A

- **Layers touched:** the CLI verb layer only (`crates/holler-cli/src/pane/park.rs`, `unpark.rs`). Nothing changes in
  `holler-pane`, `holler-pane-testkit`, `holler-hub`, `output.rs`, `args.rs`, `mod.rs` or `cli.rs`, and no manifest
  changes.
- **New dependency edge:** `pane::unpark` → `pane::park`, as planned by Decision 9.
  - The new `pub(super)` interface: `HoldRequest`, `HoldTarget::parse`, `HoldChange` and `run_hold_change`.
  - The crate's public API is unchanged. The `pub` items are still `PanePark`, `PaneUnpark` and the two `run`s, with T's
    fields.
- **Contracts:** no protocol, record or error-code change. Only closed codes are raised, ADR-0021 row 341 stands, and
  the JSON `data` is new output for verbs that answered `not-implemented`.
- **No live act and no prompt gate** (Decisions 1 and 10). `send_prompt` is untouched.
- **`archChanged: true`** is reported. This cycle adds a cross-file `pub(super)` interface and a sibling dependency
  (planned and reviewed at plan time, but new code). The role doc says to report `true` when in doubt.

## Deviations from spec / wireframe

None in behaviour or text. Two notes:

- `park.rs` is 401 lines against the brief's "about 200". The difference is the module doc and rustfmt's layout; it is
  under lint's 600-line warning.
- A warns 2 and 3 (the ADR paragraph's wording) are **not** applied. AC 12 pins the paragraph to Decision 12's text, and
  A's notes say F cannot take them without departing from the brief. They are a follow-up (Known issues).
  - Warn 3's substance is in `park.rs`'s module doc: no reconcile step is printed, because there is no live act, and a
    timed-out write may have landed, which a rerun reports as already in the asked state.
  - The message format is Decision 7's as written.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  (the 10 RED tests among them: park::park_then_unpark_round_trips_one_pane,
   park::park_and_unpark_json_pass_the_envelope_helper, park::park_and_unpark_leave_a_pane_already_in_that_state,
   park::park_and_unpark_with_a_profile_take_every_member_in_name_order,
   park::park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile,
   park::park_and_unpark_refuse_a_pane_with_no_record, park::park_and_unpark_usage_errors_touch_no_store,
   park::failures::park_failures_name_the_pane_and_stop, unpark::unpark_failures_name_the_pane_and_stop,
   unpark::unpark_of_a_member_named_with_its_profile_changes_only_that_pane ... ok)
$ cargo test -p holler-cli --test pane_verbs park     -> ok. 10 passed; 93 filtered out
$ cargo test -p holler-cli --test pane_verbs unpark   -> ok. 9 passed; 94 filtered out
$ cargo test -p holler-cli --test pane_cli_process    -> ok. 34 passed
$ cargo test -p holler-cli --test cli_surface_test    -> ok. 3 passed
$ cargo test -p holler-cli --test docs_cli_test       -> ok. 3 passed
$ cargo test -p holler-cli --test profile_verbs       -> ok. 19 passed
$ cargo test -p holler-cli --lib                      -> ok. 9 passed
$ cargo clippy -p holler-cli --all-targets -- -D warnings          -> Finished, exit 0
$ cargo clippy --workspace --all-targets -- -D warnings            -> Finished, exit 0
$ rustfmt --check --edition 2021 crates/holler-cli/src/pane/park.rs crates/holler-cli/src/pane/unpark.rs -> clean
$ bash scripts/lint.sh            -> exit 0 (only pre-existing size warnings; park.rs 401, unpark.rs 38)
$ bash scripts/changelog-check.sh -> changelog-check: ok
$ git diff --stat ce12cdb -- '*Cargo.toml'   -> empty
$ grep -c '^\*\*Park and unpark as built (#646).\*\*' docs/adr/ADR-0021.md -> 1
```

The real binary over the unwired ports (Tier 1 smoke, no fleet touched):

- `holler pane park --reason r --release-when w` exits 2 with `error: pane park needs a PANE or --profile NAME`.
- `holler pane park demo-c1r1 --reason r --release-when w` and `holler pane unpark --profile Demo` exit 1 with
  `error: not implemented`.
- `holler pane unpark demo-c1r1 --format=json` prints one `not-implemented` envelope and exits 1.

The stores are wired by #649, as for doctor and `profile list`.

## Evidence appendix

`docs/handoffs/646/evidence.md`: 27 entries. They cover the output module's error and exit-code paths, `class_of`, the
`PaneError` texts the messages embed, `quoted`/`excerpt`, `Hold`'s serde form, the `cas_put` and `resolve` contracts,
the fakes' ordering and generation behaviour, `now_millis`, the prompt-hold cap, reconcile's `resolve` arms, the pane-name
grammar, and ADR-0021's reconcile-step rules.

## Tests that look wrong (for T)

None. One note: T's handoff flagged that `git diff --stat origin/main -- '*Cargo.toml'` is not empty because of upstream
drift (#702/#705). Against the merge base `ce12cdb` it is empty.

## Known issues

- **ADR wording follow-up (A warns 2 and 3), for O.** The merged paragraph states the text guard as a property of the
  reason and release condition, which the record does not enforce. It also does not say that these verbs print no
  reconcile step on `generation-conflict` or `timeout`. A's suggested sentences fit 646b's brief, which edits ADR-0021
  too.
- **Other follow-ups A named, for O to file:**
  - one shared "panes in scope" helper for doctor, `get`, park, close and switch/reset (warn 4);
  - the rig consolidation, with an owner (warn 5);
  - optionally, a guarded type for the hold texts (warn 2).
- **Until #649** wires the hub's stores into the binary, the real verbs answer `not-implemented` for any valid request.
  The CHANGELOG says so.
- **Text mode cuts a long reason.** `quoted` cuts at 64 characters, so a reason of 65 to 200 characters prints cut in
  text mode. JSON carries all of it. This is doctor's rule for untrusted values, by design (Decision 8).

## Files changed

Production:
- `crates/holler-cli/src/pane/park.rs`
- `crates/holler-cli/src/pane/unpark.rs`

Docs (F's share of the brief's surface list):
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Handoff artifacts:
- `docs/handoffs/646/handoff-F.md`
- `docs/handoffs/646/evidence.md`
- `docs/handoffs/646/decisions.md`
