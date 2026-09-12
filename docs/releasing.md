# Releasing

How to cut a release of `holler` — one repo, one binary, two roles (hub and body; ADR 0001).
Unlike the legacy `holler-server`/`holler-client` split, there is no "each repo releases
independently" framing here: one version, one tag sequence, one changelog, one binary.

Versioning policy (SemVer, pre-1.0 PATCH/MINOR/MAJOR rules) is settled by the legacy repos'
ADRs; this doc is the mechanics that were left open ("whoever sets up the first real release").
See [issue #319](https://github.com/Performant-Labs/holler/issues/319) for the decision record
behind the choices below.

## Where the version number lives

**The workspace `Cargo.toml`'s `[workspace.package] version` field is the single source of
truth.** Every member crate inherits it via `version.workspace = true` (see ADR 0003's comment
in `Cargo.toml`). Nothing else derives it independently:

- `holler --version` / `-V` reads it at compile time via clap's `#[command(version)]`
  (`crates/holler-cli/src/cli.rs`), which pulls Cargo's own `CARGO_PKG_VERSION` build-time env
  var — this is why bumping the workspace `Cargo.toml` is the *only* step needed for the binary
  itself to report the new version.
- `query/status`'s JSON document (both roles) also carries this version — see
  `holler_proto::Status`'s `version` field, populated from the same `CARGO_PKG_VERSION`.
- The git tag (`vX.Y.Z`), the `CHANGELOG.md` heading, and a GitHub Release's title must all match
  what's in `Cargo.toml` at the commit being released — they're representations of the same
  fact, not independent decisions.

Format: standard SemVer, starting at `0.1.0`.

## Were the tests run? What proves it?

**The release commit must be `main` at a commit where CI's `test` job is green on every matrix
entry** — check the [Actions tab](https://github.com/Performant-Labs/holler/actions/workflows/ci.yml)
for that exact commit's run, or `gh run list --branch main --workflow ci.yml --limit 1` /
`gh pr checks <the merging PR>`. "Green" means both required checks passed:

| Check | What it proves |
| --- | --- |
| `test (ubuntu-latest)` | Canary + wire harness + full `cargo test --workspace` + interop-tagged tests + `cargo clippy --workspace --all-targets -- -D warnings` + `scripts/lint.sh` + `scripts/changelog-check.sh`, all clean on Linux |
| `test (macos-latest)` | Same, on macOS |

These are already the checks `main`'s branch protection requires before any PR can merge — so
in practice, if the release commit is `main` itself (not a stray unmerged branch), CI has
already gated it once at merge time. Re-checking the Actions run for that specific commit before
tagging is the confirmation step, not a re-run — don't tag off a commit whose CI run you haven't
actually looked at.

**CI green is necessary, not sufficient.** It only proves `cargo test`/`clippy`; it never
exercises the full test-case catalog ([issue #168](https://github.com/Performant-Labs/holler/issues/168)),
which includes manual and acceptance-gate cases outside CI's reach. Before tagging, also run the
catalog itself — `ruby scripts/test-run.rb run <test-run-issue>` (or `discover`/`exec <ID>` per
case; see [`running-tests.md`](running-tests.md) for the full flag reference), **run from the
repo root** — and make a deliberate call on whether this release warrants the full manual
acceptance gates ([#317](https://github.com/Performant-Labs/holler/issues/317)/[#318](https://github.com/Performant-Labs/holler/issues/318)).
A release note this doc, or the checklist, ever says "tests passed" off CI alone is wrong.

**A red test does not automatically block a release — it can be knowingly overridden.** This is
a real, standing option, not a last resort: whoever's cutting the release can decide a specific
failure doesn't hold this release up. The one hard requirement is that an override is
**recorded, not silent** — which test, why, in the release-checklist issue (see below) — so
"tests passed" in the release notes is never quietly covering for "we chose to ship anyway."

## Which platforms does a release target?

**Don't ship a binary for a platform CI never ran the suite on.** Only `ubuntu-latest` and
`macos-latest` qualify today:

| Platform | CI-tested | Released | Status |
| --- | --- | --- | --- |
| `ubuntu-latest` | Yes | Yes | Full support |
| `macos-latest` | Yes | Yes | Full support |
| Windows | No | No | Excluded from CI (ADR 0002 retires Windows from the CI matrix) — tracked in [#308](https://github.com/Performant-Labs/holler/issues/308); add a row here (and to CI's matrix) once that's resolved |

This table is the one place platform status is recorded — the checklist and any other doc
mentioning target platforms should link here rather than repeat/restate it.

## Building for a platform you don't have locally

You're usually cutting a release from one machine (a Mac, say), but the platform table above
requires a binary for Linux too. For a platform you can't build on locally, the proven recipe is
to **build on a real machine of that OS, never cross-compile**:

1. SSH to a real machine running that OS — this org's Uranus or Jupiter for Linux. Doesn't need
   to be dedicated to this, just needs to exist and be reachable.
2. If Rust isn't already installed there: `curl --proto '=https' --tlsv1.2 -sSf
   https://sh.rustup.rs | sh -s -- -y --default-toolchain stable`.
3. Clone the repo fresh **at the exact tag**, not `main`: `git clone --branch vX.Y.Z --depth 1
   https://github.com/Performant-Labs/holler.git ~/holler-build` — a shallow, tag-pinned clone,
   not a checkout of whatever that machine happened to have lying around.
4. `cargo build --release` there, for real — not cross-compiled from the Mac.
5. Verify on the remote machine before pulling anything back: `--version` reports the right
   version, and `file target/release/holler` confirms it's a real binary for that platform (e.g.
   `ELF 64-bit LSB pie executable, x86-64` for Linux).
6. `scp` the verified binary back to wherever you're assembling the release's files.

This is genuine cross-*building*, not cross-*compiling* — every platform's binary is actually
built on that platform, by a real toolchain, from a real clone of the tagged commit. Don't try to
set up cross-compilation toolchains (e.g. `cross`, manual target triples) as a shortcut; a real
remote machine per platform is simpler and gives a binary you can trust without also trusting a
cross-compilation toolchain's correctness.

## What a release actually produces

Two tiers — the first is required, the second is a deliberate extra:

1. **A git tag + `CHANGELOG.md` entry.** Always. `vX.Y.Z`, annotated (`git tag -a`), signed (this
   org's git config already has `commit.gpgsign`/`tag.gpgsign` on — sign release tags the same
   way). The `CHANGELOG.md`'s `## [Unreleased]` section becomes `## [X.Y.Z] - YYYY-MM-DD`,
   summarizing everything merged since the last tag (Keep a Changelog format, hand-written per
   release — not generated from commit messages).
2. **A GitHub Release**, with one binary attached per platform, named `holler-<os>`
   (`holler-ubuntu-latest`, `holler-macos-latest`) — built via `cargo build --release` on each
   platform, from the exact tagged commit, then renamed to that convention before attaching. This
   is a **public, outward-facing artifact** — confirm with whoever's driving the release before
   publishing it, every time; it's not something to automate past without a look.

## CHANGELOG entry structure

A release's `CHANGELOG.md` entry (`## [X.Y.Z] - YYYY-MM-DD`) is organized into fixed
subsections, in this order, each omitted entirely if empty (don't print an empty
`### Bug Fixes` with nothing under it):

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Enhancements
- New capability or additive change, one line each.

### Breaking Changes
- Anything that breaks the CLI surface, `--json` output shape, or on-disk file formats.
  Omit this subsection entirely if there are none — don't print "None" either, just leave
  it out.

### Bug Fixes
- Real fixes, one line each, linking the issue.

### Known Issues
- Real, still-open gaps worth a user knowing about before they hit them — see this doc's
  own "Known issues" section below for the policy. Link the issue.
```

This mirrors the real shape large projects converge on for the same reason. Keep a Changelog's
own vocabulary (Added / Changed / Deprecated / Removed / Fixed / Security) is a fine reference
but not literally used here — this project's four buckets (Enhancements / Breaking Changes / Bug
Fixes / Known Issues) map onto it well enough (Enhancements ⊇ Added+Changed, Bug Fixes = Fixed,
Breaking Changes ⊇ Removed+some Changed) without needing all six of its headings.

## Known issues

**A known, documented issue is not by itself a release blocker** — this project ships betas
with real gaps rather than holding a release hostage to a fix. Before finalizing release notes:

- Skim open `bug`-labeled issues (`gh issue list --repo Performant-Labs/holler --label bug
  --state open`) — a real defect doesn't need the `bug` label to be worth mentioning (e.g. a
  plainly-titled defect report with no label yet).
- For anything real and still open, add one line to the release notes: what it is, roughly when
  it bites, and a link to the issue. Silence is the failure mode here, not the bug itself — a
  user who hits a documented known issue is annoyed for a minute; a user who hits an undocumented
  one loses trust in the release notes.
- A `bug` you've already root-caused (even if not yet fixed) is worth linking that diagnosis in
  the release note too, not just the symptom.
- The per-release checklist issue (see below) is where the *specific, current* list for that
  release actually lives — this section is the durable policy, not a live list, since a specific
  issue number here would go stale the moment it's fixed.

## Who cuts a release, and when

Manual, on-demand — a person decides "time to tag." No cadence, no automated trigger. This may
change once there's real release volume to justify automating it (an automated tool like
`cargo-release`/`release-plz` was explicitly left for later, not ruled out).

## Step by step

**All version-bump work (Cargo.toml, CHANGELOG.md, README.md) lands on ONE branch, named
`release/vX.Y.Z`** — not a separate branch per file/commit. One PR, one thing to review, one
merge. `main` is protected, so this still goes through a PR like anything else; it just isn't
several PRs.

1. Confirm the release commit (usually `main`'s tip) has a green CI run on both platforms — see
   above.
2. Run the full test-case catalog against that commit (`test-run.rb run`/`exec`), and decide
   whether this release warrants the full manual acceptance gates (#317/#318) — see "Were the
   tests run?" above. CI green alone is not this step.
3. Gather known issues **now, before writing the CHANGELOG** — skim open `bug`-labeled (and
   otherwise plainly real) issues, write down the exact list. This list goes into the
   CHANGELOG's Known Issues subsection verbatim in step 7 — gathering it after would mean
   writing that subsection twice, or worse, from memory.
4. Branch `release/vX.Y.Z` off the release commit. Everything below (steps 5-8) happens on this
   one branch.
5. Decide the version bump (PATCH/MINOR/MAJOR) from everything accumulated in `CHANGELOG.md`'s
   `## [Unreleased]` section since the last tag.
6. `cargo set-version <X.Y.Z>` (or edit the workspace `Cargo.toml`'s `version` by hand) — this
   is the only source-of-truth edit; nothing else needs independent updating.
7. Move `CHANGELOG.md`'s `## [Unreleased]` content under a new `## [X.Y.Z] - YYYY-MM-DD` heading,
   organized per "CHANGELOG entry structure" above — Known Issues subsection is step 3's list,
   verbatim; leave a fresh empty `## [Unreleased]` above it.
8. **Update `README.md`** with anything a user landing on the repo needs to know about this
   release: notable new features, fixes, or changes to CLI invocation (new/changed flags,
   subcommands, output shape). Not every CHANGELOG line belongs here — only what changes how
   someone actually uses the tool, the same bar as a man page going stale.
9. Commit those file changes on `release/vX.Y.Z` (one commit or several, same branch), open a
   PR, get it merged, confirm CI is green on the merge commit itself too.
10. `git tag -s vX.Y.Z -m "vX.Y.Z"` (signed, annotated) on the merge commit, `git push origin
    vX.Y.Z`.
11. `cargo build --release` on each target platform (see the platform table above); verify
    `./target/release/holler --version` actually reports `X.Y.Z` before attaching anything, then
    rename each to `holler-<os>` for attaching.
12. Extract `CHANGELOG.md`'s `## [X.Y.Z]` section (already complete, including Known Issues) into
    a standalone file — that's the release notes, no new content to write.
13. **Confirm before publishing** — this is a public, outward-facing artifact. Get an explicit
    go-ahead, then create the GitHub Release from the tag (`gh release create vX.Y.Z
    <binaries...> --notes-file <that extracted file>`).
14. **Verify the published artifact, not just the local build.** Download the binaries actually
    attached to the GitHub Release (`gh release download vX.Y.Z`), from a clean directory, and
    run `./holler-<os> --version` against *that* file for each platform — confirms the upload
    isn't corrupted, is the right architecture, has its executable bit set, and actually reports
    `X.Y.Z`. A local build passing step 11 is not evidence the uploaded artifact works; only
    downloading and running the real thing is.

For the actual checklist to run through each time (not just the narrative above), copy
[the reusable, versionless **Release checklist (template)**](https://github.com/Performant-Labs/holler/issues/326)
into a new issue titled `Release checklist: vX.Y.Z`, and fill in that copy. Don't check boxes on
the template itself.
