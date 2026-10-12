# Decision journal: #851, `holler --version` prints the commit the binary was built from

Run handoff directory `docs/handoffs/851` · branch `issue-851-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its own entry.

## A, up-front plan review: 2026-10-11T18:56-06:00

- **Decided:** PASS with 8 warns (see handoff-A.md).
  - The only build script is `crates/holler-cli/build.rs`, as the brief says.
  - The `version` fields built in `holler-hub` and `holler-body` get the same string through one seam that `main`
    installs at start-up. The seam copies the `holler_proto::log` pattern: set once, read with a default of
    `CARGO_PKG_VERSION`. Neither crate gets a build script of its own.
  - Existing fields keep their key; only the value gains the commit. No field is added, the protocol version does not
    change, and the golden files stay the same.
  - With no O survey in this automated run, A wrote the Reuse map in handoff-A.md. It is the baseline for the Phase 7
    gate.
- **Assumed:**
  - "Any `version` field a verb already prints" means `hub status`, `hub query status`, `body query status` and
    `caps`. All of them print either `Status.version` or the `version` in `control/status`.
  - It does not mean the ACP `clientInfo` version, which is sent to the agent and never printed.
- **Hedged:**
  - T decides the `-dirty` format and whether untracked files count as dirty (finding 6).
  - How up to date `-dirty` stays depends on the build script's rerun triggers, which F chooses (finding 4).
  - I did not verify whether the workspace `[lints]` apply to `build.rs`. Finding 4 asks for a script that passes
    clippy either way.
  - The brief's statement that Homebrew sets `HOLLER_GIT_SHA` does not hold, because the formula downloads the release
    binary. Nothing in this repository needs to change for it (finding 7).
- **Evidence:**
  - Read the brief and these sources: `cli.rs`, `main.rs`, `control_status.rs`, both `query.rs` files, `docs.rs`,
    `log.rs`, `hub_cmd.rs`, `connection.rs:292`.
  - Read these scripts and workflows: `scripts/lint.sh`, `scripts/release-build.sh`,
    `.github/workflows/release-arm64.yml`.
  - Read these docs: ADR 0003, v2.md §5.1 and §10, `docs/releasing.md`, CHANGELOG 0.3.0.
  - Read the golden `Status` and `Caps` files and the three version tests.
  - `wc -l`: `docs.rs` has 899 lines and `cli.rs` has 780.
  - Ran `git rev-parse --git-path HEAD|index|refs/heads/<branch>` in this worktree. `.git` is a file here, and the
    branch ref is in the common git directory.

## T, author / RED: 2026-10-11T19:05-06:00

- **Decided:** RED is valid (see handoff-T-red.md).
  - Format: `holler 0.4.0 (bda1069)`. On a dirty tree, `holler 0.4.0 (bda1069-dirty)` (finding 6). Only tracked
    changes count as dirty.
  - Precedence: the checkout's HEAD, then a non-empty `HOLLER_GIT_SHA`, then `unknown`.
  - The resolver contract is `crates/holler-cli/build_version.rs`, which exports
    `commit_token(Option<&str>, bool, Option<&str>) -> String` and `git_state(&Path) -> Option<(String, bool)>`.
    The new `build_version_test` target pins it.
  - Each `version` field a verb prints must equal the text `--version` prints after `holler `.
  - Three existing tests were rewritten: `cli_invocation_test` and `query_test` became the process-level RED, and
    `pane_cli_process`'s `--version` check now checks only the first two tokens.
- **Assumed:**
  - The test can read the dirty state only from the build, so the process-level `--version` test accepts `-dirty`
    either way. The dirty rule is pinned at the resolver and `git_state` level, on temp repos.
  - CI has `git` on PATH, which `actions/checkout` already needs.
  - An empty `HOLLER_GIT_SHA` counts as unset.
- **Hedged:**
  - `build_version_test`'s RED is a compile error naming the missing contract file; a function that does not exist
    yet cannot be RED any other way. I offset this by running the tests against a throwaway wrong stub (7 of 8
    failed on assertions) and a throwaway correct reference (8 of 8 passed). I deleted both; neither is
    committed.
  - `git_state_outside_a_checkout_is_none` is a guard, not a RED.
- **Evidence:**
  - Ran the cargo test, clippy and `scripts/lint.sh` results quoted in handoff-T-red.md, at HEAD `3a9151e` with
    git 2.53.0.
  - Seeded `target/` with `scripts/seed-target-dir.sh ../dryrun-import/target`.

## F, implement: 2026-10-11T19:21-06:00

- **Decided:** implemented against T's RED (see handoff-F.md).
  - `crates/holler-cli/build.rs` is the only build script. It sets `HOLLER_BUILD_COMMIT`.
  - The format is written once, in `holler_cli::VERSION` (`src/lib.rs`): `concat!` of the version and the
    commit. clap prints it, and `main` installs it first thing into the new `holler_proto::build_info`.
  - `holler_proto::build_info` is a `OnceLock` with a `CARGO_PKG_VERSION` default, A's finding 1, shaped like
    `log`. The three producers read it.
  - Dirty means `git status --porcelain --untracked-files=no`, run with `GIT_OPTIONAL_LOCKS=0` so the build
    never writes or locks the index. When status fails after the hash was read, the tree counts as dirty.
  - A checkout counts only when git tracks a file in the crate's directory (`ls-files --error-unmatch -- .`).
    An archive unpacked inside another repository prints `HOLLER_GIT_SHA` or `unknown`, never that
    repository's hash.
  - Only the first word of `HOLLER_GIT_SHA` is used.
  - The script re-runs on `HOLLER_GIT_SHA` and on these paths, printed only when they exist:
    - when in a checkout, the git files git names (`HEAD`, `logs/HEAD`, the branch ref, `packed-refs`,
      `reftable`, `index`);
    - always, each `crates/*/src` and `crates/*/Cargo.toml`, and the workspace `Cargo.toml` and `Cargo.lock`.
  - Docs:
    - ADR 0003: the row, plus a Clarified-by line;
    - v2.md: the row, plus a §5.1 paragraph;
    - `docs/releasing.md`: a section on the commit;
    - checklist step 17;
    - the cutover runbook, one paragraph before step 1 (the brief's Pipeline note);
    - CHANGELOG.
- **Assumed:**
  - Workspace members that are compiled into the binary all live under `crates/`. The plugin under `plugins/`
    is not part of the binary.
  - Reflogs are on (git's default), so `logs/HEAD` moves with every commit. Without reflogs, the index and the
    loose branch ref still catch a commit, because `git commit` rewrites the index. I checked that.
- **Hedged:**
  - The dirty flag is as of the script's last run. An edit outside the watched paths (a doc, a test) re-runs
    nothing, and does not rebuild the binary either. Recorded in `docs/releasing.md` and handoff-F's Known
    issues.
  - A `git status` that refreshes the index costs one `holler-cli` recompile (about 10 s here, for every test
    target).
  - Cargo's re-run behaviour has no automated test. I checked it by hand on a fixture workspace, with the
    results tabled in handoff-F decision 6.
  - The `reftable` ref backend was not exercised.
- **Evidence:**
  - Ran T's four targets: 8, 14, 36 and 29 passed.
  - Ran the verify command, `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`:
    exit 0, with 2382 passed, 0 failed and 32 ignored.
  - Ran the docs tests, `holler-proto`, workspace clippy with `-D warnings`, `scripts/lint.sh` and
    `cargo machete`, all clean. Full output is in handoff-F.md.
  - `evidence.md` has nine F entries.
  - The fixture-workspace probe ran in the session scratchpad and was not committed.

## T (Phase 6, verify GREEN), 2026-10-11

- **Decided:**
  - GREEN is valid: T's four targets pass at `bf26366` on a clean tree, and `holler --version` prints
    `holler 0.4.0 (bf26366)`, HEAD's hash with no `-dirty`.
  - No test changes in this phase. F flagged none, and five mutations of F's code each made the matching test fail.
  - Two evidence entries added for unchanged code the test relies on: the `body caps` and `body query status`
    paths.
- **Assumed:** the CI legs (Linux, Windows, macOS) build from a checkout, so the `--version` test compares with
  `git rev-parse` there. If a leg has no `.git`, the test's `None` branch accepts any one-word token.
- **Hedged:**
  - Cargo's re-run triggers for `build.rs` still have no automated test. I confirmed two by hand: a tracked edit
    in `src/` gave `-dirty`, and restoring it gave a clean build.
  - The Windows leg is not run locally.
- **Evidence:** handoff-T-green.md (Tier 1 table, mutation table, `git archive` builds giving `unknown`,
  `abc1234`, and `unknown` for a blank `HOLLER_GIT_SHA`).
