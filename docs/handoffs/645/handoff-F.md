# Handoff-F: Phase 5 - #645a `pane switch` and `pane reset` (GREEN)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `a912c4b`; this work is
uncommitted, and the Workflow script commits it after this phase)
**Issue:** #645 (part 1 of 2, 645a)

## What was done

- `crates/holler-pane/src/tx_switch.rs`: the engine, filled. It has the three open codes and `SESSION_ID_MAX` (as RED
  declared them), and `parse_session_id` (`[A-Za-z0-9_-]{1,64}`, otherwise `usage` with the text quoted). `switch()`
  runs plan, act, observe, record in the brief's order (P0-P5, A1, A2, O1, R) through private helpers, one step each.
  `SwitchFailure::message` appends `; session <id> was created and is not recorded` and `; to reconcile, run holler pane
  doctor <pane> --fix`. The private `screen_text` is a word-for-word copy of reconcile's.
- `crates/holler-cli/src/pane/switch.rs`: the real `run`, plus what both verbs share. `Verb` serializes as `"switch"` /
  `"reset"`. `execute` types `PANE` and `--profile`, then the target, runs the engine and prints. `emit_outcome` prints
  the data `{verb, pane, previous}` and the one text line, or the failure's code and message.
- `crates/holler-cli/src/pane/reset.rs`: the real `run`, one call to `switch::execute` with `Target::Fresh`.
- `docs/adr/ADR-0021.md`: the Decision 17 edits: row 340, the section 8 paragraph "Switch and reset as built (#645)", one
  sentence at the end of section 11, and two "Deferred" bullets (the mismatch-code bullet marked decided, and the new
  PROPOSED 645b bullet).
- `CHANGELOG.md`: one `### Enhancements` entry under `[Unreleased]`, "Part of #645".
- `docs/handoffs/645/evidence.md` (new), and an F entry in `docs/handoffs/645/decisions.md`.

## Design decisions

1. **One after-clap path for both verbs** (`switch::execute`). The brief shares `Verb` and `emit_outcome`. Typing `PANE`
   and `--profile`, building the request with the clock, calling the engine and printing a usage error are the same
   for both verbs too. So `execute` does all of it, and `reset.rs` is its `Args` struct and a one-call `run`. Considered:
   each `run` typing its own arguments and calling `emit_outcome` itself. That repeats about a dozen lines in
   `reset.rs`, which Phase 7 would rightly call a copy.
2. **The order of usage errors.** Step 1 of the brief types `PANE`, then `--profile`, then `SESSION`. `switch::run` types
   `SESSION` first (a pure function) and passes the result to `execute`, which reports it only after `PANE` and
   `--profile`. So the brief's order holds, and `execute` needs no closure parameter.
3. **P1 takes the resolved pane by name** (`.into_iter().find(|pane| pane.name == request.pane)`), not blindly the first.
   See "Deviations".
4. **The error's text goes into the message unchanged** (no `findings::embedded` pass). The brief says "`error`'s text".
   Every other verb prints a `PaneError` as `ErrorBody::from` does. ADR-0021 section 9 makes every port message one
   line, and JSON mode flattens a message to one line anyway. Running `embedded` over it would cut at 200 characters,
   and the engine's own mismatch message, with two quoted 64-character ids and a long pane name, can approach that.
   Every value the engine itself writes into a message (the target, SHOWN, the created id) goes through `quoted`.
5. **The `data` struct** is `Outcome { verb: Verb, pane: Pane, previous: Option<String> }`, in that field order, so the
   JSON keys are `verb`, `pane`, `previous`, and `previous` is `null` when there was none. `Verb` derives `Serialize`
   (`rename_all = "lowercase"`), so its JSON name has one source. The text line takes the pane name and the new id from
   the stored record (`Switched.pane`).
6. **What the comments cite.** Production comments cite ADR-0021 sections and issue numbers, never the brief's labels
   (R-5, F-3, F-4). `docs/handoffs/` is deleted before the PR is pushed (`docs_cli_test.rs`'s `is_cli_doc` docs), so
   such a label would dangle on `main`. So the `screen_text` comment (A round 2, warn 1) names its original,
   `reconcile/observe.rs`, and calls the fold "a follow-up of #645".
7. **Where the CHANGELOG entry sits.** It follows #647's doctor entry, not the end of the list. `origin/main` has moved to
   `e327569`, and #641 appended its entry at the end there, so an entry at the end would conflict when the branch merges
   `main`. This placement merges cleanly, and it puts the entry next to doctor's, whose remedy `reset` now runs.

## Reuse / extend-vs-new

As the brief's Reuse map says, with nothing new outside it:

- **The objects extended.** The `tx_switch.rs` stub is filled, and the two verb stubs are replaced. The analogue is
  doctor's `--fix` repair (select, then observe, then a clone-and-set record in one compare-and-swap,
  `reconcile/observe.rs:318-386`), which the engine follows step for step.
