# Handoff-F: Phase 6 (F) - #637 slice a, the `holler-pane` crate (IMPLEMENT / GREEN)

**Date:** 2026-10-09
**Branch:** issue-637-implementation (on top of cfa96c8)
**Issue:** #637 (slice a of epic #633's skeleton). Brief: docs/handoffs/637-brief.md revision 6. RED: docs/handoffs/637/handoff-T-red.md. Plan review: docs/handoffs/637/handoff-A.md (PASS).
**Result:** done. T's 48 tests and the two `compile_fail` doctests pass, plus the 2 `holler-proto` tests, and every gate of AC 1 passes. No test was written, edited or deleted. Nothing was committed or staged (the run's phase commit does that).

## What was done

New crate `crates/holler-pane` (`serde`, `serde_json`, `holler-proto` by path; no async runtime; `[lints] workspace = true`):
- `Cargo.toml`: the manifest.
- `src/lib.rs`: declares every module (so no later story edits it), the crate docs (module map, "frozen when #637 merges", the serde policy) and the flat root re-exports.
- `src/error.rs`: `PaneError` (one variant per closed code plus `Refused`), `code()`, `Display`, `ALL_CODES`, `is_valid_code` (a `const fn`), `RefusalCode` and `RefusalCodeError`; a private single-source `PaneCode` table; the wire and serde helpers shared by the other modules (`from_wire`, `from_decode`, `coded_message`, `deserialize_parsed`, `excerpt`). Carries the two `compile_fail` doctests.
- `src/reply.rs`: `PaneReply` (`success`, `failure`, `into_result`), `ReplyError`, `decode_params`, the params structs of the twelve methods and `WatchReply<E>`.
- `src/pane.rs`: `Pane` and its records (`HerdrPane`, `HostInfo`, `HarnessInfo`, `Health`, `Hold`, `LastObserved`, `ModelSpec`, `ContextCeilings`, `PaneProbe`, `PaneRole`, `HarnessKind`), `PaneName`, `PaneId`, `PaneEvent`.
- `src/profile.rs`: `Profile`, `ProfileSpec` (and `SpecHerdr`, `SpecHost`, `SpecHarness`), `ProfileName` with the one `slug()`, `Actor`, `SpecEdit`, `ResolvedScope`, `ProfileLogEntry`, `ProfileChange`, `ProfileEvent`, and the `ProfileStore` and `ProfileScope` traits.
- `src/generation.rs`: `next_generation`, the compare-and-swap rule.
- `src/grid.rs`: `GridPos` (`parse`, `Display`, serde).
- `src/argv.rs`: `Argv` and `EnvVarName`.
- `src/ports.rs`: `Cursor`, `Watch<T>`, `PaneStore`, `HerdrPort` (with `HerdrSpec`, `HerdrSnapshot`, `Key`), `HostPort`, `HarnessPort`, `Prober`, `SystemProber`, `Ports`.
- `src/probe.rs`: `ProbeResult` and the `run_probe` stub (answers `ProbeResult::Error`, never `Ok`; #663 builds the body).
- `src/profile_snapshot.rs`, `profile_diff.rs`, `tx_apply.rs`, `tx_launch.rs`, `tx_switch.rs`, `reconcile.rs`, `findings.rs`, `import.rs`: empty stubs, one `//!` line each naming the owning story.

Four empty crates (`Cargo.toml` with `[lints] workspace = true` and no dependency, `src/lib.rs` a doc comment only): `crates/holler-adapter-herdr`, `crates/holler-adapter-host`, `crates/holler-adapter-opencode`, `crates/holler-pane-testkit`.

Edited:
- `crates/holler-proto/src/methods.rs`: `PANE_METHODS` (5 names, with `pane/delete`), `PROFILE_METHODS` (7, `profile/rename` PROPOSED), `is_pane_method`, `is_profile_method`, outside `CATALOG`; the module doc says so and names `holler_hub::serve::CONTROL_METHODS`. No production line outside those; T's in-file `mod tests` is untouched. No root re-export, `vocab.rs` and every golden file untouched.
- `CHANGELOG.md`: an `### Enhancements` entry under `## [Unreleased]` linking #637 and #633.
- `Cargo.lock`: the five new crates only.

## Design decisions

The full list, with alternatives, is in decisions.md (F entry). The ones that freeze at merge:
- **Error taxonomy.** A private `PaneCode` enum is the one table (`ALL_CODES`, `code()` and the wire parse-back all derive from it; A row 2a). `ALL_CODES` is a `&[&str]` const, so adding a code is not a type change. `NotFound { what }` of the brief is split into `PaneNotFound` and `SessionNotFound` (a `String` cannot choose a code; A row 2b). `ProfileDrift` has a variant no port returns (A row 2c).
- **Variant shapes.** Unit: `NotImplemented`, `Conflict`, `CommandNotArgv`, `EnvNameInvalid`, `ProfileSecretRefused`. Every other closed variant carries one `String` (`what`, `message` or `op`), which travels as the wire `detail`, so `PaneError -> PaneReply -> PaneError` is lossless for every closed code (A row 2d). The wire `message` is always `Display`.
- **`RefusalCode`**: private `Cow<'static, str>`; `from_static` const-asserts the grammar and that the code is not closed; `parse(String)` returns `Result<_, RefusalCodeError>`. Both `compile_fail` doctests are T's text.
- **Unknown fields.** Records, events and params structs refuse unknown fields (fail closed; a peer that does not know a field would otherwise drop it on the next CAS write-back). The `PaneReply` envelope ignores them (a client never writes it back). `None` options are omitted and default to absent.
- **Slug** is ASCII (lower-case ASCII letters and digits, other runs one `-`). `Profile.slug == name.slug()` is checked on read. Invalid `PaneName`, `ProfileName`, `Actor` give `usage`.
- **Watch contract.** `Cursor(0)` is "from the beginning" (a put for every current record, then changes); each event is `{cursor, name, pane|profile or null}` (null is a deletion); an idle `next()` yields `Err(Timeout)` and the stream stays usable.
- **Helpers in one place** (A rows 6, 7): `generation::next_generation`, `reply::decode_params`, `Argv::from_json`.
- **Not typed here:** the `data` shape of get/list/cas_put/delete/log/rename (the issue lists params only; #639, #661, #649 agree on it against ADR-0021; `reply.rs` says so).
- **Ports** are T's pinned signatures, unchanged; their data types are minimal and provisional (`HerdrSpec`, `HerdrPane`, `HerdrSnapshot`, `PaneId`, `Key`).

## Reuse / extend-vs-new

- `PaneName` is a newtype over `holler_proto::vocab::SessionName` and calls `SessionName::parse`; the grammar is not copied and `vocab.rs` is not edited (evidence.md entries 1 and 2).
- The error table follows `holler_proto::Code` (one enum, one `ALL`, one `match` per mapping); the validator `is_valid_code` is the only one in the workspace (a search of `crates/` found no other kebab-case validator or slug helper).
- `holler_proto::clock::now_millis` is not used: this slice only holds `i64` timestamp fields and reads no clock.
- The brief's Reuse map named no existing object to extend for the crate; the new objects are the brief's own (decisions 2, 3, 7, 8). No parallel path exists.

## Architecture notes for A

- **Layers.** `holler-pane` is a new leaf crate that depends only on `holler-proto`, `serde` and `serde_json`; `cargo tree -p holler-pane` shows no async runtime. Nothing depends on it yet (#669, #670 and the wave-3 stories will). `holler-proto` gains four items in `methods.rs` and no dependency.
- **New contract surface that freezes at merge:** the root re-exports (list in `lib.rs`), the closed code list, the variant shapes, the record JSON, the port signatures, `Ports`, the reply and params JSON, the watch semantics. `HerdrPort` and `HarnessPort` stay provisional until #636 and #635 report.
- **Module boundaries:** `ProfileStore` and `ProfileScope` in `profile.rs` (the epic and #663 say so), `Prober`/`SystemProber`/`Ports` in `ports.rs`, `ProbeResult`/`run_probe` in `probe.rs`. The module list is exactly the brief's.
- **Local patterns followed:** the single-source code table of `holler_proto::Code`; `deny_unknown_fields` on persisted documents (`docs.rs`, `config.rs`); `skip_serializing_if = "Option::is_none"` with `default` for optional fields (`hold.rs`); the empty-skeleton manifest template of `holler-body`.
- `archChanged` is reported `true`: new crates and a new public contract, and A has reviewed the plan, not these shapes.

## Deviations from spec / wireframe

- No wireframe (no UI). No deviation from an acceptance criterion.
- From the brief's text: `NotFound { what }` became two variants (above); `ProfileStore` and `ProfileScope` sit in `profile.rs`, not `ports.rs`; the pane role type is `PaneRole`, not `Role`. All three are A's PASS notes and are recorded in decisions.md.

## Tier 1 self-check (incl. tests now GREEN)

Run on the final tree, in the worktree:

```
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s)      (exit 0)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)      (exit 0, no warning)

$ cargo test -p holler-pane --no-fail-fast        (T's tests, now GREEN)
    tests/argv_env_test.rs   test result: ok. 6 passed; 0 failed
    tests/error_test.rs      test result: ok. 12 passed; 0 failed
    tests/grid_test.rs       test result: ok. 5 passed; 0 failed
    tests/names_test.rs      test result: ok. 8 passed; 0 failed
    tests/ports_test.rs      test result: ok. 6 passed; 0 failed
    tests/records_test.rs    test result: ok. 11 passed; 0 failed        (48 tests in all)
    Doc-tests holler_pane    error::RefusalCode::from_static - compile fail ... ok
                             error::PaneError::Refused - compile fail ... ok
                             test result: ok. 2 passed; 0 failed

$ cargo test -p holler-proto --lib methods
    methods::tests::is_pane_method_and_is_profile_method_classify_by_list ... ok
    methods::tests::pane_methods_not_in_wire_catalog ... ok

$ cargo test --workspace --no-fail-fast
    91 "test result" lines, all ok: 911 passed; 0 failed; 5 ignored (the ignored ones are not mine)

$ cargo machete
    cargo-machete didn't find any unused dependencies in this directory. Good job!

$ bash scripts/lint.sh                                   (exit 0)
    warn: crates/holler-pane/src/error.rs is 608 lines   (the only new line; warn at 600, fail at 900)

$ bash scripts/changelog-check.sh
    changelog-check: ok

$ bash scripts/golden-diff-summary.sh                    (no output, exit 0: no golden drift)

$ rustfmt --check --edition 2021 <each new .rs file, and methods.rs>   (all pass; no existing file reformatted)
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane --no-deps       (clean)
$ gitleaks dir <each changed path>                                    (no leaks)
$ git diff --name-only origin/main (plus untracked)                   (only Blast-radius paths)
```

Also checked in a scratch crate in the session scratchpad (nothing in the worktree): the `compile_fail` cases fail with E0277 and E0080 (a third, a closed code in `from_static`, fails with E0080), a private field blocks direct construction (E0423), and `decode_params`, `Argv::from_json`, `next_generation`, the slug mismatch, malformed replies, all 22 parse-backs and the event and log JSON behave as documented.

## Evidence appendix

docs/handoffs/637/evidence.md (five entries: `SessionName`'s derives and `parse`, the closed 22-row `CATALOG` and `find`, the workspace lint set and CI's `-D warnings`, the workspace `serde` line).

## Tests that look wrong (for T)

None is wrong; every test passes against this implementation. Notes for T's GREEN pass:
- **Adopted by F, not covered by the RED tests** (A rows 2, 4, 6-9; T said it would add a test for any of these): unknown-field refusal on `Pane`/`Profile`/`ProfileSpec`/params/events; `Profile.slug == name.slug()` on read; the ASCII slug (for example a non-ASCII letter is a separator) and `usage` for an invalid name; `generation::next_generation`; `reply::decode_params` (a guard failure keeps its code; anything else is `usage`); `Argv::from_json`; the params structs, `WatchParams`, `WatchReply`, `PaneEvent`, `ProfileEvent`, `ProfileLogEntry`/`ProfileChange` JSON; `PaneReply::into_result` on a malformed reply (`ok` and `error` disagree is `unavailable`); `GridPos` with `pos` absent (accepted) and with leading zeros (accepted); `EnvVarName` with a control character (`env-name-invalid`).
- **Doctests:** both `compile_fail` blocks pass; I confirmed each fails for the right reason (E0277, E0080) in a scratch crate, but T should repeat the "remove the privacy" check the RED handoff names.
- **`HarnessPort::health` returns `bool`** (T's pinned shape) while `Pane.harness.health` can hold `Unhealthy(reason)`; a `Health` return would carry the reason. Not a test error: provisional until #635 reports. No change made.

## Known issues

- `crates/holler-pane/src/error.rs` is 608 lines, so `scripts/lint.sh` prints a warn (not a failure; the limit is 900). The three exhaustive matches over the 23 variants are the bulk and are deliberate.
- A closed variant without a payload (`not-implemented`, `generation-conflict`, ...) is rebuilt from its code alone on parse-back; a peer's richer message text is not kept. #669's stub can name its story only in the wire `message`.
- `RefusalCode::from_static` panics when called at run time with an invalid literal (the build-time check holds in a `const`); documented on the function.
- The `# for` consumer markers on the workspace `serde` and `serde_json` lines (`Cargo.toml:44`, `:48`) do not list `holler-pane`. The workspace `Cargo.toml` is outside this story's blast radius and `lint.sh` check 5 passes; a comment-only follow-up edit is O's call.
- No `docs/` change (the brief's radius excludes it): the control-socket method names are by design not in `docs/protocol/v2.md`; ADR-0021 (#634) documents them.
- Nothing else fails an acceptance criterion.

## Files changed

Production (created):
- crates/holler-pane/Cargo.toml
- crates/holler-pane/src/lib.rs
- crates/holler-pane/src/error.rs
- crates/holler-pane/src/reply.rs
- crates/holler-pane/src/pane.rs
- crates/holler-pane/src/profile.rs
- crates/holler-pane/src/generation.rs
- crates/holler-pane/src/grid.rs
- crates/holler-pane/src/argv.rs
- crates/holler-pane/src/ports.rs
- crates/holler-pane/src/probe.rs
- crates/holler-pane/src/profile_snapshot.rs
- crates/holler-pane/src/profile_diff.rs
- crates/holler-pane/src/tx_apply.rs
- crates/holler-pane/src/tx_launch.rs
- crates/holler-pane/src/tx_switch.rs
- crates/holler-pane/src/reconcile.rs
- crates/holler-pane/src/findings.rs
- crates/holler-pane/src/import.rs
- crates/holler-adapter-herdr/Cargo.toml
- crates/holler-adapter-herdr/src/lib.rs
- crates/holler-adapter-host/Cargo.toml
- crates/holler-adapter-host/src/lib.rs
- crates/holler-adapter-opencode/Cargo.toml
- crates/holler-adapter-opencode/src/lib.rs
- crates/holler-pane-testkit/Cargo.toml
- crates/holler-pane-testkit/src/lib.rs

Production (modified):
- crates/holler-proto/src/methods.rs
- CHANGELOG.md
- Cargo.lock

Pipeline artifacts: docs/handoffs/637/handoff-F.md, docs/handoffs/637/evidence.md, docs/handoffs/637/decisions.md (F entry appended).
