# Handoff-F: #851 - `holler --version` prints the commit the binary was built from

**Date:** 2026-10-11
**Branch:** issue-851-implementation
**Issue:** #851

| Field | Value |
|-------|-------|
| GitHub issue | #851 |
| Working branch | `issue-851-implementation` (worktree `.claude/worktrees/0851-version-hash`) |
| Build plan phase | none (the issue is the source of truth); pipeline phase F |
| Input documents read | `docs/handoffs/851-brief.md`, `handoff-A.md`, `handoff-T-red.md`, `decisions.md`, ADR 0003, v2.md §5.1 and §10, `docs/releasing.md`, `docs/pane-control.md` §10 |
| Acceptance criteria count | 3 (plus Work items 1-4) |
| Handoff document path | `docs/handoffs/851/handoff-F.md` |

Run by the Workflow script, so the table is a record, not a question. No scope split: the files are the ones A's
plan names (handoff-A, Reuse map), and each production change is a few lines.

## What was done

- `crates/holler-cli/build_version.rs` (new): T's contract file. `commit_token(git_hash, dirty, holler_git_sha)`
  picks the token (checkout hash, `-dirty` inside the parentheses, else the first word of a non-empty
  `HOLLER_GIT_SHA`, else `unknown`). `git_state(dir)` asks git for `rev-parse --short=7 HEAD` and the dirty
  flag. `git(dir, args)` runs git for both and for `build.rs`.
- `crates/holler-cli/build.rs` (new): the only build script. It sets `HOLLER_BUILD_COMMIT` and tells Cargo
  when to run again (see the design decisions below).
- `crates/holler-cli/src/lib.rs`: `pub const VERSION = "<CARGO_PKG_VERSION> (<HOLLER_BUILD_COMMIT>)"`, the one
  place the format is written.
- `crates/holler-cli/src/cli.rs`: the attribute only, `version` becomes `version = crate::VERSION`.
- `crates/holler-cli/src/main.rs`: `holler_proto::build_info::install(holler_cli::VERSION)` first thing in
  `main`, before any command runs.
- `crates/holler-proto/src/build_info.rs` (new) and `lib.rs`: the seam of A's finding 1, a `OnceLock<&'static
  str>` with `install` and `version()`, defaulting to `CARGO_PKG_VERSION`.
- `crates/holler-hub/src/control_status.rs`, `crates/holler-hub/src/query.rs`, `crates/holler-body/src/query.rs`:
  the three producers read `holler_proto::build_info::version()` instead of `env!("CARGO_PKG_VERSION")`.
- `crates/holler-proto/src/docs.rs`: the `Status.version` comment reworded in place (still 899 lines).
- Docs: ADR 0003 (the `--version` row and a `**Clarified by:** #851` header line), v2.md (the same row, and a
  §5.1 paragraph on `version`), `docs/releasing.md` (the version bullets and a new "The commit in the version"
  section, plus step 11), `docs/release-checklist-template.md` step 17, one paragraph before step 1 of
  `docs/pane-control.md` §10, and a CHANGELOG `[Unreleased]` Enhancements entry.

Output now (this worktree, tree dirty mid-phase): `holler 0.4.0 (5e27fa7-dirty)`; `hub status` begins
`hub 0.4.0 (5e27fa7-dirty) (protocol 3)`.

## Design decisions