- **The reconcile step:** `findings::doctor_command(Some(pane), true)`. **Quoting:** `findings::quoted`. **The SHOWN
  comparison:** `reconcile::shown_differs`. **The remedy:** `FindingKind::ServerDown.remedy(Some(&pane),
  FixState::NotFixable)`, with `doctor_command` as the no-`unwrap` fallback. **Scope:** `ports.scope.resolve`.
  **Output:** `output::{emit, emit_error, ErrorBody, ErrorCode, VerbCtx}` and `ProfileOpt`. **The clock:**
  `holler_proto::clock::now_millis`.
- **The one copy:** `screen_text`, a private two-arm helper of a #647 file, justified in the brief (O1) and accepted
  by A in both rounds. Its comment names the original, and the fold is the recorded follow-up.

## Architecture notes for A

- **Layers:** the engine is in `holler-pane`, pure over `Ports` (no I/O, no async, no new dependency). The verbs are in
  `holler-cli`. These are the edges the brief planned: `tx_switch` uses `findings` (`doctor_command`, `quoted`,
  `FindingKind`, `FixState`) and `reconcile::shown_differs`; `pane/switch.rs` uses `holler_pane::tx_switch`; `pane/reset.rs`
  uses `pane/switch.rs` (`execute`, `Verb`).
- **Public interface:** `holler_pane::tx_switch`'s items and signatures are exactly T's RED stub, which is the brief's
  API. The CLI adds crate-private items only: `Verb`, `emit_outcome` (both named by the brief) and `execute`.
- **Contracts:** no frozen file is touched (`lib.rs`, `ports.rs`, `pane.rs`, `error.rs`, `mod.rs`, `args.rs`, `output.rs`,
  `wiring.rs`, `main.rs`, any `Cargo.toml`), and no #647 file. There is no protocol change, no new closed code and no
  golden file change.
- **Patterns followed:** reconcile's one-step-per-helper split (for `cognitive_complexity`), and doctor's argument typing
  (D-5).

## Deviations from spec / wireframe

1. **P1 uses `find` by name, not `into_iter().next()`.** Under `ProfileScope::resolve`'s contract the two pick the same
   pane: `resolve(P, Some(n))` answers just `n` (`profile.rs:381-389`, in evidence.md). They differ only for a scope that
   answers with another pane. There `next()` would select in that pane's TUI and write its record, the wrong-session
   class of incident, and `find` answers `pane-not-found`. A's warn 7 (no indexing, `None` is `pane-not-found`) still
   holds.
2. **Decision 17(c), the "Deferred" bullet**, takes A's round-2 form (warn 2), because #644's paragraph is not on `main`:
   "decided, `unavailable` (exit 1), section 8: #645 for switch and reset; #644 to follow for launch and relaunch". The
   brief's literal form credits section 8 with a #644 decision it does not hold yet. AC 24 is unchanged by this.
3. **`reset.rs` reuses `super::switch::{execute, Verb}`** rather than `{emit_outcome, Verb}`: it reaches `emit_outcome`
   through `execute` (design decision 1). Nothing is copied, which is the brief's point. `emit_outcome` keeps the
   brief's `pub(crate)` and signature.
4. **The CHANGELOG entry's position** (design decision 7). AC 26 asks for one entry under `[Unreleased]`, `### Enhancements`,
   and that holds.

