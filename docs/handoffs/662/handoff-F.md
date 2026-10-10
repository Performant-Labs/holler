# Handoff-F: Phase 5 - #662b profile write verbs (`holler profile create`, `holler profile delete`)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `76c774f`, changes uncommitted: F does not commit)
**Issue:** #662 (662b: closes the issue)

## What was done

- `crates/holler-cli/src/profile/create.rs`: the stub is replaced by `holler profile create NAME [--from-current | --from
  PROFILE]`. It is a plan (every B3 check before the first write) and then the writes. Every form writes through
  `insert_profile` (B7). `--from-current` joins each pane in `list()` order. If a join fails, B2's undo runs: re-read the
  failed pane, clear the joined panes newest first, delete the profile, and stop at the first failed step. The
  `Created { profile, members }` data and the carried text lines are printed once through `emit`.
- `crates/holler-cli/src/profile/delete.rs`: the stub is replaced by `holler profile delete NAME [--keep-panes]`. The
  plan checks the name, the profile (`profile-not-found`) and the members (`profile-has-live-panes` without the flag).
  The writes detach each member in order, stopping at the first failure with no re-attach, and then delete the profile.
  B4 maps the codes. The verb prints `Deleted { name, slug, detached }` once through `emit`. It also holds the three
  helpers `create` shares: `detach`, `single_quoted` and `pane_list`.
- `docs/adr/ADR-0003.md`: rows 65-66 now read `holler profile create NAME [--from-current | --from PROFILE]` and
  `holler profile delete NAME [--keep-panes]`, with `#662` kept in column 67 (AC 5).
- `docs/adr/ADR-0021.md`: section 9's `profile create` and `profile delete` rows each gain `profile-conflict` as their
  last code. Nothing else in the file changes (AC 6, Decision 14 (ii)).
- `CHANGELOG.md`: one `[Unreleased]` / `### Enhancements` entry after 662a's. It says the real binary answers
  `not-implemented` until #649 and links #662 (AC 11).
- `docs/handoffs/662/evidence.md`: the evidence appendix (new). `docs/handoffs/662/decisions.md`: the F entry.

## Design decisions

- **Plan, then write, in both verbs.**
  - `plan(args, ports) -> Result<Plan, PaneError>` runs every check of B3 in its order, before any write.
  - `write(plan, ports) -> Result<_, ErrorBody>` makes the writes.
  - `run` chains the two and calls `emit` once.

  This keeps "a check fails, nothing is written" visible in the code's structure. The alternative was one function with
  interleaved checks and writes, which hides that line.
- **A's W-1: the reusable writes have no CLI types.**
  - `insert_profile(&dyn ProfileStore, &Profile, &Actor) -> Result<Profile, PaneError>`.
  - `join(&dyn PaneStore, &dyn ProfileStore, ..) -> Result<Vec<PaneName>, JoinFailed>`.
  - `undo(..) -> Result<(), PaneError>`.
  - `detach(&dyn PaneStore, &Pane) -> Result<Pane, PaneError>`.

  `JoinFailed { pane, error, undo_error }` is a plain value. Only `JoinFailed::body` and `run` build an `ErrorBody`, so a
  later move of these functions into `holler-pane` (#650) is mechanical.
- **`JoinFailed` does not hold the profile name.** The caller passes it to `body`. With the name inside, clippy's
  `result_large_err` failed: the struct is 144 bytes against a threshold of 128. The alternative was
  `Box<JoinFailed>`. I dropped the field instead because it only repeated a value the caller already holds. `PaneError`
  is frozen, so the 120 bytes that remain cannot grow.
- **A's W-3: the quoting helper is named `single_quoted`, not the brief's `shell_word`.** That is #663's name and body
  (`pane/profile_scope.rs` on its branch), so folding the two copies later is a move, not a rename. Its home is
  `delete.rs` (B1). `delete.rs`'s module doc says it is shared, as `super::delete::...`, because the frozen
  `profile/mod.rs` admits no new module. That is the same sentence `list.rs` has for `count`. `list.rs` is outside this
  run's blast radius ("Not touched"), so the helper does not go there.
- **Two more shared helpers, so neither verb has a copy of the other's code.**
  - `pane_list` turns pane names into `a, b` or `none`. It is B4's `<list>` and `create`'s `members:` line.
  - `detach` is the one compare-and-swap that clears `Pane.profile`. `delete --keep-panes` uses it, and so does each
    leave step of `create`'s undo.

  Neither has a counterpart on `main`. The only other `", "` joins are in `hub_cmd.rs` and `show.rs`, and they join
  other things.
- **The plan check uses `is_member`.** A pane is "in another profile" when `pane.profile` is `Some` and
  `!is_member(pane, NAME)`. That is the Reuse map's "no inline slug filter": a pane already naming NAME's slug joins.
- **The undo clears `profile` to none (B2), not to the pane's earlier value.** This only matters for a pane that named
  NAME's slug before the create. NAME did not exist then (`get(NAME)` was `None`), so that was a dangling membership.
  The undo deletes NAME again, so none is the only state consistent with the registry.
- **B3's "its `Err` through `emit_error`".** The actor's parse error, like every plan error, flows into the one `emit`
  call. `emit_error` is `emit` with an `Err`, so the output is the same, and nothing is unwrapped.
- **Help text.** Each `Args` struct gets a long doc comment in the style of `list.rs` and `show.rs`: what it prints and
  the JSON shape. `ProfileCreate`'s first line is kept verbatim from the carried struct. Field docs are unchanged.

## Reuse / extend-vs-new

The brief's Reuse map is followed row by row:

- **Reused from `main`:**
  - `output::{emit, ErrorBody, ErrorCode, VerbCtx}`;
  - the closed `PaneError` variants and `class_of` through `emit`, with no new code and no `error.rs` edit;
  - `ProfileName::parse`, `slug()` and `Actor::parse`;
  - `profile_snapshot::profile_from_panes` for every new record (`&[]` for an empty profile and for `--from`), so no
    `Profile` is built by hand;
  - `profile_diff::is_member` for the plan check, `delete`'s members and B2's re-read;
  - `profile::list::count` (`2 specs`, `1 live pane`).
- **New, as the brief justified:**
  - `insert_profile` (B7);
  - the quoting helper (B1), named `single_quoted` per A's W-3.
- **New beyond the brief's list:** `pane_list` and `detach`. Each exists once, in `delete.rs`, and is used by both
  verbs. Without them there would be two copies.

No parallel path: neither verb calls `ProfileScope` (B8), any adapter or the prober.

## Architecture notes for A

- **Layers.** The change is in the CLI verb layer only (`holler-cli/src/profile/{create,delete}.rs`). Nothing in
  `holler-pane`, `holler-hub`, the test kit or a manifest changes. No frozen file is touched (`profile/mod.rs`,
  `output.rs`, `error.rs`, the ports).
- **New crate-internal interfaces (`pub(crate)`):**
  - `create::insert_profile`;
  - `delete::{detach, single_quoted, pane_list}`.
- **A new edge inside `profile/`.** `create` now uses `super::delete::{detach, pane_list, single_quoted}`. B1 planned
  this edge for the quoting helper; the other two extend it. Both verbs use `super::list::count`, as `show` does.
- **No public interface changes.** The clap structs were landed by T and are unchanged except for their doc comments,
  that is the help text.
- **Write orders** follow ADR-0021 section 8:
  - `create`: the profile first, then each join. The hub's `pane/cas_put` hook needs the profile to exist (see
    evidence).
  - `delete`: every detach, then the profile delete.

