<!-- Story: #NNN — one story per PR. Title: "<verb> <what> (#NNN)". -->

## What

<!-- One or two sentences. What changes, and what does NOT (if that is surprising). -->

## Why

<!-- The story line, or the defect this fixes. Link the issue. -->

## Checklist (the build guards — #149/#155)

- [ ] `bash scripts/lint.sh` passes locally
- [ ] `cargo clippy --all-targets -- -D warnings` passes locally
- [ ] `cargo test --workspace` passes locally
- [ ] `cargo machete` reports nothing
- [ ] Every new `#[allow(...)]` names its issue (`// #NNN`)
- [ ] Every new `#[ignore = "..."]` names its issue
- [ ] If the wire changed: the golden files under `crates/holler-proto/tests/golden/` changed **on purpose** (`BLESS=1`, diff reviewed) and `docs/protocol/v2.md` matches
- [ ] If the CLI changed: `crates/holler-cli/tests/fixtures/cli-surface.txt` (or `.pending.txt`) changed with it, and ADR 0003 matches
- [ ] If a doc shows a `holler …` command, it parses (`cargo test -p holler-cli --test docs_cli_test`)
- [ ] No file crossed 600 lines (900 for tests) without a `// #NNN` split issue

## Test evidence

<!-- Paste the relevant `test result:` lines, or the hlr-NNNN catalog ids this PR turns green. -->
