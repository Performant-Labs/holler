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
