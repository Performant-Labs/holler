# Handoff-F: Phase 6 - #647 the reconcile engine and `holler pane doctor`

**Date:** 2026-10-09
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-reconcile-doctor`, on head `8d1b199`)
**Issue:** #647

## What was done

- `crates/holler-pane/src/findings.rs` (filled, 351 lines). Contents:
  - `FindingKind`: the 12 kinds, `ALL` and `code()`. `Serialize` is hand-written from `code()`, so the codes have one source.
  - `FindingKind::remedy`: the one remedy table.
  - `FixState` (`as_str`, and `Serialize` from it), `FixError` (with `From<&PaneError>`) and `Finding`, plus its
    crate-private constructors.
  - `doctor_command`: the pane form of the reconcile step, `pub`.
  - The two sanitizers: `quoted` (`pub`, one value) and `embedded` (an adapter message).
- `crates/holler-pane/src/reconcile.rs` (filled, 444 lines). Contents:
  - the public API of Decision 10;
  - `shown_differs`, the one SHOWN-vs-record comparison, `pub`;
  - `reconcile()`;
  - scope resolution, the concurrent fan-out, the Herdr, stray and unregistered rules, the sort and the report.
- `crates/holler-pane/src/reconcile/observe.rs` (new, 391 lines). One pane's chain: observe, compare, repair (`--fix`),
  record. This is the brief's planned fallback file, and `lib.rs` is untouched.
- `crates/holler-cli/src/pane/doctor.rs` (replaced, 113 lines). Contents:
  - the `Args`, unchanged from T's scaffold;
  - `run`, which types the arguments, calls `reconcile` with `holler_proto::clock::now_millis()` and hands the result to
    `output::emit`;
  - the text renderer.
- `docs/adr/ADR-0021.md`: Decision 1's three edits.
  - (a) Section 12's "Deferred to #647" paragraph is now "**Decided (#647): no hub timer.**".
  - (b) The "Deferred to named stories" bullet is removed.
  - (c) One sentence follows the DRIVEN paragraph of section 11.
- `docs/adr/ADR-0003.md`: the `pane doctor` row is now `holler pane doctor [PANE] [--fix] [--profile NAME]`, with `#647`
  in the existing column.
- `CHANGELOG.md`: one `[Unreleased]` / Enhancements entry linking #647.
- `docs/handoffs/647/evidence.md`: the source facts the diff relies on.

## Design decisions

- **The 600-line split was planned up front.** `reconcile.rs` and the chain together are about 830 lines. So the chain
  went straight into `reconcile/observe.rs`, the brief's own fallback, rather than crossing lint's 600-line warning first.
  - The parent holds the API and the fleet-level rules.
  - The child holds the per-pane rules, the fix and the record write.
  - The child uses the parent's private `failed_message`, `ObservedHealth::recorded` and `shown_differs`.
- **Threads use `thread::Builder::spawn_scoped`, not `Scope::spawn`.** A thread that cannot start, or a port that
  panics, is not a panic of the pass:
  - a pane's thread becomes one `observe-failed` for that pane;
  - the Herdr thread becomes one `observe-failed` with no pane.
  This is Decision 9's "a thread that panics becomes an observe-failed finding". One thread runs the two Herdr calls,
  and one runs each pane's chain.
- **Which fields a finding fills.** The brief fixes the fields, not which kinds fill them:
  - `herdr_pane` only where the Herdr pane is the subject (`herdr-pane-missing`, `unregistered-herdr-pane`);
  - `session` only where a session is the subject (stray, missing record, foreign, the shown side of a mismatch);
  - `ports` for strays (every in-scope server that listed one), and the pane's own port for `server-wedged`,
    `server-down`, `session-of-record-missing` and `tui-foreign-session`.
  The field docs on `Finding` say this.
