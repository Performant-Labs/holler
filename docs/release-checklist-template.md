# Release checklist (template)

**This is the reusable template — it carries no version number.** Copy this file's content
into a new GitHub issue titled `Release checklist: vX.Y.Z` (fill in the real version) each time
a release is actually cut; fill in and check off boxes on that copy, never here. See
[`docs/releasing.md`](releasing.md) for the full narrative behind each step below.

## Pre-flight

- [ ] 1. **Identify the release commit** (usually `main`'s tip)
  - Commit SHA: `___`
- [ ] 2. **Confirm CI is green** on that exact commit
  - Both required checks: [`test (ubuntu-latest)`](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml) and `test (macos-latest)`
  - Link the specific run (not "CI passes in general"): `___`
- [ ] 3. **No real, ready work left unmerged**
  - `gh pr list --repo Performant-Labs/holler --state open`

## Testing

CI green (step 2) only proves `cargo test`/`clippy` pass — it does not exercise the full
test-case catalog, which includes manual and acceptance-gate cases CI never runs. See
[`docs/releasing.md`](releasing.md)'s "Were the tests run?" section for the full reasoning.

- [ ] 4. **Run the full catalog against this commit**
  - `ruby scripts/test-run.rb run <test-run-issue>` — run from the repo root
  - Or `discover` / `exec <ID>` per case
  - This records real pass/fail against *this* commit, not just "CI passed once at merge time"
- [ ] 5. **Every failure accounted for** — fixed, or explicitly overridden
  - A red test does **not** automatically block a release; it can be knowingly overridden
  - But the override must be recorded here, not silently skipped: `___` (which test, why, whether it also needs a "Known issues" line below)
- [ ] 6. **Run the manual acceptance gates** — [#317](https://github.com/Performant-Labs/holler/issues/317) (hlr-1103) and [#318](https://github.com/Performant-Labs/holler/issues/318) (hlr-1104)
  - Both must **run and pass** for a real release — this project exists to prove the rebuild; do not silently skip them
  - Result: `___`

## Known issues

This is a **beta** project — shipping with known, documented issues is acceptable — but they
must be checked and named, not silently omitted. Gathered here, **before** the CHANGELOG is
written below, so nothing gets missed by writing the CHANGELOG's Known Issues subsection before
this list exists.

- [ ] 7. **Skim open `bug`-labeled issues in this repo**
  - `gh issue list --repo Performant-Labs/holler --label bug --state open`
- [ ] 8. **Write one line per real, still-open issue** relevant to this release, with a link
  - List them here — this exact list goes into `CHANGELOG.md`'s Known Issues subsection in step 11, verbatim: `___`

## Version & changelog

**All of steps 9-13 happen on ONE branch, `release/vX.Y.Z`** — not a branch per file/commit.
One PR, one merge.

- [ ] 9. **Decide the version bump** (PATCH/MINOR/MAJOR)
  - From everything in `CHANGELOG.md`'s `## [Unreleased]` since the last tag
- [ ] 10. **Update the workspace `Cargo.toml`'s `version`** to `X.Y.Z`
  - This is the only source-of-truth edit — nothing else needs independent updating
- [ ] 11. **Restructure `CHANGELOG.md`**
  - Move `## [Unreleased]` content under a new `## [X.Y.Z] - YYYY-MM-DD` heading
  - Organize per [`docs/releasing.md`](releasing.md)'s "CHANGELOG entry structure" (Enhancements / Breaking Changes / Bug Fixes / Known Issues — omit empty subsections)
  - **Known Issues subsection = the list from step 8, verbatim** — don't re-derive it, don't let it drift from what was actually gathered
  - Leave a fresh empty `## [Unreleased]` above it
- [ ] 12. **Update `README.md`**
  - Notable user-facing features, fixes, or CLI-invocation changes from this release
  - Skip explicitly (don't just leave unchecked) if there's genuinely nothing README-worthy
- [ ] 13. **Push the bump commit, confirm CI green on it too**

## Tag & build

Target platforms: see [`docs/releasing.md`](releasing.md)'s platform table (currently
`ubuntu-latest` (x86_64), `macos-latest` (Apple Silicon), and `ubuntu-24.04-arm` (Linux arm64);
no Windows binary — [#308](https://github.com/Performant-Labs/holler/issues/308) fixed the
instance-lock Unix-only compile break but explicitly deferred the real blocker (the
control-socket transport is Unix domain sockets end to end); tracked separately in
[#378](https://github.com/Performant-Labs/holler/issues/378)).

- [ ] 14. **Tag the bump commit**
  - `git tag -s vX.Y.Z -m "vX.Y.Z"` (signed, annotated)
- [ ] 15. **Push the tag**
  - `git push origin vX.Y.Z`
- [ ] 16. **Build the release binaries**
  - macOS + Linux x86_64: `cargo build --release` on each target platform — build on a real
    machine of that OS (Uranus/Jupiter for Linux), never cross-compile
  - No local machine for a platform? See [`docs/releasing.md`](releasing.md)'s "Building for a
    platform you don't have locally" (the real recipe: SSH to a real machine of that OS, clone
    at the exact tag, build, verify, scp back)
  - **Linux arm64**: this org's Linux fleet (Uranus/Jupiter) is x86_64-only, so there's no SSH
    target for this one. Instead, dispatch the `release-arm64.yml` GitHub Actions workflow
    (`gh workflow run release-arm64.yml --repo Performant-Labs/holler`) — it builds and
    `--version`-verifies the binary on a real, hosted `ubuntu-24.04-arm` runner, which satisfies
    "real machine of that OS/arch" even though it isn't a machine in this org's own fleet.
    Poll it to completion (`gh run watch` or `gh run list --workflow release-arm64.yml`), then
    download the artifact: `gh run download <run-id> -n holler-ubuntu-arm64`
- [ ] 17. **Verify each build's version, then rename** before attaching anything
  - macOS/Linux x86_64: `./target/release/holler --version` actually reports `X.Y.Z`, then
    rename to `holler-<os>` (`holler-ubuntu-latest`, `holler-macos-latest`)
  - Linux arm64: the `release-arm64.yml` run already verified `--version` and `file` (real
    ELF aarch64 binary) on-runner as part of the workflow; the downloaded artifact is already
    named `holler-ubuntu-arm64`, no rename needed

## Publish (confirm before doing — public, outward-facing)

- [ ] 18. **Explicit go-ahead obtained** to actually publish
  - This step is outward-facing — don't run it on autopilot even if every step above is green
- [ ] 19. **Extract the release notes**
  - `CHANGELOG.md`'s `## [X.Y.Z]` section (step 11) is already complete — including Known Issues, since that was fed in from step 8, not written separately here
  - Pull just that section into a standalone file, e.g. `/tmp/release-notes-vX.Y.Z.md` — no new content, this is extraction only
  - Read it once as a stranger would: does it stand alone without needing the rest of the CHANGELOG for context?
- [ ] 20. **Create the GitHub Release from the tag**
  - `gh release create vX.Y.Z holler-ubuntu-latest holler-macos-latest holler-ubuntu-arm64 --notes-file /tmp/release-notes-vX.Y.Z.md`
- [ ] 21. **Review the published Release page**
  - Binaries present, notes render correctly, known-issues section visible

## Post-release smoke checks

Verify the *published* artifact, not just the local build that produced it — a locally-built
binary passing steps 16-17 is not evidence the uploaded one works.

- [ ] 22. **Download and run the actual published binary**
  - `gh release download vX.Y.Z` into a clean directory
  - `./holler-<os> --version` against *that* downloaded file, for macOS and Linux x86_64
  - Confirms the upload isn't corrupted, is the right architecture, has its executable bit set, and actually reports `X.Y.Z`
  - Linux arm64 (`holler-ubuntu-arm64`): this repo's fleet has no persistent arm64 Linux machine
    to execute it on after the CI runner is gone, so `--version` can't be re-run here. Instead
    confirm `file holler-ubuntu-arm64` reports a real `ELF 64-bit ... ARM aarch64` binary (proves
    architecture and that the upload isn't corrupted/truncated) — the `--version` check for this
    platform already happened for real on the `release-arm64.yml` runner in step 16
- [ ] 23. **Bump the Homebrew tap**
  - In a checkout of [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap): `scripts/update-formula.sh vX.Y.Z && git push`
  - Downloads the real published assets and computes real sha256s — never hand-copy checksums
  - Verify: `brew upgrade holler` (or a fresh `brew install Performant-Labs/tap/holler`) actually reports `X.Y.Z`
- [ ] 24. **Close out**
  - Link this checklist issue from the GitHub Release
  - Tick every box above, then close this issue
