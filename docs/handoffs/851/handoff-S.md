# Handoff-S: #851 - `holler --version` prints the commit the binary was built from  (spec audit)

**Date:** 2026-10-11
**Branch:** issue-851-implementation (HEAD `f652001`, based on `origin/main` at `bda1069`)
**Issue:** #851. The brief (`docs/handoffs/851-brief.md`) is a verbatim copy of the issue; I checked it against
`gh issue view 851`.
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md`, `evidence.md`, all in `docs/handoffs/851/`
**Verdict:** PASS

## A precondition

Met.
- `handoff-A.md` (the plan review): PASS. It has 8 warns, all addressed by T or F or carried as recorded decisions.
- `handoff-A-dup.md` (the anti-duplication gate): PASS. Its one warn is an optional wording fix (advisory 3 below).

## T precondition

Met. There are no blocking issues, and RED and then GREEN are both confirmed.

- **RED, at `3a9151e`:**
  - The two process-level tests failed on assertions about the missing behaviour:
    `cli_invocation_test.rs:54` (`want holler 0.4.0 (<commit>)`, got the bare version) and `query_test.rs:87`.
  - The new `build_version_test` target failed to compile, and the error named the missing contract file.
  - To show these tests are sound, T ran them against a throwaway wrong stub (7 of 8 failed on assertions) and a
    throwaway correct reference (8 of 8 passed).
- **GREEN, at `bf26366` on a clean tree:**
  - T's four targets passed 8, 14, 36 and 29.
  - The verify command passed: 2382 tests, 0 failures, 32 ignored.
  - `holler --version` printed `holler 0.4.0 (bf26366)`, HEAD with no `-dirty`.
  - Five mutations (M1 to M5) each failed the matching test.
  - `Blocking issues: None.`

## Acceptance criteria

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| AC 1a | A build from a git checkout prints the 7-character hash of `HEAD` | `cli_invocation_test::version_flag_prints_version_and_commit` (`:44`) runs the real binary with both `--version` and `-V`. It compares the commit with its own `git rev-parse --short=7 HEAD` (`checkout_head`, `:25`), not with the code under test. `build_version_test::git_state_of_a_clean_checkout_is_the_short_head_hash` (`:113`) and `git_state_of_a_linked_worktree_is_its_own_head` (`:145`) run on throwaway repositories. `a_checkout_uses_the_head_hash` (`:35`) also pins that a checkout beats `HOLLER_GIT_SHA`. Mutation M5 (a bare clap `version`) fails the process test. | Met |
| AC 1b | `HOLLER_GIT_SHA=abc1234` and no `.git` prints `abc1234` | `build_version_test::no_checkout_reads_holler_git_sha` (`:56`) is a pure-function test, which the issue allows. `git_state_outside_a_checkout_is_none` (`:163`) is the no-checkout side. End to end, T-green built a `git archive` tree with the variable set, and it printed `holler 0.4.0 (abc1234)`. | Met |
| AC 1c | A build with neither prints `unknown` | `build_version_test::neither_is_unknown` (`:64`), which also checks that a blank value counts as unset. T-green's `git archive` builds printed `unknown`, both with no variable and with a blank one. | Met |
| AC 2 | `holler --version` on a dirty tree ends `-dirty` | Read as T fixed it at RED (A's finding 6): the commit token ends `-dirty`, as in `holler 0.4.0 (bda1069-dirty)`. `build_version_test::a_dirty_checkout_appends_dirty` (`:48`) fails under M3. `git_state_of_a_changed_tracked_file_is_dirty` (`:130`) covers an unstaged and a staged edit; M4 shows that untracked files do not count. End to end, F's binary printed `holler 0.4.0 (5e27fa7-dirty)` mid-phase. T-green's M1 and M2 builds printed `bf26366-dirty`, and the build after the restore printed `bf26366`. | Met (see advisory 1) |
| AC 3 | No other output changes; golden files are unchanged | No golden or fixture file appears in `git diff --name-only origin/main...HEAD`. The outputs that change are the ones Work 1 and Work 3 ask for: <ul><li>the `--version`/`-V` line;</li><li>the `version` field of `query/status` (both roles), of `caps`, and of `control/status` (and `admin/status`, which v2.md defines as identical);</li><li>the first line of `hub status`, which prints that field.</li></ul>clap 4.6.6's default help template has no version line, so `--help` is unchanged (`pane_cli_process` still pins it). The log banner and `Hello` carry no build version, and the ACP `clientInfo` keeps the bare one. Nothing in `crates/`, `plugins/`, `install.sh` or the `herdr-setup` skill parses the value. F and T-green report the same workspace counts: 2382 passed, 0 failed. | Met |
| Work 1 | The first token and the version stay as they are, `-V` included | `cli_invocation_test` pins the exact prefix `holler <version> (` for both flags. `pane_cli_process::top_level_help_names_pane_and_profile` (`stub.rs:168`) pins the first two tokens. | Met |
| Work 2 | A `build.rs` in `holler-cli` running `git rev-parse --short=7 HEAD`, else `HOLLER_GIT_SHA`, else `unknown` | `crates/holler-cli/build.rs` is the workspace's only build script. Its logic is in `build_version.rs`, which the test includes with `#[path]`. For the Homebrew sentence, see Spec compliance. | Met |
| Work 3 | The same string wherever the version is shown; no new field | `query_test::status_docs_carry_version` (`:97`) checks that five values equal the `--version` text: `body query status`, the `status.version` of `body caps`, `hub query status`, the live `hub status --json`, and the first line of the human `hub status`. M1 (no install) and M2 (the hub reads `CARGO_PKG_VERSION`) both fail it. Each key and type is unchanged. | Met |
| Work 4 | The CLI surface fixture and ADR 0003's row, if the version line is pinned there; a CHANGELOG entry | `tests/fixtures/cli-surface.txt` does not pin `--version` (grep finds no match), so it is unchanged. ADR 0003's row and its verbatim copy in v2.md §10 changed together, which `docs_cli_test` enforces, and ADR 0003 gained a `**Clarified by:** #851` line. The CHANGELOG has an `[Unreleased]` Enhancements entry that links #851. | Met |
| Pipeline note | The cutover runbook says to confirm the binary with `--version` | `docs/pane-control.md` §10 has one paragraph before step 1. It says the commit must be the `main` the cutover runs, without `-dirty`. | Met |

## Spec compliance

The issue's decisions and how they were implemented:

- **Format, and the first two tokens unchanged:** implemented as stated, through one format string,
  `holler_cli::VERSION` (`src/lib.rs`).
- **"A dirty build appends `-dirty`" and "ends `-dirty`":** with the hash in parentheses, these two readings differ.
  - A flagged it (finding 6), T chose `(bda1069-dirty)` at RED, and F implemented that.
  - ADR 0003, v2.md §5.1, `docs/releasing.md` and the CHANGELOG all state the same form.
  - The choice is recorded, not silent, and I audited against it.
  - The issue does not say whether untracked files count. T's contract counts only tracked changes, as
    `git describe --dirty` does.
- **The build script and the git command:** implemented as stated. `--short=7` is the issue's own command.
- **"When there is no `.git`":** F counts a source that no git checkout tracks as having no `.git` (decision 4). An
  archive unpacked inside an unrelated repository therefore prints `HOLLER_GIT_SHA` or `unknown`, never that
  repository's hash. This adds a case and leaves each AC case as it was. It is recorded in handoff-F, Deviations.
- **"The packaged release build (Homebrew) sets it, so a release prints its tag's commit":** the premise does not
  hold, but the goal does.
  - The formula installs the published release binary, not a source build (`CHANGELOG.md:805`, the 0.3.0 entry).
  - That binary is built in a checkout of the tag (`scripts/release-build.sh:10`; `release-arm64.yml:35` runs
    `actions/checkout`), so it names the tag's commit without the variable.
  - F changed nothing in the tap or the release scripts, and said so in handoff-F, the CHANGELOG and
    `docs/releasing.md`.
  - The release checklist (step 17) and `docs/releasing.md` step 11 now check for the tag's commit without `-dirty`.
  - This is recorded, not silent. It is not an ADVISORY-HOLD either: F did not implement a defect, and the outcome the
    issue wants holds.
- **No new field in a frozen JSON envelope:** none was added. `PROTOCOL_VERSION` is unchanged. The pane contract is
  untouched, so no contract amendment was needed, as the issue says.
- **Rules:** nothing in the diff reaches a live fleet, a real Herdr session, a running pane, or a Claude or OpenCode
  session. The tests use throwaway git repositories and a loopback hub.

## Quality audit

**Correctness and failure handling**
- **The build cannot fail on git.** Every git error gives `None`, and then `HOLLER_GIT_SHA` or `unknown` is used. When
  `git status` fails after the hash was read, the tree counts as dirty, so the build never claims a clean tree it has
  not checked.
- **No locks, no rebuild loop.** `GIT_OPTIONAL_LOCKS=0` stops `git status` writing the index. A build therefore never
  holds `index.lock` against a concurrent `git commit`, and never touches a file it asks Cargo to watch.
- **Linked worktrees.** The paths to watch come from `git rev-parse --git-path`, and a worktree is tested
  (`:145`).
- **The rerun triggers cover every compiled input.** I checked this independently.
  - Production code has no `include_str!` or `include_bytes!`, and its only out-of-tree `#[path]` is `build.rs`
    including `build_version.rs`. Cargo already tracks that file through the build script's own dependency info.
  - `crates/*/src`, every crate's `Cargo.toml`, the workspace `Cargo.toml` and `Cargo.lock` cover the rest.
  - So `-dirty` describes the binary's own sources.
  - An edit to a doc or a test rebuilds neither the script nor the binary, so the binary keeps naming the tree it was
    built from. This is documented in `docs/releasing.md` and handoff-F's Known issues.
- **The seam.**
  - `holler_proto::build_info::install` is `main`'s first statement (`main.rs:105`).
  - Every status producer runs inside the `holler` process; A-dup traced the callers.
  - `version()` reads with `get()` and a default, so a read before the install cannot lock in the default.
- **CI.**
  - Both required legs build from an `actions/checkout` tree, and the process test compares against the same
    `git rev-parse` there.
  - Windows is not on the matrix (`ci.yml:67-72`). T-green's note about a Windows leg does not apply.
  - CI's clippy step (`ci.yml:348`) is the same `--workspace --all-targets -D warnings` that F and T-green ran clean.

**Build guards**
- **Panics.** `build.rs`, `build_version.rs`, `build_info.rs` and the three producers have no `unwrap`, `expect` or
  `panic`.
- **Allows.** The one new `#![allow]` (`build_version_test.rs:1`) carries `// #851`. No `dead_code` allow is needed,
  because both files that include the resolver use every item in it.
- **File size** (`wc -l` on every touched `.rs` file):
  - `docs.rs` 899: unchanged, and under the 900 gate;
  - `cli.rs` 780, `query_test.rs` 585, `cli_invocation_test.rs` 423;
  - the new files: `build.rs` 90, `build_version.rs` 59, `build_version_test.rs` 176, `build_info.rs` 27.
- **Dependencies and dead code.** There is no dead code. No dependency was added, and there are no
  `[build-dependencies]` (`cargo machete` is clean).
- **Other gates.** `lint.sh` exits 0 and `changelog-check` is ok (T-green).

**Protocol**
- `version` keeps its key and its string type, and no field was added. No golden file changed.
- v2.md §5.1 gained a paragraph: what the value holds, that it is for display only, and that the examples show a bare
  number.
- No error code changed, so the error-code table is untouched.

**Tests**
- Cross-process behaviour runs against a real hub (`query_test`, `Hub::start`).
- The new and changed tests use no fixed sleeps.
- The throwaway repositories pass their own identity and turn off hooks and signing, so the operator's git config
  cannot affect them.
- RED came first and is recorded. Each test asserts behaviour, not implementation, and M1 to M5 show each one fails
  for its own reason.

**Documentation**
- CHANGELOG: an `[Unreleased]` entry that links #851.
- ADR 0003: the row and a Clarified-by line.
- v2.md: the §10 row and the §5.1 paragraph.
- `docs/releasing.md`: a new section, "The commit in the version", and step 11.
- The release checklist: step 17.
- `docs/pane-control.md` §10: the runbook paragraph.
- The READMEs, including the translations, and `AGENTS.md` never show the `--version` output, so none needed a change.
- `docs_cli_test` skips `docs/handoffs/`, and the only test that scans every tracked file (the `setup-wizard` rename
  scan) matches nothing in this diff.

**Public-repository privacy**
- I searched every added line, handoffs included, for personal or account names, home paths, hostnames, tailnet
  names, IPs, email addresses and key patterns. There are no hits.
  - The only email address is `holler-test@example.invalid`, which uses a reserved domain.
  - No scratchpad or home path appears.
- The binary embeds only the one-word token and no file path, so `scripts/check-binary-paths.sh` is unaffected.

**Commit and PR hygiene**
- The six commits are the Workflow script's own, `chore(#851): <phase> -- ...`. Each carries
  `Co-Authored-By: Claude <noreply@anthropic.com>`.
- They have no session link. That is the script's format, the same as merged pipeline runs (for example
  `0e62b51`), and not something F or T can change.