- **A fix acts, then observes again even when the act failed.** A timed-out select may still have switched the TUI. The
  re-observation, not the act's answer, is what gets recorded. When it fails too, the screen is "unseen" and
  `last_observed.shown` stays as stored, because nothing is guessed.
  - `fixed` needs the select to succeed and the TUI to then show the session of record.
  - When the select was acknowledged but the TUI still shows another session, `fix_error.code` is
    `shown-driven-mismatch`. ADR-0021 leaves the closed code of a post-act mismatch to #644 and #645, and a finding-kind
    code invents nothing.
- **Mismatch rule order.** `tui-foreign-session` wins over the mismatch (Decision 3, "and not `tui-foreign-session`").
  A mismatch is fixable only when the server's trusted list (healthy, read) holds the session of record.
  - When the list could not be read or the server is unhealthy, it is `not-fixable` with remedy relaunch, per the
    Decision 3 table.
  - A fixed mismatch's message says "showed", not "shows".
- **The record write** is built as `next = pane.clone()` with only `harness.health` and `last_observed.shown` changed.
  It is skipped when `next == pane`. So no other field can change, and `driven` stays as stored. A `generation-conflict`
  gets its own message; any other `cas_put` error is a plain `observe-failed`. Neither ends the pass, because the report
  is still worth printing.