1. **The seam is `holler_proto::build_info`** (A's finding 1): set once by `main`, read with a default, like
   `log::init` (`log.rs:257,305`). It holds a `&'static str`, because the only value is the compile-time
   `holler_cli::VERSION`. Rejected: a parameter threaded through `hub serve` and `body run` to the three
   producers (many async layers for one string); a build script in `holler-hub`, `holler-body` or
   `holler-proto` (a parallel path, and per A every crate would rebuild on each commit). The name avoids
   `holler_proto::version`, which is the protocol version.
2. **One format string.** `holler_cli::VERSION` is `concat!(CARGO_PKG_VERSION, " (", HOLLER_BUILD_COMMIT, ")")`.
   clap prints it, `main` installs it, and nothing else writes the format. `build.rs` emits only the token. The
   output variable `HOLLER_BUILD_COMMIT` is named apart from the input `HOLLER_GIT_SHA`.
3. **Dirty is `git status --porcelain --untracked-files=no`, run with `GIT_OPTIONAL_LOCKS=0`.** Tracked changes
   only, staged or not (T's contract). The variable stops `git status` from writing the index back. Without it
   a build could hold the index lock while an agent runs `git commit` in the same checkout, and that commit
   would fail on `index.lock`. It would also touch a file Cargo watches, which re-runs the script once more. I
   used the variable rather than the `--no-optional-locks` flag: a git older than 2.15 ignores the variable but
   fails on the flag. If `status` fails after `rev-parse` worked, the tree counts as dirty: the hash is kept and
   is not claimed clean.
4. **A checkout must track a file in the crate's directory** (`git ls-files --error-unmatch -- .`). A source
   archive unpacked inside some other repository (an AUR build directory, a vendored copy) then counts as no
   checkout and prints `HOLLER_GIT_SHA` or `unknown`, never the other repository's hash. A wrong commit is
   worse than `unknown`. T's four `git_state` tests use a repo with a tracked file, so they are unaffected
   (all pass).
5. **`HOLLER_GIT_SHA` contributes its first word**, so the token is always one word: blank counts as unset (T's
   test), and a stray newline cannot break the one-line `cargo:rustc-env` directive. T's `--version` test makes
   the same no-whitespace demand.
6. **What re-runs the script** (A's finding 4: F chooses and records it, here and in `docs/releasing.md`). Only
   paths that exist are printed, because Cargo re-runs a script at every build while a watched path is
   missing:
   - when in a checkout, the git files that git itself names through `rev-parse --git-path`: `HEAD`,
     `logs/HEAD`, the branch `HEAD` names, `packed-refs`, `reftable` and `index`. In this worktree that is
     the worktree's own HEAD, log and index, and the common directory's ref and `packed-refs`;
   - always, the `src/` and `Cargo.toml` of every crate under `crates/`, and the workspace `Cargo.toml` and
     `Cargo.lock`;
   - `HOLLER_GIT_SHA` (`rerun-if-env-changed`).

   The source paths go beyond A's minimum. Without them, editing code and rebuilding gives a binary that says
   clean while it is not, which is the AC "on a dirty tree ends `-dirty`" failing in the most common case.
   They cost almost nothing: an edit there recompiles `holler-cli` anyway. I did not watch every tracked
   file, because then every doc or test edit would recompile `holler-cli`.

   Checked by hand on a two-crate fixture workspace using this `build.rs` and `build_version.rs` (scratch, not
   committed):

   | Step | Result |
   |---|---|
   | no-op rebuild | no re-run |
   | `git status` that refreshes the index | re-run |
   | tracked edit in a dependency's `src/` | re-run, `-dirty` |
   | commit | re-run, new hash |
   | doc-only edit | no re-run, the binary is not rebuilt |
   | staging that doc | re-run, `-dirty` |
   | detached checkout and back | re-run each time, correct hash |
   | `git pack-refs --all` (the loose ref disappears) | one re-run, then stable |
   | commit after packing | re-run, new hash |
   | editing `build_version.rs` | the script is rebuilt and re-run |
   | no `.git` | `unknown` |
   | no `.git` with `HOLLER_GIT_SHA=abc1234` | `abc1234` |
   | no `.git` with a blank `HOLLER_GIT_SHA` | `unknown` |
   | untracked copy inside another repo | `unknown` |

   Measured here, a re-run that recompiles every `holler-cli` test target costs about 10 s
   (`cargo test -p holler-cli --no-run`).
7. **Docs.** The `--version` row is changed the same way in ADR 0003 and its verbatim copy in v2.md §10, with
   two spaces kept before the note. ADR 0003 gets a "Clarified by: #851" line, not a new ADR (the #676
   precedent). `docs_cli_test` parses every inline code span that starts with `holler ` in `docs/**`, so
   prose never shows an output line such as `holler 0.4.0 (...)` in backticks. It writes "`X.Y.Z (COMMIT)`
   after the word `holler`" instead (a span such as `holler X.Y.Z` fails that test). The v2.md §5.1 JSON
   examples keep `"0.1.0"`, and the new paragraph says they show a bare number (A's finding 2 asked for the
   sentence; nothing parses those examples). The brief's Pipeline note asks the cutover runbook to say to
   confirm the binary by `--version`, so §10 of `docs/pane-control.md` now says that before step 1. #835's
   branch does not touch that file, and its ADR 0003 and CHANGELOG hunks are far from these.

## Reuse / extend-vs-new

Per A's Reuse map:
- **Extended:** the existing `version` producers and fields (`control_status.rs`, both `query.rs`, the clap
  attribute). No field was added.
- **New, as A justified:** `crates/holler-cli/build.rs` and its resolver `build_version.rs` (no build script
  existed; the brief names it), and the seam module `holler_proto::build_info`, which copies `log`'s shape.
- **No parallel path:** one build script, one resolver, which T's test includes rather than copies, and one
  format string.

## Architecture notes for A

- **New public API:** `holler_proto::build_info::{install, version}` and `holler_cli::VERSION`.
- **Dependency direction unchanged.** `holler-cli` → `holler-proto` ← `holler-hub`/`holler-body`. `holler-proto`
  gains only `std::sync::OnceLock` and stays free of OS, network and async dependencies.
- **Build script.** `holler-cli` gets its first build script. It uses the standard library only (no
  `[build-dependencies]`) and embeds only the token, no file path, so `scripts/check-binary-paths.sh` still
  holds. The workspace lints apply to `build.rs` (checked: clippy-driver compiles it with the workspace denies).
- **Rebuild scope.** A re-run recompiles only `holler-cli`, the leaf crate.
- **Wire unchanged.** `Status.version` and `control/status`'s `version` keep their key and type; only the value
  gains ` (COMMIT)`. The protocol version is unchanged and the golden files are untouched.
- **Process start.** `main`'s first statement is the install; the dispatch order is otherwise unchanged.

## Deviations from spec / wireframe

- **"When there is no `.git`" (Work 2).** It is read as "no git checkout tracks this source". An archive unpacked
  inside an unrelated repository has a `.git` above it but is treated as no checkout (decision 4).
- **"The packaged release build (Homebrew) sets it" (Work 2).** This does not hold, and nothing changed for it.
  The formula installs the published release binary, and that binary is built from a checkout of the tag, so it
  names the tag's commit without `HOLLER_GIT_SHA` (A's finding 7; evidence `CHANGELOG.md:805`,
  `scripts/release-build.sh:10`). S should not count this as unmet. `HOLLER_GIT_SHA` remains the setting for an
  archive build.
- **"Ends `-dirty`" (AC 2).** It is satisfied as T chose at RED: `holler 0.4.0 (bda1069-dirty)`, git's `--dirty`
  form inside the parentheses (A's finding 6).
- Wireframe: N/A (no UI).

## Tier 1 self-check (incl. tests now GREEN)

At HEAD `5e27fa7` plus this phase's uncommitted changes, git 2.53.0, rustc 1.98.1:

```
$ ./target/debug/holler --version ; ./target/debug/holler -V
holler 0.4.0 (5e27fa7-dirty)
holler 0.4.0 (5e27fa7-dirty)

$ cargo test -p holler-cli --test build_version_test --test cli_invocation_test --test query_test --test pane_cli_process
build_version_test   test result: ok. 8 passed; 0 failed
cli_invocation_test  test result: ok. 14 passed; 0 failed
pane_cli_process     test result: ok. 36 passed; 0 failed
query_test           test result: ok. 29 passed; 0 failed

$ cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test --test cli_surface_test
docs_cli_test 3 passed; docs_pane_control_test 16 passed; cli_surface_test 3 passed
$ cargo test -p holler-proto            every target ok (golden, docs_wire, docs_errors, codec, ...)

$ cargo clippy --workspace --all-targets -- -D warnings      Finished, no warnings
$ bash scripts/lint.sh                                        exit 0 (only the existing 600-line warns;
                                                              docs.rs 899, cli.rs 780)
$ cargo machete                                               no unused dependencies
```

The verify command, `.aftersight/pipeline.config.json`'s `test.unit.command`:

```
$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
exit 0: 153 test-result lines, 2382 passed, 0 failed, 32 ignored
```

After that run I made only layout edits: rustfmt on `build.rs`, the watched list built with `extend` instead of a
shadowing `let`, `commit_token`'s `let sha`, and `VERSION` wrapped at 100 columns. I then re-ran T's four
targets, `docs_cli_test`, `docs_pane_control_test` and `cli_surface_test`, which all passed. I also re-ran
clippy with `-D warnings` on `holler-cli`'s build script, lib and test targets after touching them, `lint.sh`
and `changelog-check.sh`, all clean.

## Evidence appendix

`docs/handoffs/851/evidence.md`: nine F entries. They cover `hub status`'s print of `version`, the `log`
`OnceLock` precedent, the hub and body running inside `main`, both `query/*` dispatch points, the ACP
`clientInfo` left alone, the golden literals, the release build from the tag, and the Homebrew formula
installing the release binary.

## Tests that look wrong (for T)

None.

## Known issues

- **The dirty flag is as of the last run of the build script.** A tracked file outside the watched paths
  (a doc, a test) can change without a re-run. The binary is then not rebuilt either, so it still names the
  tree it was built from; staging, committing or a code edit brings it up to date. Documented in
  `docs/releasing.md`.
- **Extra `holler-cli` recompiles.** Each of these re-runs the script, and the next build recompiles
  `holler-cli` (about 10 s for every test target here):
  - a `git status` that refreshes the index;
  - a new untracked file, such as an editor swap file, inside a watched `src/`;
  - a rewrite of `packed-refs` by any worktree of the clone (`git pack-refs`, `gc`, deleting a packed branch),
    because all worktrees share that file.
- **Not covered by an automated test:** Cargo's re-run behaviour and the end-to-end `HOLLER_GIT_SHA`/`unknown`
  builds. I checked them by hand (the table in decision 6). T's tests pin the same logic as pure functions. The
  `reftable` backend was not exercised; its `reftable/` directory is watched when it exists.

## Files changed

Production:
- `crates/holler-cli/build.rs` (new)
- `crates/holler-cli/build_version.rs` (new)
- `crates/holler-cli/src/lib.rs`
- `crates/holler-cli/src/cli.rs`
- `crates/holler-cli/src/main.rs`
- `crates/holler-proto/src/build_info.rs` (new)
- `crates/holler-proto/src/lib.rs`
- `crates/holler-proto/src/docs.rs` (comment only)
- `crates/holler-hub/src/control_status.rs`
- `crates/holler-hub/src/query.rs`
- `crates/holler-body/src/query.rs`

Docs:
- `CHANGELOG.md`
- `docs/adr/ADR-0003.md`
- `docs/protocol/v2.md`
- `docs/releasing.md`
- `docs/release-checklist-template.md`
- `docs/pane-control.md`

Handoffs: `docs/handoffs/851/handoff-F.md`, `docs/handoffs/851/evidence.md`, `docs/handoffs/851/decisions.md`
(appended).