No wireframe applies (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo build -p holler-pane -p holler-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.56s

$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 93 filtered out
   (all 13 switch cases, all 6 reset cases, and help_names_the_arguments)

$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
     Running tests/cli_surface_test.rs   test result: ok. 3 passed; 0 failed
     Running tests/docs_cli_test.rs      test result: ok. 3 passed; 0 failed
     Running tests/pane_verbs/process/main.rs (pane_cli_process)   test result: ok. 34 passed; 0 failed
     Running tests/pane_verbs/main.rs (pane_verbs)                 test result: ok. 113 passed; 0 failed

$ cargo clippy --workspace --all-targets -- -D warnings      -> exit 0, no warnings
$ rustfmt --check --edition 2021 crates/holler-pane/src/tx_switch.rs crates/holler-cli/src/pane/switch.rs \
    crates/holler-cli/src/pane/reset.rs crates/holler-cli/tests/pane_verbs/switch.rs \
    crates/holler-cli/tests/pane_verbs/reset.rs              -> exit 0
$ bash scripts/lint.sh                                       -> exit 0 (pre-existing size warnings only; tx_switch.rs
                                                                319 lines, switch.rs 160, reset.rs 43)
$ bash scripts/changelog-check.sh                            -> changelog-check: ok
$ git diff $(git merge-base HEAD origin/main) -- '*Cargo.toml' | wc -l    -> 0 (no dependency added)
$ grep -n 'Switch and reset as built (#645)' docs/adr/ADR-0021.md
308:**Switch and reset as built (#645).** ...       (### 8. is line 265, ### 9. is line 329)
$ grep -c '#645' docs/adr/ADR-0021.md   -> 9 (origin/main: 6)
```

`pane_verbs` RED to GREEN: 94 passed and 19 failed at RED, 113 passed now. The 19 RED cases pass and every earlier
test still passes, doctor's included.

**`cargo test --workspace`:** WORKSPACE_RESULT

**The real binary** (`target/debug/holler`, `Unwired` ports until #649):

```
$ holler pane switch demo-c1r1 ses_0001                     -> error: not implemented              exit 1
$ holler pane switch BAD_NAME ses_0001                      -> error: invalid pane name "BAD_NAME": ...  exit 2
$ holler pane switch demo-c1r1 'ses_bad!' --format=json     -> {"schema_version":1,"ok":false,"data":null,"error":
    {"code":"usage","message":"invalid session id \"ses_bad!\": a session id is 1 to 64 ASCII letters, digits, '_' or '-'"}}  exit 2
$ holler pane reset demo-c1r1 --profile demo --format=json  -> {... "code":"not-implemented" ...}  exit 1
```

**The messages, as the engine writes them on the fakes.** These come from a throwaway probe crate in the session
scratchpad, outside the repo:

```
session not found: "ses_00000000000000000000000003"
"ses_00000000000000000000000002" is the session of record of demo-c2r1
the harness server of demo-c1r1 on port 48100 does not answer; run holler pane relaunch demo-c1r1
demo-c1r1 is the orchestrator's pane; pass --as-operator to change it
unavailable: the TUI of demo-c1r1 shows its home screen, not session "ses_00000000000000000000000003"; to reconcile, run holler pane doctor demo-c1r1 --fix
unavailable: the TUI of demo-c1r1 shows its home screen, not session "ses_00000000000000000000000003"; session "ses_00000000000000000000000003" was created and is not recorded; to reconcile, run holler pane doctor demo-c1r1 --fix
unavailable: tui; session "ses_00000000000000000000000003" was created and is not recorded; to reconcile, run holler pane doctor demo-c1r1 --fix
timed out: harness.create_session            (reset's create failed: nothing acted, no reconcile step)
invalid session id "ses_\u{1b}[31m": a session id is 1 to 64 ASCII letters, digits, '_' or '-'
```

## Evidence appendix

`docs/handoffs/645/evidence.md`. It has 15 facts in unchanged code, each with `file:line` and a verbatim excerpt:

- the remedy table, `doctor_command`, `quoted` and `excerpt`, and `shown_differs`;
- reconcile's `screen_text`, the original of the copy;
- `class_of` (why a mismatch is `unavailable`), and how the errors display;
- the `cas_put` and `resolve` contracts, and that the hub registry's compare-and-swap enforces nothing about
  `session_of_record`;
- doctor's `reset` remedy, the output paths, and the `Unwired` ports;
- the fake harness's `health` and its `SelectAckedWithoutTui` quirk.

## Tests that look wrong (for T)

None. Every RED test passes against the brief's behaviour, unedited. One note on coverage, not a defect: no test pins
the order between the usage errors (all of AC 12's argvs have exactly one bad argument). The order follows the brief's
step 1 by construction (design decision 2).

## Known issues

- **`origin/main` moved** to `e327569` (#641, the host adapter) after this branch's merge of `dc300ab`. That commit
  touches only `holler-adapter-host`, `Cargo.lock` and `CHANGELOG.md`, so the branch needs an ordinary merge before
  the PR. The CHANGELOG entry is placed so that the merge does not conflict.
- **A flaky hub test, unrelated to this change.** The first `cargo test --workspace` stopped on
  `join_held_test::racing_senders_get_exactly_one_prompt_through_a_grant`. In round 13 of its race matrix a body
  answered `connection_lost` mid-turn, while the machine's load average was about 25 on 24 cores. The test drives a
  real hub, body and stub agent and reaches nothing this story changes. Its target passed 11/11 on a rerun.
- **The accepted risks, unchanged:** the `session-of-other-pane` race (the brief's R-5; follow-up F-3). The window before
  645b, in which a busy pane can be switched (R-3): nothing runs against a live pane until #649. A successful reset
  leaves a permanent `stray-session` finding (F-1).
- The real binary prints a `logging_started` line on stderr before every pane verb's output, JSON mode included. It
  does the same for `pane doctor` and `pane list`, so it predates this change and is outside its scope.

## Files changed

Production:
- `crates/holler-pane/src/tx_switch.rs`
- `crates/holler-cli/src/pane/switch.rs`
- `crates/holler-cli/src/pane/reset.rs`

Docs:
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Pipeline records:
- `docs/handoffs/645/handoff-F.md` (this file)
- `docs/handoffs/645/evidence.md`
- `docs/handoffs/645/decisions.md` (the F entry, appended)