- **`observe-failed` covers the record write too** (A's W-5(c)). AC 25 pins exactly 12 kinds, so no `record-conflict`
  kind could be added. The variant's doc defines the kind as "a port call of the pass failed: a read ... or the record
  write".
- **Vocabulary has one spelling** (A's W-5):
  - `FindingKind` and `FixState` serialize from `code()` and `as_str()`;
  - `ObservedHealth` serializes as `healthy | server-wedged | server-down | unknown`, with the two unhealthy spellings
    taken from `FindingKind::code()`;
  - the record's `{"unhealthy": reason}` uses the same strings.
  So one run's JSON spells a wedged server one way in `panes[].health`, `findings[].kind` and the stored record.
- **Every remedy is in one table** (A's W-4). `FindingKind::remedy(pane, fix)` is an exhaustive match. `doctor_command`
  is `pub`, so #644 and #663 can print the same reconcile step instead of spelling it again. No remedy carries a
  session, a Herdr id, a port or `--profile`.
- **One sanitizer for text** (Decision 8, A's W-6(d)).
  - `quoted` is the `pub` face of the crate-private `error::excerpt` (`{:?}`, 64 characters). The engine uses it for
    single values, and the CLI renderer for host names and the Herdr version.
  - `embedded` escapes `char::is_control` characters and cuts a message to 200 characters, as the brief specifies.

## Reuse / extend-vs-new

The brief's Reuse map is followed row by row:

- `Ports` and the six traits;
- `ProfileScope::resolve` (doctor has no membership check of its own);
- `PaneStore::list`, `get` and `cas_put`;
- the closed `PaneError` variants and `class_of`, through `ErrorBody::from` and `output::emit`;
- `GridPos` `Display` and `Serialize` (no hand-formatted row or column);
- `error::excerpt` (wrapped as `quoted`);
- `args::ProfileOpt`, flattened;
- `PaneName::parse` and `ProfileName::parse`;
- A's W-3: `holler_proto::clock::now_millis()`, not a hand-written `SystemTime` read.

The new objects are the brief's own:

- `findings.rs` and `reconcile.rs` fill the stubs ADR-0021 section 5 assigns to #647;
- `reconcile/observe.rs` is the brief's named fallback.

One near-neighbour checked: `holler-proto/src/log.rs` has a private `escape_field_value`, the same `is_control` rule
for log lines. It is not reachable from `holler-pane`, it is outside this story's blast radius, and it lacks the 200-character
cut. So `embedded` is new.

## Architecture notes for A

- **Layers.** The engine is in `holler-pane`, works only through `Ports`, and adds no dependency and no async runtime
  (`std::thread::scope`). The verb is one file, with its own `Args`, and prints only through `output::emit`. No frozen
  file, manifest, `lib.rs`, test-kit file or other verb's file is touched.
- **Public API beyond Decision 10.** Each item is additive and was asked for by A:
  - `reconcile::shown_differs` (W-2);
  - `findings::doctor_command` (W-4);
  - `FindingKind::remedy` (W-4, "one table");
  - `findings::quoted` (W-6(d));
  - `FixState::as_str`;
  - `impl From<&PaneError> for FixError`.
  `ObservedHealth`'s variants are renamed `ServerWedged`/`ServerDown` (W-5(b)); T confirmed no test pins them.
- **W-1.** The meanings of what reconcile writes are stated in `reconcile.rs`'s module doc:
  - the `harness.health` writer;
  - `shown: None` with `at > 0`;
  - `at` as first-observed;
  - `driven` left as stored.
  ADR-0021 section 1's rows are not edited, because AC 28 forbids any other ADR-0021 line. Edit (c) uses A's more
  accurate wording ("which I2 makes the session the hub drives"), not the brief's "which the hub drives under I2".
- **The pass order.** Each pane's chain makes at most seven calls (`ps`, `health`, `list_sessions`, `shown_session`, then
  with `--fix` `select_session` and `shown_session`, then `cas_put`). The pass takes about the slowest chain.

## Deviations from spec / wireframe

- **Dedupe is on the whole finding, not Decision 2's `(kind, pane, session, herdr_pane)` key.** AC 21 requires two
  `observe-failed` findings with no pane (`herdr.version` and `herdr.snapshot`). They share that 4-tuple, so the decision's
  key would merge them and AC 21 would fail.
  - Every finding is unique by construction: one rule per pane per kind, strays merged by session.
  - The final `sort` plus `dedup()` drops only exact duplicates, such as a snapshot listing one pane twice.
  - The sort key is Decision 2's, with the message as a last tie-break so the order is total.
- **A record's first observation is written even when it equals the defaults.** When `last_observed.at == 0` and
  something was observed, the record is written even if `shown` and `health` equal the stored defaults. A's W-1 found that
  "write only on change" otherwise leaves `at == 0` after a real observation (a pane with no TUI whose health check
  failed), which a reader takes for "never observed". After that first write, only a change is written, so AC 8 and AC 13
  are unaffected.
- **Two serde forms are hand-written.** `FindingKind`'s `Serialize` comes from `code()`, not `#[serde(rename_all)]`
  (A's W-5(a)). `ObservedHealth`'s values are `server-wedged`/`server-down`, not `wedged`/`down` (A's W-5(b)).
- **ADR-0021 edit (c) uses A's wording**, as above.
- No other deviation: the remedy strings, exit codes, JSON shape, text line formats and the fixture group are exactly
  Decisions 3, 7, 10 and 11.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test pane_verbs doctor
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 63 filtered out
$ cargo test -p holler-cli --test cli_surface_test --test docs_cli_test --test pane_cli_process --test pane_verbs
cli_surface_test 3 passed; docs_cli_test 3 passed; pane_cli_process 34 passed; pane_verbs 94 passed (0 failed)
$ cargo test -p holler-pane
every target ok (findings_test 1 passed; error_test 12; records_test 11; ...; doc-tests 3)
$ for i in $(seq 20); do cargo test -p holler-cli --test pane_verbs observation_runs_concurrently || break; done
observation_runs_concurrently: 20/20 passed
$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (no warning)
$ bash scripts/lint.sh
lint exit: 0 (warnings only for pre-existing files; none for a touched file)
$ bash scripts/changelog-check.sh
changelog-check: ok
$ rustfmt --check --edition 2021 <the 4 production files, doctor.rs and its 3 submodules, process/stub.rs, findings_test.rs>
(clean)
$ cargo fmt --all --check 2>/dev/null | grep '^Diff in' | sed "s|$PWD/||" | grep -E 'reconcile|findings|pane/doctor|pane_verbs/doctor|process/stub'
(nothing; see Known issues for why the sed is needed)
$ grep -rn unsafe crates/holler-pane/src crates/holler-cli/src/pane crates/holler-cli/tests/pane_verbs
(nothing)
$ git diff origin/main -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml' | wc -l
0
$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!
$ grep -c 'Deferred to #647' docs/adr/ADR-0021.md ; grep -n 'Decided (#647)' docs/adr/ADR-0021.md
0
472:**Decided (#647): no hub timer.** ...
$ grep -c '^holler pane doctor \[PANE\] \[--fix\] \[--profile NAME\] *#647$' docs/adr/ADR-0003.md
1
$ gitleaks detect --no-git --source <each new production file and evidence.md>
no leaks found
```

`cargo test --workspace` (AC 30), first run, against this machine's default state dir:

- every target passed up to `logging_test`;
- `logging_test` failed 4 of its 11 cases with "Unexpected success" for `holler roster`. Those cases assume no hub is
  reachable, and a live hub on this machine answered.
- With an isolated `HOLLER_STATE_DIR`, `logging_test` passes 11/11.

That is environmental and unrelated to this diff, which touches no logging or roster code.

**Workspace run (isolated):**

```
$ HOLLER_STATE_DIR=<fresh empty dir> cargo test --workspace --no-fail-fast
126 test targets: 1415 passed; 0 failed; 5 ignored
cargo test --workspace exit: 0
```

T: run the workspace suite with an isolated `HOLLER_STATE_DIR` on this machine, or `logging_test` sees the live hub.

## Evidence appendix

`docs/handoffs/647/evidence.md`: 11 facts covering:

- the fake harness's frozen/killed answers and its `shown_session`;
- `ps` on a missing session;
- the `<port>.<method>` timeout op;
- `excerpt`;
- the pane-name grammar;
- `Ports` being `Copy` and `Send + Sync`;
- the hub's periodic-writer rule;
- the fixture's `at: 0` and `unknown` health;
- `class_of` exit codes;
- `now_millis`.

## Tests that look wrong (for T)

None.

## Known issues

- **AC 31's grep is vacuous as written in this worktree.** The brief's
  `cargo fmt --all --check 2>/dev/null | grep '^Diff in' | grep -E 'reconcile|findings|...'` matches every `Diff in` line of
  every file, because the worktree path `.claude/worktrees/0647-reconcile-doctor/` contains `reconcile` and `doctor`. It
  passes once the path is made relative (`sed "s|$PWD/||"` before the `grep -E`), which is how it was run above. T and S
  should use the relative form.
- **AC 33 and `findings_test.rs`.** `crates/holler-pane/tests/findings_test.rs` (T's, per A's W-6(a)) is still not in the
  brief's Files list. T flagged it for O, and nothing of F's is outside the list.
- **C-8 is still open.** The relaunch and reset remedies name `holler pane relaunch <pane>` and `holler pane reset <pane>`,
  exactly as specified. Both verbs are stubs on `main` until #644 and #645 merge. The doctor-form remedies parse (AC 26).
- **A's W-1 standing-spec half is not done here.** ADR-0021 section 1's `harness` and `last_observed` rows still name no
  meaning for what reconcile writes, because AC 28 forbids the edit. The meanings are in `reconcile.rs`'s module doc. O
  decides whether to carry them into the ADR through the amend channel.
- **A's W-7 is unchanged by this story, as the brief says.** (a) A foreign TUI on a shared data directory is not
  detectable through the frozen ports. (b) Strays accumulate after a reset.

## Files changed

- `crates/holler-pane/src/findings.rs`
- `crates/holler-pane/src/reconcile.rs`
- `crates/holler-pane/src/reconcile/observe.rs` (new)
- `crates/holler-cli/src/pane/doctor.rs`
- `docs/adr/ADR-0021.md`
- `docs/adr/ADR-0003.md`
- `CHANGELOG.md`
