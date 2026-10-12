# Handoff-F: Phase 6 - #650 Import today's panes from the launcher's files

**Date:** 2026-10-11
**Branch:** `issue-650-implementation` (on `243f910`, T's RED staged; nothing committed by F)
**Issue:** #650

## What was done

The verb, split as the brief names it: the pure plan in `holler-pane`, the file formats, the writes and the undo in
`holler-cli`.

`holler-pane` (the plan; no new dependency, no file format):

- `crates/holler-pane/src/import.rs` (stub replaced): `IMPORT_BLOCKED`, `FLEET`, `ImportRequest`, `Action`, `PaneRow`,
  `Plan` (`blocked()`, `refusal()`) and `plan()`. It calls `herdr.version()` and `herdr.snapshot()` once each, then
  `pane_store.get` per pane, then observes the remaining panes at once.
- `crates/holler-pane/src/import/findings.rs` (new): `ImportFinding` (16 kinds, `ALL`, `code()`, `level()`, `Serialize`
  from the code), `Level`, and the report's `Finding` (`{"pane", "level", "kind", "message"}`; the level is always the
  kind's).
- `crates/holler-pane/src/import/files.rs` (new): the plain values the CLI hands the plan: `PaneFiles`, `EnvValues`,
  `LauncherEntry`, `LauncherCommand`, `FileState<T>`, `SessionFiles` and `SessionsTable`.
- `crates/holler-pane/src/import/observe.rs` (new): one scoped thread per pane (`host.ps`, `harness.health`,
  `harness.list_sessions`), reconcile's server classification and `observe-failed` message (deliberate copies).
- `crates/holler-pane/src/import/assess.rs` (new): the join per pane. It covers the label in the named Herdr session, the
  cell, the tmux and server findings, the session files' agreement, the session of record, the role and the command. It
  builds the record a real run writes.

`holler-cli` (the verb):

- `crates/holler-cli/src/pane/import.rs` (stub replaced): `PaneImport` (`--from`, `--herdr-session`, `--launcher`,
  `--orchestrator`..., `--dry-run`), the usage checks, `run`, the JSON data and the text form.
- `crates/holler-cli/src/pane/import/sources.rs` (new): the state-root listing (`ignored-entry`, an unparsable name) and
  each pane's files. `sessions.toml` goes through `holler_body::config::load`.
- `crates/holler-cli/src/pane/import/pane_env.rs` (new): the `pane.env` grammar. It reads five values and every other
  variable by name only.
- `crates/holler-cli/src/pane/import/launcher.rs` (new): the launcher table. A refusal names its file, line and column,
  never `toml`'s text.
- `crates/holler-cli/src/pane/import/writes.rs` (new): the real run's checks, writes and undo (a) to (c), and the
  `profile-conflict` remedy.
- `crates/holler-cli/src/profile/show.rs`: `push_spec` is now `pub(crate)` (visibility only).

Docs:

- `docs/adr/ADR-0003.md`: the `pane import` row.
- `docs/adr/ADR-0021.md`:
  - section 9's import row and the open-codes row (`import-blocked`);
  - section 10, the import's join;
  - section 13, the launcher table's shape and step 2 narrowed (operator decision 4).
- `CHANGELOG.md`: an `[Unreleased]` Enhancements entry linking #650.

## Design decisions

1. **The order of the reads.** `herdr.version()` comes first, then `herdr.snapshot()`; either error ends the run with
   that error and no report (acceptance 14). Then each pane is checked in name order:
   - `pane_store.get`: a record means `already-imported`;
   - a `sessions.toml` row whose harness is not a `HarnessKind` means `harness-not-opencode`.

   Both are decided before any observation, and a pane left alone keeps only that finding, so a skipped pane's file
   problems never block. The other panes are observed at once (a thread each), then joined. The `get`s run in sequence
   before the threads, so a store that cannot be read ends the run with its own error (as reconcile's `resolve` does).
2. **What the plan receives.**
   - `pane.env` and the launcher entry are `Option` values. When one is `None`, its problem (`source-missing` or
     `source-unreadable`) arrives in `PaneFiles::problems`, built by the CLI, which alone knows files and positions.
   - The three session files are `FileState::{Absent, Present, Unreadable}`. When any of them is unreadable, the agreement
     is not judged; its own problem blocks the pane.
3. **The agreement** (join item 4).
   - A `sessions.toml` or `owned-sessions` that is absent imposes nothing.
   - A present file must name `last-session`. A `sessions.toml` with no row, like an `owned-sessions` with only blank and
     `#` lines, is present and names nothing, so it disagrees.
   - `sources-disagree` names the files, never the ids.
   - `session-not-found` quotes the id that the files agree on, through `quoted`.
4. **Health** follows T's decision 2:
   - `healthy` only for `Ok(true)` with the list read;
   - `unknown` for a `health` error, and for a list error after `Ok(true)`;
   - `Unhealthy("server-wedged")` or `Unhealthy("server-down")`, with the strings taken from `ImportFinding::code`
     (reconcile's).
5. **`pane.env`.**
   - A later assignment wins, as the shell's does.
   - An empty `OC_DEFAULT_AGENT` means no agent.
   - An empty `OC_EFFORT`, an `OC_MODEL` without both halves around its first `/`, a non-numeric ceiling and an
     `OC_DEFAULT_AGENT` that is not an agent key are each "a value its variable does not take". That is
     `source-unreadable`, naming the file and line only.
   - Only the first bad line is reported (one finding per file). An unreadable file is not also checked for missing
     variables.
   - `source-missing` names every required variable that is unset.
6. **The launcher table is strict.**
   - The whole file is parsed into the story's shape with `deny_unknown_fields`. A file that is not TOML, a wrong type or an
     extra key is one refusal, reported for every pane with the line and column of the error's span.
   - Per pane, port 0 or a `cwd` that is not absolute is `source-unreadable` (the brief's shape: 1 to 65535, an absolute
     path). A missing entry, port or cwd is `source-missing`.
   - A missing file is `source-missing` for every pane, including one named by `--launcher` (not a usage error).
   - A string `command` is read as a string and its text is never kept (`deserialize_with` into `()`).
7. **The text form** (A finding 6; T left it to F).
   - Each pane's line is `PANE  ACTION  ROLE  POS  SESSION`, its fields two spaces apart, as the brief writes it (no
     header, no column padding). Its findings follow it as `  <level> <code>: <message>`.
   - Findings about no pane come first, under a `-` line, as doctor's subject `-`.
   - A dry run then prints `profile fleet (fleet) to create: N specs` and each spec through `push_spec`.
   - A last line sums up the run.
8. **N in `import-blocked`.** It counts each pane directory with a problem once, plus each problem about no pane. The only
   such problem is a directory of pane files whose name is not a pane name, which is a pane directory whose name cannot be
   used.
9. **Rows.**
   - `role` is set for `record` and `blocked` rows (from the flags) and null for panes left alone.
   - `pos` is the joined Herdr cell when there is exactly one labelled pane (also for a blocked pane).
   - `session_of_record` is set only on a row that gets a record.
10. **`profile-exists` is checked with `profile_store.get` before any write.** A failed `cas_put` would still be a write
    call in the log (acceptance 9 asserts none). `insert_profile` keeps its own race mapping.
11. **The undo names what remains.** The undo answers the failed write's own code when it completes. When a step fails, it
    answers `profile-conflict`, which names:
    - the records kept (for a failed `get` at step (a), the records known to remain);
    - the two remedies of acceptance 10, with `'fleet'` single-quoted (A finding 5).
12. **Usage.**
    - `--from` is required by clap: a run without it is clap's usage error, and an envelope in JSON mode.
    - `--herdr-session` is checked by the verb, as launch's is: absent or blank is `usage`, before any read.
    - A `--from` that is not a directory is `usage`.
    - A state root that cannot be listed (permissions) is `unavailable`. It is not malformed input.
13. **Names in messages.** A pane file is named `<pane>/<file>`, relative to the state root. The launcher table is named
    by its quoted file name, not its path: `quoted` cuts at 64 characters, which would drop `launcher.toml` from a long
    temporary path.

## Reuse / extend-vs-new

- **Extended or reused:**
  - `insert_profile` (the profile write);
  - `profile_from_panes` (the plan's profile);
  - `push_spec`, widened (the dry run's spec text);
  - `single_quoted` and `pane_list` (remedies);
  - `profile::list::count`;
  - `pane::list::{optional_text, NO_VALUE}` (cells);
  - `findings::{quoted, embedded}` and the reused codes from `FindingKind::code` and `PaneCode::as_str` (one source each);
  - `holler_body::config::load` (no second `sessions.toml` parser);
  - `tx_launch::HOST_NAME`;
  - `HarnessKind`'s serde name (what counts as OpenCode).
- **New by the brief's design:** `ImportFinding`, the import's undo (operator decision 5: it deletes where `create`
  detaches), and the readers of `pane.env`, the launcher table, `last-session` and `owned-sessions`.
- **Deliberate copies** (A's list, and no other):
  - reconcile's thread-per-pane scaffold (`observe_all`, `joined`);
  - the server classification;
  - `failed_message` (`reconcile.rs` and `reconcile/observe.rs` are frozen).

## Architecture notes for A

- **New module boundary:** `holler_pane::import` with:
  - `pub mod files` and `pub mod findings`;
  - the private `assess` and `observe`;
  - the public API `IMPORT_BLOCKED`, `FLEET`, `ImportRequest`, `Plan`, `PaneRow`, `Action` and `plan()`.
- **Dependency direction unchanged:** `holler-pane` still depends on `serde`, `serde_json` and `holler-proto` only. It
  uses the crate-private `PaneCode::as_str` and `findings::embedded`. All file-format knowledge is in `holler-cli`
  (`toml`, `holler-body`; both were already dependencies). There is no manifest change, and no protocol, hub or golden
  change.
- **No frozen file edited:** `pane/mod.rs`, `ports.rs`, `pane.rs`, `profile_snapshot.rs`, `findings.rs`, `reconcile*`,
  `tx_launch.rs`, the test kit's `src/` and the Herdr adapter are untouched.
- **Ports the verb calls:**
  - Herdr: `version`, `snapshot`;
  - pane store: `get` (plan), `cas_put` and `delete` (real run);
  - profile store: `get`, `cas_put` (through `insert_profile`) and `delete` (undo);
  - host: `ps`;
  - harness: `health`, `list_sessions`.

  No prober call, and nothing that starts, stops, types or moves anything.

## Deviations from spec / wireframe

- **`tmux-session-missing`, not `no-tmux-session`; `session-not-found` reused, not new** (A finding 1, T decision 1).
  That leaves 10 new codes. The tests pin this.
- **`text` in `show.rs` is not widened.** Only `push_spec` is, since the import never calls `text` itself. The brief
  named both, and widening an unused name buys nothing.
- **Recorded health after a failed list is `unknown`** (T decision 2, acceptance 6's text). A suggested reconcile's
  `healthy`.
- **The ADR-0021 section 9 import row lists more than the brief's six codes:** also `timeout`, and that
  `session-not-found` and `command-not-argv` are findings. The six the test pins are all there.
- No wireframe (no UI).

## Tier 1 self-check (incl. tests now GREEN)

| Check | Result |
|---|---|
| `cargo build -p holler-cli -p holler-pane` | clean |
| `cargo clippy -p holler-cli -p holler-pane --no-deps --all-targets -- -D warnings` | clean |
| `rustfmt --edition 2021 --check` on every file F wrote or edited | clean |
| `bash scripts/lint.sh` | exit 0, no import file warned (largest: `assess.rs` 411 lines, the CLI's `import.rs` 322) |
| `cargo test -p holler-pane --test import_test` | 5 passed |
| `cargo test -p holler-cli --test pane_verbs` | 304 passed, 2 failed: the two tests below, on the log window only |
| `--test cli_surface_test`, `--test docs_cli_test`, `--test pane_cli_process`, `--test profile_verbs` | 3, 3, 36, 44 passed |
| `cargo test -p holler-pane` | every target passes |
| The pipeline's `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`, run with `--no-fail-fast` | 149 targets: 2158 passed, 25 ignored, 2 failed (the two tests below, the only failures, both in `pane_verbs`) |

Against the real binary (its ports are unwired until #649):

| Run | Result |
|---|---|
| a `--from` that is not a directory | `usage`, exit 2 |
| no `--herdr-session` | `usage`, exit 2 |
| an unknown `--orchestrator` | one `error:` line, exit 2 |
| no `--from` | clap's `usage` envelope, exit 2 |
| `--profile demo` | `unexpected argument '--profile'`, exit 2 |
| a valid run | stops at `herdr.version()` with `not-implemented`, exit 1, as every verb does before #649 |

## Evidence appendix

`docs/handoffs/650/evidence.md` holds 15 entries:

- `insert_profile`, `single_quoted`, `is_member`, `class_of`, `HOST_NAME` and `profile_from_panes`;
- `ConfigError`, toml's `span()`, reconcile's `observe_server` and `failed_message`;
- `HarnessKind`, `EnvVarName::parse`, `push_spec` and `output`'s text-mode error;
- the call-log facts behind the two tests below.

## Tests that look wrong (for T)

Two tests read the pane store's log window after the rig's own read has appended to it:

- `import::writes::a_failed_write_is_undone_and_answers_its_own_code`
- `import::writes::a_write_that_landed_and_timed_out_is_undone_too`

Both call `rig.records()` (a `PaneStore::list` through the fake, which logs `List`) and `rig.fleet()` before
`pane_ops_tail(&rig, &mark, 3)`. The last three calls therefore end with that `List`.

| Test | Observed tail | Expected tail |
|---|---|---|
| (a) | `[Get, Delete, List]` | `[CasPut, Get, Delete]` |
| (b) | `[Delete, Delete, List]` | `[Get, Delete, Delete]` |

Each observed tail is the expected sequence followed by the rig's `List`. So the undo order is the one the tests intend:
(a) write 1, the undo's `get` of B, the delete of A; (b) the `get` of B, the delete of B, then the delete of A. Every
other assertion in both tests passes (the codes, the exits, the empty store and the deleted `fleet`).

**Suggested fix (test only):** take the tail before `rig.records()`, or take `pane_ops_tail(..., 4)` and drop its last
element. Nothing in production code needs to change. `rig.rs`'s own doc already says to read `records()` last.

**The secret scan refuses one of T's staged lines.** This blocks the commit, not a test run.

- **The finding:** `gitleaks protect --staged --redact` (what `.githooks/pre-commit` runs, failing closed) reports one
  `generic-api-key` finding, at `crates/holler-cli/tests/pane_verbs/import/sources.rs:170`. That line is
  `let secret = "zq-env-secret-…"` in `a_pane_env_line_that_does_not_parse_names_the_file_and_line_only`.
- **The cause:** the value is made up, but the rule matches the `secret = "<token>"` shape. No other staged line is
  flagged, F's files included.
- **The effect:** the commit of the staged set (T's tests with F's code) is refused until that line changes.
- **The fix (test only):** an inline `// gitleaks:allow` on the line, or a variable name and value the rule does not take
  for a credential.

## Known issues

- **The follow-up issue** for a verb that removes only a record (acceptance 10: "A follow-up issue is filed") is not filed.
  F takes no outward action; O or S should file it.
- **Nothing a user runs changes yet.** The real binary's ports are `Unwired` until #649, so a valid run answers
  `not-implemented` from Herdr.
- **The real fleet's files are untested.** Their shapes (the launcher table above all) are checked by the cutover's
  (#654) dry run, as the brief says.
- **Formatting:** `cargo fmt --check` over whole crates shows diffs in files this story does not touch (CI runs no fmt
  check). Only F's own files were formatted.

## Files changed

- `crates/holler-pane/src/import.rs`
- `crates/holler-pane/src/import/assess.rs`
- `crates/holler-pane/src/import/files.rs`
- `crates/holler-pane/src/import/findings.rs`
- `crates/holler-pane/src/import/observe.rs`
- `crates/holler-cli/src/pane/import.rs`
- `crates/holler-cli/src/pane/import/launcher.rs`
- `crates/holler-cli/src/pane/import/pane_env.rs`
- `crates/holler-cli/src/pane/import/sources.rs`
- `crates/holler-cli/src/pane/import/writes.rs`
- `crates/holler-cli/src/profile/show.rs`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`