## Deviations from spec / wireframe

- The quoting helper is `single_quoted`, not B1's `shell_word` (A's W-3). Its body, placement and visibility are as
  B1 says. No behaviour changes.
- `pane_list` and `detach` are added as shared `pub(crate)` helpers in `delete.rs`, beyond the Reuse map's two new
  objects. They exist so that neither verb copies the other's code. No behaviour changes.
- ADR-0021 changes only in the two rows (AC 6 as written). A's W-2 is not applied: it would widen `profile-conflict`'s
  definition and record `delete --keep-panes` as an I1 exception. A's own fallback is "file as a follow-up" (see Known
  issues).

There is no wireframe (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

`cargo test -p holler-cli --test profile_verbs`: RED before F was `18 passed; 25 failed`, as T reported. After F:

```
running 43 tests
...
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Every AC 2a-2m and AC 3a-3h test passes, plus T's four extra tests.

Surface targets (AC 5), in one run (`cargo test -p holler-cli --test profile_verbs --test cli_surface_test --test
docs_cli_test --test pane_cli_process`):

```
Running tests/cli_surface_test.rs        test result: ok. 3 passed; 0 failed
Running tests/docs_cli_test.rs           test result: ok. 3 passed; 0 failed   (T's intended RED on ADR-0003:65-66 is gone)
Running tests/pane_verbs/process/main.rs test result: ok. 34 passed; 0 failed
Running tests/profile_verbs/main.rs      test result: ok. 43 passed; 0 failed
```

Lints, formatting and gates:

```
cargo clippy --workspace --all-targets -- -D warnings   -> Finished, no warning (after the JoinFailed fix above)
bash scripts/lint.sh                                    -> exit 0 (only pre-existing "warn: ... lines" for untouched files;
                                                           create.rs 346 lines, delete.rs 218)
rustfmt --check --edition 2021 crates/holler-cli/src/profile/create.rs crates/holler-cli/src/profile/delete.rs -> exit 0
bash scripts/changelog-check.sh                         -> changelog-check: ok
AC 1:  grep -nE 'ports\.(herdr|host|harness|prober|scope)|std::env|env::var' create.rs delete.rs -> nothing (exit 1)
AC 6:  grep -n 'profile create. |.*generation-conflict., .profile-conflict. |$' docs/adr/ADR-0021.md -> 346 (one line)
       grep -n 'profile delete. |.*generation-conflict., .profile-conflict. |$' docs/adr/ADR-0021.md -> 347 (one line)
       git diff -- docs/adr/ADR-0021.md -> only rows 346-347, each + `, \`profile-conflict\``
AC 9:  git diff ce12cdb -- '*.rs' | grep -n '^+.*unsafe' -> nothing; git diff ce12cdb -- '*Cargo.toml' Cargo.lock -> nothing
AC 12: git diff --name-only ce12cdb -> CHANGELOG.md, the two src files, the three test files and the fixture
       (T's), stub.rs (T's), ADR-0003.md, ADR-0021.md, docs/handoffs/662*
```

The real binary is unchanged until #649. `holler profile create Demo` prints `error: not implemented` and exits 1;
`holler profile delete Demo --keep-panes --json` prints a `not-implemented` envelope and exits 1. Both answers come
through `Unwired`, run with an empty `HOLLER_STATE_DIR`.

`cargo test --workspace` (existing tests):

- **First run, plain:** stopped at `holler-cli --test logging_test` with 4 failures:
  - `debug_flag_beats_env`;
  - `env_none_loses_to_flag_noisy`;
  - `banner_names_resolved_level_and_format`;
  - `log_output_stays_off_stdout`.

  Each says `Unexpected success` for `holler roster`. These tests assert that `roster` exits 1 with no hub reachable,
  and this machine runs a live hub that answered. The cause is the environment: no profile verb is involved.
- **Second run:** `HOLLER_STATE_DIR=<an empty scratch directory> cargo test --workspace --no-fail-fast`, so that no
  test can reach the machine's hub. Result: 128 test targets, **1463 passed, 2 failed, 5 ignored**.
  - `logging_test` passed: 11 of 11.
  - The 2 failures are both in `holler-cli --test interrupt_test`:
    - `interrupt_with_text_on_idle_session_just_prompts`;
    - `session_accepts_new_prompt_immediately_after_interrupt_and_reply_is_fresh`.

    In each, the test's own hub logs `conn_liveness_expired` a few milliseconds after the body connects. That is the
    hub's 45 s liveness check in `holler-hub/src/circuit.rs`, unchanged code that no profile verb reaches.
- **Third check:** `interrupt_test` alone, the same isolated command, three runs in a row: 11 passed and 0 failed
  each time. Those 2 failures are a timing flake under the fully parallel workspace run, not this change. T should
  re-run them at Phase 6.

## Evidence appendix

`docs/handoffs/662/evidence.md`. It covers 16 facts in unchanged code, each with `file:line` and a verbatim excerpt:

- `profile_from_panes`, `is_member`, `count`, `Ports: Copy`, `ErrorBody` and `ErrorCode::from`;
- `emit`'s exit code and one-line message, and `class_of` with the exit codes;
- the `Display` texts;
- `ProfileName::parse` and the pane name grammar;
- the hub's create-race answer, its name rule, its `pane/cas_put` membership hook, its move rule and its delete order;
- get-by-slug in the real store and in the fake.

## Tests that look wrong (for T)

None. Every assertion matched the brief's B1-B7 as written, including the undo order T pinned in AC 2i: the `get` of
the failed pane comes first, and the 3rd `cas_put` is pane 1's undo.

## Known issues

- **A timed-out write that landed.** Suppose `insert_profile` answers `timeout` or `unavailable` but its write landed
  (A's note for T and S). The verb reports `timeout`, and a profile with specs but no members may exist. Running it again
  answers `profile-exists`, and `holler profile show NAME` shows the state. The brief maps only `Conflict` here, so the
  error passes through unchanged. The same holds for a non-`Conflict` error from `delete`'s final profile delete after
  some detaches: the store's code and text pass through without the detached list. Running the delete again converges,
  because the detached panes are no longer members.
- **A's W-2, not applied, for the MO to file as a follow-up.** ADR-0021 defines `profile-conflict` as "after the live
  change (I8)". These two verbs now use it for "a later write of a multi-write verb, or its undo, lost after an earlier
  write landed". Also, `delete --keep-panes` is a second exception to I1, and ADR-0021 section 3 does not record it.
  AC 6 forbids both edits in this run.
- **A's W-1 follow-up.** When #650's engine (`holler-pane/src/import.rs`) needs the create write, #650 moves
  `insert_profile`, and `join`/`undo`/`detach` if it reuses them, down into `holler-pane` as a spec'd
  refactor-to-extend. Their signatures already take only ports and `holler-pane` types.
- **The `single_quoted` fold with #663.** #663's branch has a private copy with the same body. Whichever story merges
  second, or a filed issue, moves one copy to a shared home.
- **`CHANGELOG.md` will conflict with `origin/main`.** `origin/main` has moved two commits past `ce12cdb`: #640 part 2
  and #642 part 1. Both add `[Unreleased]` entries, so the run's own agent keeps both sides when it rebases. Neither
  commit touches the profile verbs, ADR-0003 or ADR-0021.

## Files changed

- `crates/holler-cli/src/profile/create.rs`
- `crates/holler-cli/src/profile/delete.rs`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`
- `docs/handoffs/662/evidence.md` (pipeline file, new)
- `docs/handoffs/662/decisions.md` (pipeline file, F entry appended)
- `docs/handoffs/662/handoff-F.md` (this file)
