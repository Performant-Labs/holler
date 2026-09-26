## This stack

Rust 2021 workspace. The build guards make these mistakes unmergeable, so write to them from the start. The numbers below are copies: `Cargo.toml` (`[workspace.lints]`), `clippy.toml` and `scripts/lint.sh` are the source of truth and win if they differ.
- Clippy denies `unwrap`, `expect`, `panic`, `unreachable`, `too_many_lines` (100), `cognitive_complexity` (15) and `struct_excessive_bools`; rustc denies `dead_code`. Helpers return `Result`; a helper lands with its first caller. Every `#[allow(...)]` needs a trailing `// #NNN` issue link.
- File size: `scripts/lint.sh` warns at 600 lines and **fails at 900**. If your change would push a file past about 850, decompose first (a sibling module) rather than after the lint fails.
- A dependency with a feature list needs a `# for <consumer>` marker per feature; `cargo machete` flags unused dependencies.
- Protocol changes: the error-code table in `holler-proto` is closed (add to `ALL` and the count); new wire fields are additive and optional, with no protocol version bump; existing golden files must stay byte-identical.
- Prompts reach a body only through `send_prompt` in `crates/holler-hub/src/circuit/dispatch.rs`. Put control-plane behavior at that choke point, not at call sites. Blocking store I/O in async code goes through `spawn_blocking`; token-store operations take `acquire_lock_retrying`.
- Hub state lives under `<state dir>/hub/`: write files atomically, set the permission a file that carries secrets needs, and decide what happens on a corrupt or unwritable file (fail closed, never silently drop state).
- This is a **public** repository: no personal hostnames, tailnet names, IPs, machine or account names, or private domains anywhere (code, tests, docs, comments, commit metadata). Use neutral placeholders.
- `CHANGELOG.md` needs an `## [Unreleased]` entry (Enhancements or Bug Fixes) linking the issue; `scripts/changelog-check.sh` enforces it. Update the README "Debug output" table and `docs/` when a log event, CLI surface or protocol field changes.
- Commits use a Conventional Commit subject (`fix(hub): ...`, `feat(hub): ...`, `docs: ...`) and a `Co-Authored-By` trailer with a session link. The PR body states the AI assistance per `CONTRIBUTING.md`.

## Project-specific references

- `CONTRIBUTING.md`, `docs/adr/` (ADR 0003 CLI surface and versioning, ADR 0004 protocol, ADR 0007 tokens), `docs/protocol/v2.md`
- `crates/holler-hub/src/circuit/dispatch.rs`, `crates/holler-hub/src/token.rs`, `crates/holler-hub/src/lockout.rs`