- The PR is not open yet. Advisory 6 says what is owed when it opens.

## Scope check

F delivered the brief's scope, and nothing outside it.
- **Production:** the build script and its resolver, one format string, the clap attribute, the install in `main`,
  the three producers, and a one-line rewording of a comment in `docs.rs`.
- **The `holler_proto::build_info` seam** is not extra scope. A's finding 1 requires it: `holler-hub` and
  `holler-body` cannot depend on `holler-cli`.
- **The release-checklist and `docs/releasing.md` step 11 wording** is the optional item in A's finding 7. It serves
  the goal of Work 2 (a release names its tag's commit).
- **The `pane-control.md` paragraph** is the brief's Pipeline note.
- **There is no unrelated refactor.**
- **T's new `[[test]]` target**, instead of new cases in `cli_invocation_test` as A's finding 5 suggested, is
  justified in handoff-T-red: a compile error at RED in its own target hides no other test.
- **Size:** at `f652001`, before this handoff, the branch adds 1,391 lines, 861 of them pipeline handoffs. Without
  the handoffs it adds 530: 205 production, 255 test, and 70 in docs and the CHANGELOG. CI's PR-size step will only
  warn, above 800 added lines.
- **Since GREEN:** after T-green's checks at `bf26366`, only handoff files changed (`git diff --name-only bf26366
  HEAD`), so its recorded results hold for HEAD's code.

## Verdict

**PASS.** Every acceptance criterion and Work item is met, and each has a test that fails without the change. The
deviations are recorded and justified, and none is silent. The build guards, the protocol rules, the docs, privacy and
scope are all in order. Ready for O.

## Advisory notes (non-blocking)

1. **The wording of AC 2.**
   - The issue's two sentences are both satisfied literally by `holler 0.4.0 (bda1069)-dirty`, and only loosely by
     the chosen `holler 0.4.0 (bda1069-dirty)`, whose line ends `-dirty)`.
   - The chosen form is git's own `--dirty` convention, keeps the flag on the commit it qualifies, and serves the
     issue's purpose (telling builds apart, and confirming a clean cutover binary).
   - My call is to keep it, and the operator can confirm in review. If they want the other form, it is a one-line
     change in `commit_token`, plus T's tests and the docs that show the form.
2. **The issue's Homebrew sentence is inaccurate.** The formula installs the release binary and builds nothing, so the
   variable is never set there, and does not need to be. Worth one line in the PR body so the record is right. No
   code change is needed.
3. **Two descriptions of the watched sources overstate them:** the comment at `build.rs:15` and `:71`, and
   `docs/releasing.md:59`.
   - Both call the watched paths "the sources compiled into the binary" or "the code that goes into the binary".
   - The script actually watches every crate under `crates/`, a superset: `holler-load-test` and
     `holler-pane-testkit` are not in the binary.
   - This was A-dup's warn 1. The fix is optional wording, and no follow-up issue is needed.
4. **`checkout_head` (`cli_invocation_test.rs:25`) is looser than the build script's rule.**
   - The test counts a directory as a checkout whenever `git rev-parse` succeeds there. `git_state` also requires git
     to track a file in the crate (F's decision 4).
   - Consider a test run from a source archive unpacked inside an unrelated repository, the AUR case F names. The test
     would expect that repository's hash, while the binary prints `unknown` or `HOLLER_GIT_SHA`.
   - This cannot happen in CI or a clone. If a test-only cycle happens for another reason, `checkout_head` could make
     its own `git ls-files --error-unmatch -- .` check. That keeps the oracle independent of `git_state`.
5. **Cargo's rerun triggers have no automated test.** F, T and A all recorded this. F's table of fixture runs and
   T-green's mutation builds cover the triggers by hand. A build of a fixture workspace inside a test would cost more
   than a display string is worth.
6. **Once the PR opens:**
   - Add the AI disclosure to the PR body with `gh pr edit`, as CLAUDE.md and `CONTRIBUTING.md` ("Disclosure is
     required") ask. The script's PR body does not include it.
   - Make the squash commit's subject a Conventional Commit, for example the issue's own title,
     `feat(cli): holler --version prints the commit the binary was built from`.
