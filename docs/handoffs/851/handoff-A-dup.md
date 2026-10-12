# Handoff-A-dup: #851 - `holler --version` prints the commit the binary was built from  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** issue-851-implementation
**Diff base:** `bda1069`   **Diff head:** `549b451`
**Reuse map:** docs/handoffs/851/handoff-A.md, section "Reuse & Analogous-Feature map" (written by A, because the brief has none)
**Verdict:** PASS

## Summary
PASS. F extended the objects the map named and built no parallel path. The three `version` producers and the clap
attribute were changed in place, and no field was added. The change has one build script and one resolver file. The
test includes the resolver with `#[path]` instead of copying it. Only `holler_cli::VERSION` writes the
`X.Y.Z (COMMIT)` format. The seam, `holler_proto::build_info`, is the new module the map justified: it copies the
`OnceLock` shape of `log`, and its name is distinct from `holler_proto::version`. There was no rework cycle, so no
rework drift needs re-checking. One warn: the build script watches two crates that are not part of the binary, and its
comment says they are.

## Reuse map, entry by entry

| Map entry | Expected | Found in the diff | Result |
|---|---|---|---|
| The `version` producers: `holler-hub/src/control_status.rs`, `holler-hub/src/query.rs`, `holler-body/src/query.rs` | EXTEND; no new field | Each `env!("CARGO_PKG_VERSION")` now reads `holler_proto::build_info::version()` (`control_status.rs:46`, hub `query.rs:53`, body `query.rs:69`). Key and type are unchanged, and no golden file is in the diff. | extended |
| clap's `#[command(version)]` (`cli.rs:27`) | EXTEND | Only the attribute changed: `version = crate::VERSION`. | extended |
| `crates/holler-cli/build.rs` and its resolver | NEW, one of each | `find` shows this is the workspace's only `build.rs`. `build_version.rs` holds `commit_token`, `git_state` and `git`. `build.rs` calls `build_version::git` and does not run git itself. | as planned |
| The seam module | NEW, shaped like `log` | `holler_proto::build_info` has `static VERSION: OnceLock<&'static str>`, `install`, and `version()` with a `CARGO_PKG_VERSION` default. `main` installs it as its first statement (`main.rs:105`). | as planned |
| Parallel paths to reject | a second build script, or a per-crate copy of the resolver or the format | None found. There is no other build script. The test includes the resolver with `#[path = "../build_version.rs"]` (`build_version_test.rs:21`). The format is written once (`lib.rs:42-43`), and only there and in `build.rs` does `HOLLER_BUILD_COMMIT` appear. | none found |

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/build.rs:71-90` (`source_paths`); `docs/releasing.md:58-60` | `source_paths` watches `src/` and `Cargo.toml` in all ten directories under `crates/`. The comment calls them "the sources compiled into the binary", and the doc calls them "the code that goes into the binary". Two of the ten are not in the binary: `holler-load-test` and `holler-pane-testkit` (a dev-dependency only). `cargo tree -p holler-cli -e normal` lists the other eight. An edit to either crate re-runs the script and recompiles `holler-cli` for nothing. The reported `-dirty` is still true, because the tree is dirty. Scanning the directory needs no upkeep, so a hard-coded crate list would be worse. | Optional: change the comment and the doc bullet to say "every crate under `crates/`, a superset of the crates in the binary". No follow-up issue is needed. |

No duplication; the extension is clean. Checked and not findings:

- **No producer was missed, and no other copy of the version exists.** Production code has three remaining `CARGO_PKG_VERSION` uses: the seam's default, the format in `lib.rs`, and the ACP `clientInfo` (`holler-body/src/acp_driver/connection.rs:292`). handoff-A finding 2 left `clientInfo` unchanged on purpose. Every other `version` field in `holler-cli`, `holler-hub` and `holler-body` holds a schema, file-format, Herdr or HTTP version.
- **One install covers every caller.** All callers of the three status builders run in the `holler` process: `body_cmd.rs:269`, `holler-body/src/dispatch.rs:39`, and `control_server.rs:192,226`. `holler-load-test` starts the real binary, and the `herdr-holler` plugin only talks to the control socket. Neither builds a status document of its own.
- **Nothing depends on the exact old line.** Outside the tests, nothing matches the old `--version` output exactly. `release-arm64.yml:68` and the `herdr-setup` skill only print it, and `install.sh:68` checks only the exit status.
- **The test helpers are not near-copies.** `query_test.rs` reuses `StateDir`, `Hub` and its own `run`. Before `build_version_test.rs`, the repo had no fixture that builds a temporary git repo (`git`, `repo_with_one_commit`). That file's `git` differs from `build_version::git` on purpose: it sets an identity, turns off hooks and signing, and panics on failure, where the build's helper stays silent. `checkout_head` (`cli_invocation_test.rs:25`) works out the expected value on its own. Calling `git_state` there would test the code against itself.
- **The seam improves on its precedent.** `build_info::version()` reads with `get()` and a default, while `log::get()` uses `get_or_init`. A read before `install` therefore cannot lock in the default.
- **Size, docs and naming are in order.** `docs.rs` still has 899 lines (handoff-A finding 3), and `bash scripts/lint.sh` exits 0 at `549b451`. The ADR 0003 row and its copy in v2.md §10 changed the same way, which `docs_cli_test` enforces. `docs/releasing.md` extends its existing version section and adds no new doc. The diff names no personal hosts or infrastructure.

## Notes for F
None required (PASS). Finding 1 is an optional wording fix and needs no follow-up issue.
