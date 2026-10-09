# Decisions — #637 pane-control skeleton

## A (Phase 3, up-front plan review) — 2026-10-08T21:15:21-06:00
- **Decided:** BLOCK on docs/handoffs/637-brief.md at 8fd92f8, with 6 blocks and 11 warns (see handoff-A.md).
  - The overall shape is right: a new `holler-pane` domain crate, stubs per verb, `CATALOG` kept at 22, clap structs moved out of `cli.rs`, and verbs run CLI-side against ports.
  - The six blocks are plan defects that T and F cannot fix inside the brief's blast radius:
    1. The `main.rs` dispatch and exit point is unlisted, and `main()` is at 99 of clippy's 100 deny-level lines.
    2. Decision 7 puts error codes in the frozen `vocab.rs` but names only 5, while the epic and its siblings name 8+ more, and #660's fixed text puts codes in each verb's file.
    3. Decisions 5 and 6 contradict each other on `holler-pane`'s dependencies, because `SessionName` lives in holler-proto.
    4. The hub hook is never reached: `dispatch_control` refuses non-`control/` methods, no state plumbing exists from serve.rs, and the hub `Cargo.toml` lacks `holler-pane`, which neither #639 nor #661 may add.
    5. AC 4's `say --pane X TEXT` cannot parse without a positional redesign whose consumers are out of radius.
    6. ADR 0003 is left contradicted, and #634's radius excludes it.
- **Decided:** `mod.rs` under `src/` deviates from the codebase's `foo.rs` + `foo/` convention. Recorded as a justified deviation (warn 14), because it keeps each module root inside its owner's glob. Phase 7 and S should not flag it.
- **Assumed:**
  - Verbs execute in the CLI process, so `pane/*` holds only the four `PaneStore` operations. This is inferred from #644's blast radius (`launch.rs`, `relaunch.rs`, `holler-pane/src/tx_launch.rs`). #649's "wire the real adapters into the hub's `pane/*` handlers" reads the other way; O/the operator should confirm.
  - The epic's contract section and the sibling issues' fixed text (#639, #643, #644, #646, #649, #660, #661, #663) are as fetched from GitHub on 2026-10-08.
- **Hedged:**
  - Row 6 (ADR 0003) is a block under the stack's ADR rule and the established practice, where every verb-adding story updated ADR 0003. The operator can instead move it to #634 by amending #634's radius, accepting that the ADR lags the code.
  - Rows 8-10 (port signatures, `Prober`, verb entry and `Ports` bundle) are warns, not blocks, because the brief can let F settle them. But #637 freezes them, and #638 implements them in parallel.
- **Evidence:**
  - Read: main.rs (full), cli.rs (full), lib.rs, say_cmd.rs, interrupt_cmd.rs, answer_cmd.rs, hold_cmd.rs; control_server.rs:1-200; serve.rs:565; holler-proto vocab.rs, methods.rs, lib.rs, hold.rs and error.rs header; scripts/lint.sh; clippy.toml; ci.yml machete step; holler-cli and holler-hub `Cargo.toml`; cli_surface_test.rs, docs_cli_test.rs and both fixtures; ADR-0003.md table and its `git log`.
  - Fetched with `gh issue view`: #633, #634, #637, #638, #639, #643, #644, #646, #649, #660, #661, #663.
  - Counted `main()` code lines the way clippy's `too_many_lines` does (99). Ran a scratch crate in the scratchpad pinned to the workspace's clap 4.6.6 to test the `say`/`interrupt`/`answer` `--pane` shapes; no worktree files were touched.

## A (Phase 3 re-review, brief revision 2) — 2026-10-08T21:49:01-06:00
- **Decided:** BLOCK on docs/handoffs/637-brief.md at d63d278 (revision 2), with 2 blocks and 9 warns. See handoff-A.md, which replaces the revision-1 review; that review is kept in git at 16c27d7.
  - All six revision-1 blocks are answered, and each was re-verified against the code.
  - Block 1: `pane/args.rs` and `profile/args.rs` are frozen, but they declare no positionals and none of the verb-specific flags that #643-#647, #650 and #662-#665 need. Each of those stories' radius holds only its own verb files.
  - Block 2: the closed `PaneError` list misses `profile-exists` and `profile-has-live-panes`. It also has no variant for timeouts, missing records, corrupt stores or unreachable services, all of which the port implementers (#638-#642, #649, #661) must return.
- **Decided:** the hub stubs answering with the CLI envelope is a warn, not a block (row 3). Its sharers (#639, then #661, then #649) run in sequence, not in parallel. It is still a layering and duplication risk.
- **Assumed:**
  - The sibling issue texts are as fetched with `gh issue view` on 2026-10-08. The latest edit to any of them was at 19:06 MDT, before revision 2 was committed at 21:26 MDT.
  - Decision 0's knock-on issue amendments (row 9) are not posted yet; the brief says they are drafted separately.
- **Hedged:**
  - Row 6 (the output API) is a warn because F can settle it. But #660 cannot change the signatures later, and an `emit()` that writes straight to stdout leaves every verb story's in-process JSON test without a seam.
  - Row 1 offers two fixes: enumerate the whole surface now, or move each verb's `Args` struct into its own verb file. The operator picks one.
- **Evidence:**
  - Read:
    - the brief (both revisions);
    - in holler-cli: main.rs, cli.rs, lib.rs, say_cmd.rs, and hold_cmd.rs (parts);
    - in holler-hub: control_server.rs (:1-420 and :697-760), serve.rs (:340-610), and the `load` path in holds.rs;
    - manifests and config: the holler-hub, holler-cli and holler-proto `Cargo.toml`, the workspace `Cargo.toml`, clippy.toml, and lint.sh;
    - in holler-proto: methods.rs, vocab.rs, clock.rs, and the error.rs table;
    - CLI tests: docs_cli_test.rs, cli_invocation_test.rs, the cli-surface fixtures, the support/mod.rs API, and hub_serve_test.rs;
    - ADR-0003.md.
  - Fetched #633-#667 with `gh issue view`, and grepped every kebab-case code they name.
  - Counted `main()`'s lines the way clippy does (99).
  - Ran a scratch crate on clap 4.6.6 to test the `--pane` tail with and without `trailing_var_arg`. No worktree files were touched.

## A (Phase 3 re-review, brief revision 3) — 2026-10-09T02:30:25-06:00
- **Decided:** BLOCK on docs/handoffs/637-brief.md at bbc28ee (revision 3), with 3 blocks and 7 warns. See handoff-A.md, which replaces the revision-2 review (kept in git at c33c255).
  - Both revision-2 blocks are answered: per-verb clap structs in their own verb files, and the completed error taxonomy. Both were re-verified against the code and issues #633-#667.
  - Block 1: the frozen verb/output seam does not type-check and cannot reach stderr.
    - `run(args, ctx: &VerbCtx)` cannot write to its writers (E0596).
    - `emit`, `emit_stream` and `emit_usage_error` take no error writer.
    - #660 must keep these signatures beside six wave-3 verb stories.
    - The missing writer came from my own revision-2 row 6 suggestion.
  - Block 2: `ProfileScope::resolve` requires a pane and returns one membership bit. #643, #646, #647, #648, #663 and #638 all need "no pane name = every pane of P" from this frozen trait.
  - Block 3: AC 1's `cargo fmt --check` cannot pass.
    - 176 of 195 `.rs` files fail it on origin/main, and CI does not run it.
    - Formatting `control_server.rs` and `serve.rs` puts them at 1161 and 949 lines, past the 900-line gate.
    - It has been in every revision; both earlier reviews missed it.
- **Decided:** row 4 (`PaneError` wire parse-back) is a warn, not a block. Its only consumer, #649, runs in wave 4, after the server stories, so the gap blocks no parallel work.
- **Decided:** row 7 (no path from `PaneState` to `send_prompt`) is a warn. The directive is right under the stack's choke-point rule; only the amendment draft must name the plumbing and #646's radius.
- **Assumed:**
  - The sibling issue texts are as fetched with `gh issue view` on 2026-10-09. The latest edit to any of them was on 2026-10-08 at 19:06 MDT, before revision 3 was committed at 02:04 MDT on 2026-10-09.
  - The issue and epic amendments are drafted, not posted.
- **Hedged:**
  - Block 3 could be called an AC wording issue. I kept it a block for three reasons: AC 1 cannot be met by any implementation; it contradicts AC 1's own 900-line gate on two hot-spot files (the stack overlay's size rule); and a forced reformat would push whole-file churn into the files every sibling rebases onto.
  - Row 5: I confirmed the adjacent-line merge conflict in a scratch git repo. The separator layout is a suggestion; any layout that leaves one unchanged line between owners works.
- **Evidence:**
  - Read:
    - the brief (revision 3, plus the revision 2-to-3 diff), and the earlier handoff and decisions;
    - in holler-cli: `main.rs`, `cli.rs`, `lib.rs` and `say_cmd.rs`;
    - in holler-hub: `control_server.rs` :1-200 and :697-740, `serve.rs` :330-590, `state.rs`, the `holds.rs` `load` path, `live.rs` (Registry), `circuit/dispatch.rs` (the `send_prompt` header), `control.rs` (`ControlCall`, `run`) and the hub `Cargo.toml`;
    - in holler-proto: `methods.rs`, `vocab.rs` (`SessionName`) and `clock.rs`;
    - config and gates: the workspace `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `scripts/changelog-check.sh` and the `ci.yml` steps;
    - CLI tests: `cli_surface_test.rs`, `docs_cli_test.rs`, `cli_invocation_test.rs`, the `fixtures/cli-surface*.txt` files and the `support/mod.rs` API;
    - docs: `ADR-0003.md`, and `v2.md` §4 and §10.
  - Fetched #633-#667 with `gh issue view`, and grepped every kebab-case code they name.
  - Ran, in the scratchpad only (no worktree files touched):
    - `cargo fmt --check` (176 dirty files), and rustfmt on scratch copies of the hub files;
    - `bash scripts/lint.sh` and `cargo machete` (both clean on the base);
    - a rustc probe of the `VerbCtx`/`run` shapes (E0596);
    - a clap 4.6.6 crate testing the tail with flags between the positionals (all parse);
    - a git repo testing adjacent-line merges (conflict) against a one-line gap (clean).

## A (Phase 3 re-review, brief revision 5) — 2026-10-09T03:20:46-06:00
- **Decided:** PASS on docs/handoffs/637-brief.md at 98ff7ce (revision 5, slice a only), with 0 blocks and 13 warns. See handoff-A.md, which replaces the revision-3 review (kept in git at ce0e68b). Revision 4 was never reviewed; the split replaced it.
  - None of the three revision-3 blocks applies to slice a. The verb/output seam moved to #670. `ProfileScope::resolve` takes an optional pane and returns the panes in scope. The fmt gate reads "new files rustfmt-clean".
  - The plan's shape matches the codebase. A domain crate depends only on serde, serde_json and holler-proto. It reuses `SessionName` and `clock::now_millis`, leaves `CATALOG` and the golden files unchanged, and needs no workspace `Cargo.toml` edit.
- **Decided:** row 1 (`Refused` can be built around `refused()`) is a warn, not a block.
  - F can make the brief's own "built only through" rule true inside the radius, with a validated code newtype or `#[non_exhaustive]`.
  - If it is left as is, #670's validated `ErrorCode` still catches a bad code at the CLI's output boundary, so the gap costs enforcement at construction time, not correctness.
  - It is still the cheapest row to fix now: a later fix breaks every outside construction site in wave 3.
- **Decided:** row 12 (no way to remove a pane record) is a warn, not a block. The contract already has a plausible tombstone, `hold: Drained`. Only the "delete" answer changes this story (the trait and AC 7's list), so the decision must come before merge.
- **Decided:** row 11's placement of `PANE_METHODS` in holler-proto, beside a holler-hub `CONTROL_METHODS` precedent, is a recorded deviation rather than drift. The issue and epic ruling 6 fix the location. Only the module doc and the test location need changing.
- **Assumed:**
  - The issue and sibling texts are as fetched with `gh issue view` on 2026-10-09. #637 was last edited at 02:53 MDT, before revision 5 was committed at 02:55 MDT. #639, #646-#649, #665 and #667 were edited at 03:02 MDT, after it, with split notes that do not change slice a.
  - The operator's "option 1" relayed with this run is the split (#637 slice a); this review covers slice a only.
- **Hedged:**
  - Rows 3, 5, 10 and 12 depend on sibling issues (#661, #665, #670) or on an operator decision. They are warns because #637's plan matches its own issue text, and each sibling can work around the gap within its own radius, at the cost of duplication or a breaking change later.
  - Row 4's event shapes are suggestions. Any shape works that carries a cursor per event and can express a profile deletion.
- **Evidence:**
  - Read:
    - the brief (revision 5, and revision 4's decision 8 at 182a69f), the split proposal, and the earlier handoff and decisions;
    - holler-proto: `lib.rs`, `methods.rs`, `vocab.rs`, `error.rs`, `hold.rs`, `clock.rs`, and the catalog tests in `tests/codec_test.rs`;
    - holler-hub: `serve.rs:55-80`, `control_server.rs:1-130` and `:697-730`, and `control_hold.rs`;
    - config, gates and docs: every crate's `Cargo.toml`, the workspace `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `changelog-check.sh`, `golden-diff-summary.sh`, the `ci.yml` steps, `docs/testing.md` (Layout), the ADR index and ADR 0006, and the outline of v2.md.
  - Fetched #633-#670 with `gh issue view`, and checked that every `holler-pane/src/*.rs` path they name is in the brief's module list.
  - Ran, in the scratchpad only (no worktree files touched), a two-crate scratch workspace on rustc and clippy 1.98.1:
    - the plain `Refused` variant built from outside the crate, and with `#[non_exhaustive]` refused (E0639);
    - a const-validated `RefusalCode` (E0080 on an invalid literal, clean under `-D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -D warnings`);
    - `Ports`/`VerbCtx`/`edit_spec` with a port-calling `act` closure (compiles, `Send + Sync`);
    - `empty_docs`, `large_enum_variant` and `result_large_err` all fatal under `-D warnings`.

## T (Phase 4, AUTHOR / RED) — 2026-10-09T03:45:00-06:00
- **Decided:** RED is valid for brief revision 6. 48 tests in six `crates/holler-pane/tests/*.rs` files plus 2 in `crates/holler-proto/src/methods.rs`; see handoff-T-red.md for the table, the pinned API and the RED output.
  - The holler-pane RED is a compile failure (E0432 on every item the feature defines). That is unavoidable for a crate that does not exist; it is the RED the brief names. The holler-proto RED is assertion-level against a minimal stub.
  - Round trips are written as JSON-in/JSON-out against the epic's contract fixtures, so the tests pin the wire form (what #639/#661/#649 depend on) without pinning constructors for the records.
  - `PaneError` variants are reached through the wire parse-back (a reply with a closed code), not constructed, except the four whose shape the brief states (`NotImplemented`, `Conflict`, `Timeout { op }`, `Unavailable { what }`) and `Refused`. This leaves F free to settle the other variant shapes (A rows 2) and still checks every closed code.
  - `SpecEdit::Set` takes a `Box<ProfileSpec>`: `large_enum_variant` is fatal under `-D warnings`.
- **Assumed:**
  - Root-level re-exports of every public item (the shape #669 already uses: `holler_pane::Pane`, `PaneError`, `PaneReply`), except `error::{is_valid_code, ALL_CODES, RefusalCode}`.
  - `ProfileName::parse` trims surrounding whitespace; "trimmed" in decision 3 could also mean "refuse padded input". Trimming is the simpler reading and the slug rule already trims separators.
  - `ALL_CODES` includes `profile-drift` (22 codes), with a `ProfileDrift` variant, so the closed list, `ALL_CODES` and the variants stay one-to-one (A row 2c, first option).
  - `HostPort`/`HarnessPort`/`SpecEdit`/`PaneReply` method and type shapes as listed in handoff-T-red.md, because the brief names the methods but gives no signatures. They stay provisional until compiled, as decision 7 says.
- **Hedged:**
  - The AC 5 `compile_fail` doctest cannot be authored by T (doctests live in `src/`, a production file). The exact text is in the handoff; T verifies it at GREEN. If F writes it differently, T checks it still fails for the right error code (E0277 / E0080).
  - The stale-CAS test (AC 4) runs against an in-test store, as the brief says, so it proves the contract and the error code, not a library check. A shared `generation::cas` helper (A row 7b) would let the AC test library code; if F adds one, T adds a test for it at GREEN.
  - The throwaway scaffold used to see the E0432 RED, and the stub for the `methods.rs` items, are removed again; the worktree holds only the test files, the handoff and this entry.
- **Evidence:**
  - Read: the brief (revision 6), handoff-A.md, decisions.md, `gh issue view 633` and `637`, `holler-proto` `methods.rs`, `vocab.rs` (`SessionName`, `check_segment`), `clock.rs`, `lib.rs`, `Cargo.toml` (workspace lints), `clippy.toml`, `scripts/lint.sh`, existing test-file `allow` conventions.
  - Ran: throwaway reference crate in the scratchpad (path dependency on the worktree's `holler-proto`, symlinked `tests/`): `cargo test` 48/48, `cargo clippy --all-targets -- -D warnings` clean, 17 mutations all killed; `rustfmt --check --edition 2021` on every new file; `bash scripts/lint.sh` passes; `cargo test -p holler-pane --no-fail-fast` on an empty-lib scaffold (RED output in the handoff); `cargo test -p holler-proto --lib methods` against an empty stub (RED) and a real stub (GREEN).

## F (Phase 6, IMPLEMENT / GREEN) — 2026-10-09T04:15:00-06:00
- **Decided:** slice a is implemented against brief revision 6 and T's RED. `cargo test -p holler-pane` is 48 of 48 green plus the two `compile_fail` doctests, `cargo test -p holler-proto --lib methods` is 2 of 2, and the whole workspace suite, workspace clippy (`-D warnings`), `cargo machete`, `lint.sh` and `changelog-check.sh` pass. No test was edited. The calls the brief leaves to F (decision 7's latitude and A's rows 2, 4, 6-9, 13), each reversible until #637 merges:
  1. **Error taxonomy.** A private single-source enum `PaneCode` (22 closed codes) derives `ALL_CODES` (a `const` slice, built in a `const` block, so adding a code is not a type change), `PaneError::code()` and the wire parse-back, the way `holler_proto::Code::ALL` does (A row 2a). `profile-drift` has a `ProfileDrift` variant that no port returns (A row 2c). The brief's `NotFound { what }` is split into `PaneNotFound { what }` and `SessionNotFound { what }`, because one `String` field cannot choose between two codes (A row 2b).
  2. **Variant shapes.** `NotImplemented`, `Conflict`, `CommandNotArgv`, `EnvNameInvalid` and `ProfileSecretRefused` are unit variants (the last three so no refused text is echoed). Every other closed variant carries one `String`: `what` (the subject), `message` (free text) or `op` (`Timeout`). That string is the wire `detail`, so `PaneError -> PaneReply -> PaneError` is lossless for every closed code (A row 2d). `Display` is `"<phrase>: <payload>"`, and the wire `message` is always `Display`. `Refused { code, message }` displays as `message` alone.
  3. **`RefusalCode`** wraps `Cow<'static, str>` with a private field and no `From`/`Default`. `from_static` const-asserts the grammar and also that the code is not closed (one representation per code); `parse(String)` returns `Result<_, RefusalCodeError>` (`Invalid` or `Closed`). The two `compile_fail` doctests are T's text, on `PaneError::Refused` (E0277) and `RefusalCode::from_static` (E0080); I confirmed both fail for those codes in a scratch crate.
  4. **Unknown-field policy** (A row 8, decision 3 left it open). Records (`Pane`, `Profile`, `ProfileSpec` and every struct or enum inside them), events and the params structs refuse unknown fields: a record read, changed and written back by a peer that does not know a field would drop it silently, and the stack rule is fail closed. The `PaneReply` envelope (and `ReplyError`) ignores unknown fields: a client never writes it back. Optional fields are omitted when `None` and default to absent on read; `Vec` fields default to empty.
  5. **Slug is ASCII** (A row 13a): lower-case ASCII letters and digits, every other run is one `-`. A name with no ASCII letter or digit is refused (`usage`), so a name in another script alone is refused. Invalid `PaneName`, `ProfileName` and `Actor` give `usage` (A row 13c). `ProfileName` and `Actor` trim surrounding whitespace, refuse control characters and more than 64 characters; `Actor` and `ProfileName` refuse empty.
  6. **`Profile.slug == name.slug()` is checked on read** (A row 8): `#[serde(try_from)]` over a private `RawProfile`; a mismatch does not load.
  7. **`GridPos`**: `pos` is optional on read and, when present, must parse to the same cell (a spelling difference such as `R2C1` is not a disagreement); leading zeros are accepted (`r02c1`); a zero cell is `grid-out-of-range` through serde too. The parser returns `grid-ambiguous` for any structure it cannot read before it looks at range.
  8. **`Argv`** is `Vec<String>` behind `Argv::new`/`as_slice`/`into_vec`, deserialized through a JSON value so every non-array-of-strings shape (not only a string) is `command-not-argv`; `Argv::from_json(&str)` gives the CLI's `--command-json` the typed error (A row 7a). An empty argv is allowed (the verbs that need a program refuse it). **`EnvVarName`**: `=` is checked first (`profile-secret-refused`), then empty, whitespace or control characters (`env-name-invalid`).
  9. **Watch contract** (A row 4): `Cursor(0)` is "from the beginning" (a put for every current record, then later changes); each event is `{cursor, name, pane|profile or null}`, null meaning deleted, so a deletion is expressible now that `PaneStore::delete` exists; an idle `next()` yields `Err(Timeout)` and the stream stays usable. `WatchReply<E> { events, cursor }` and `WatchParams { since }` are in `reply.rs`.
  10. **Shared helpers, one place each** (A rows 6 and 7): `generation::next_generation(current, expected)` (the CAS rule, `Conflict` or the bumped generation), `reply::decode_params::<T>(Value)` (a guard failure maps to its own code via the `"<code>: ..."` prefix every serde error of this crate carries, anything else to `usage`), `Argv::from_json`. The JSON-RPC-versus-`PaneReply` split is written in `reply.rs`'s module doc (A row 6).
  11. **Placement** (A row 9): `ProfileStore` and `ProfileScope` in `profile.rs` (the epic and #663 say so); `Prober`, `SystemProber` and `Ports` in `ports.rs`; `probe.rs` keeps `ProbeResult` and `run_probe` with the frozen-by-#637 doc. The pane role is `PaneRole`, so nothing re-exported at the root shares a name with `holler_proto` (its `Role` is the A2A role). Each empty stub has a `//!` naming its owner (A row 9e). `run_probe` and `SystemProber` answer `ProbeResult::Error`, never `Ok`.
  12. **Ports** are T's pinned signatures, unchanged. The data types the brief calls minimal are `HerdrSpec { session, workspace, grid }`, `HerdrPane { session, workspace, pane_id, grid }` (also the `Pane.herdr` record), `HerdrSnapshot { panes }`, `PaneId(String)` and `Key(String)`. `rename` has no default body (the brief says implementers answer `not-implemented` themselves).
  13. **`holler-proto`**: `PANE_METHODS` and `PROFILE_METHODS` are `&[&str]` consts, `is_pane_method`/`is_profile_method` are `contains`; the module doc is extended as the brief says; no root re-export, no edit to `vocab.rs` or `lib.rs`.
  14. **Result `data` shapes** of `pane/get`, `list`, `cas_put`, `delete` and of `profile/get`, `list`, `cas_put`, `delete`, `log`, `rename` are not typed here (the issue lists the params structs only); #639, #661 and #649 agree on them against ADR-0021, and `reply.rs` says so.
- **Assumed:**
  - T's "API the tests pin" in handoff-T-red.md is the contract; the `HostPort`/`HarnessPort`/`HerdrPort` signatures are T's proposals and stay provisional until #635/#636.
  - Slice a is the operator-approved unit (the 2026-10-09 split), so the implementer role's scope cap (about 6 files) was answered by that split and no further split is proposed: 27 new production files (plus edits to `methods.rs`, `CHANGELOG.md` and `Cargo.lock`), of which the 8 module stubs and the 8 files of the four empty crates are a few lines each.
  - The pipeline's stable toolchain is at least 1.80 (`str::trim_ascii`); the worktree builds with 1.98.1.
  - `holler_proto::clock::now_millis` is not needed in this slice: the records only hold `i64` fields and nothing here reads the clock.
  - The sibling issue texts are as fetched with `gh issue view` on 2026-10-09 (none names a `PaneError` variant beyond the brief's).
- **Hedged:**
  - A closed variant without a payload (`not-implemented`, `generation-conflict`, ...) is rebuilt from the code alone on parse-back, so a peer's richer message text is not kept (#669's stub can name its story only in the wire `message`).
  - `RefusalCode::from_static` panics if called at run time with an invalid literal (the build check applies in a `const`); documented, and clippy's `panic` deny does not cover `assert!`.
  - The unknown-field policy and the watch semantics are my reading of A's rows 4 and 8; ADR-0021 (#634) may amend them, and a change after merge goes through the epic's amend-first rule.
  - `HarnessPort::health` returns `bool` (T's pinned shape) while the record's `Health` has an `Unhealthy(reason)`; the spike #635 may change it. Not changed here because it is pinned by `ports_test.rs`.
  - Not covered by T's tests, so T adds tests at GREEN for what I adopted: unknown-field refusal, the slug check on read, the ASCII slug, `next_generation`, `decode_params`, `Argv::from_json`, `WatchParams`/`WatchReply`/`PaneEvent`/`ProfileEvent`/`ProfileLogEntry` shapes, `PaneReply` malformed cases.
  - `error.rs` is 608 lines: `lint.sh` warns at 600 and fails at 900; the three exhaustive matches are the reason and are intentional.
  - The operator's relayed request for this run was "Ask for decisions now before I go back to sleep". F found no blocking decision (every call above is inside decision 7's latitude and A's PASS notes), so it sent the main conversation a message listing the five that freeze at merge, and proceeded on those defaults rather than stalling the run.
- **Evidence:**
  - Read: the brief (revision 6), handoff-A.md, handoff-T-red.md, decisions.md, all six test files and `tests/common/mod.rs`, `gh issue view 633` and `637`, the bodies of #634-#670 searched for `PaneError`, `Cursor`, `Watch`, `Actor`, `PaneReply` and `ProfileLogEntry`, `holler-proto` `lib.rs`, `error.rs` (head), `vocab.rs`, `methods.rs`, `clock.rs`, `hold.rs`, the workspace `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `changelog-check.sh`, `golden-diff-summary.sh`, `docs/releasing.md` (CHANGELOG structure), `.github/workflows/ci.yml` (clippy and test steps), `.githooks/pre-commit`.
  - Ran: `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --no-fail-fast` (911 passed, 0 failed, 5 ignored, on the final tree); `cargo test -p holler-pane` (48 tests, 2 doctests); `cargo test -p holler-proto --lib methods`; `cargo machete`; `bash scripts/lint.sh` (exit 0; one new warn for `error.rs`); `bash scripts/changelog-check.sh`; `bash scripts/golden-diff-summary.sh` (no drift); `rustfmt --check --edition 2021` on every new `.rs` file and on `methods.rs`; `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane --no-deps`; `gitleaks dir` on every changed path (no leaks).
  - Scratchpad only, no worktree files: a scratch crate (path dependency on `holler-pane`) that exercised `decode_params`, `Argv::from_json`, `next_generation`, the slug mismatch, malformed replies, every closed code's parse-back and the event and log JSON shapes, and that confirmed the two `compile_fail` cases (E0277, E0080) and a third (a closed code in `from_static`) fail for the right reason.

## T (Phase 7, VERIFY / GREEN + Tier 2)

- **Decided:**
  1. The suite is GREEN and Tier 2 is clean, so the verdict is PASS. F's production code needs no change.
  2. T added `crates/holler-pane/tests/adopted_test.rs` (9 tests) for what F adopted from A's rows 2, 4, 6-9 (unknown-field refusal, slug-on-read check, ASCII slug, `next_generation`, `decode_params`, `Argv::from_json`, watch params/replies/events JSON, malformed-reply rule, optional `pos`/leading zeros, env control characters). One file, because they share no state with the RED files and keep each under the 900-line gate.
- **Assumed:** a mutation that the suite does not kill is only worth a finding when production code is wrong; the `RefusalCode` field is private today (F's E0423 check), so field privacy is advisory, not a block.
- **Hedged:** the "remove the privacy" check from the RED handoff does not fail either doctest (they pin `From<&str>` and the const assertion, not field privacy). T cannot edit `src/`, so a third `compile_fail,E0423` doctest is left as an advisory for O or F.
- **Evidence:** ran `cargo build/clippy/test --workspace`, `docs_cli_test`, `wire_selftest`, `cargo machete`, `lint.sh`, `changelog-check.sh`, `golden-diff-summary.sh`, `rustfmt --check` on all new files; five production mutations, each reverted with `git checkout` and each killed by a named adopted-behavior test; the sixth (privacy) survived; `git diff --name-only origin/main` inside the Blast radius.

## A (Phase 7, anti-duplication gate) — 2026-10-09T04:31:59-06:00
- **Decided:** PASS on the diff 27da2da..40b722b, with 0 blocks and 6 warns. See handoff-A-dup.md.
  - F extended every object the Reuse map named: `PaneName` wraps `SessionName`; no second clock; `error.rs` follows `holler_proto::Code`'s single-source table; no new dev-dependency.
  - No near-copy of the stack's Phase 7 candidates exists: the token store, `Lockout`, `Roster`, `log::emit`, `atomic_file` and the test harness.
- **Decided:** `decode_params` beside `holler_proto::typed_params` is a justified second helper, not a parallel path.
  - Control frames are not `Envelope`s, and `typed_params` drops the serde error, which would lose the guard's code.
  - Phase 3 row 6 asked for it.
  - Only its rule for absent `params` is missing (row 3).
- **Decided:** row 1 (`Watch` signals idle with `Err(Timeout)`) is a warn, not a block. It is not duplication. It corrects my own Phase 3 row 4 suggestion, which missed the hub's `control/wait` precedent: "nothing happened in this window" is an ordinary outcome, not an error. It goes to the operator because it changes the brief's fixed `Watch<T>` alias.
- **Decided:** row 5 (`HarnessKind` beside `HARNESS_IDS`) is a warn. The Reuse map did not name `HARNESS_IDS`, and the epic's non-goals support a closed set. It is still a second vocabulary with nothing linking the two lists.
- **Assumed:**
  - The run's phase commit stages this entry and handoff-A-dup.md; I committed nothing.
  - The sibling issue texts are as fetched with `gh issue view` on 2026-10-09 at about 04:30 MDT. #637, #638, #639 and #661 were last edited at 03:30 MDT.
- **Hedged:**
  - Rows 2-4 are doc and test changes with no decision needed, since #639's text already decides row 2.
  - Row 6 (`HerdrSpec` against `SpecHerdr`) is optional naming.
  - None blocks, because the verdict rule blocks only an unjustified parallel path.
- **Evidence:**
  - Read in full: every changed production file; `tests/common/mod.rs`, `ports_test.rs`, `adopted_test.rs` and `records_test.rs`; the helper lists of the other test files; the methods.rs, CHANGELOG and Cargo.lock diffs; the brief; and the A, F, T-red, T-green and evidence handoffs.
  - Compared against:
    - holler-proto: `error.rs`, `vocab.rs`, `lib.rs`, `envelope.rs`, `envelope/dispatch.rs`, `hold.rs`, and `log.rs:455-497`;
    - holler-hub: `control_server.rs` (`:1-140` and `:466-540`), `serve.rs:55-80`, `roster.rs` (`:190-240` and `:480-520`), `holds.rs:205-230` and `token.rs:405-427`;
    - holler-body: `config.rs` (`:1-60` and `:95-200`), `acp_driver/auth.rs:140-165`, `query.rs:225-250` and `http_attach_driver.rs`'s module doc;
    - every crate's `Cargo.toml`.
  - Grepped `crates/` for analogs of each new helper and type: excerpt or truncate, const string equality, slug, control-character checks, cursor, generation or CAS, watch or long-poll, argv, env names, probe, `try_from` and `deserialize_with`, harness ids and role enums.
  - Ran `bash scripts/golden-diff-summary.sh` (no drift) and a blast-radius filter over `git diff --name-only origin/main...HEAD` (nothing outside it).
  - Fetched #633, #637, #638, #639, #642, #643, #649 and #661.

## S (Phase 9, spec audit) — 2026-10-09T04:45:00-06:00
- **Decided:** REWORK (production), with two required changes. See handoff-S.md.
  - Preconditions are met: A returned PASS (and A-dup PASS), and T confirmed RED, then GREEN, with no blocking issues.
  - AC 1, 2 and 4 to 8 are met, each by a named test or a recorded gate.
  - **Required 1 (decision 7):** five traits in `ports.rs` lack the per-trait doc rule ("call from `spawn_blocking`; return within I5's bound or with `Timeout`"). They are `PaneStore`, `HerdrPort`, `HostPort`, `HarnessPort` and `Prober`. Their only copy is the module doc, while `ProfileStore` and `ProfileScope` carry the rule. The deviation is silent.
  - **Required 2 (AC 3, ruling 7, I7):** a bare-string `env` such as `"TOKEN=hunter2"` in a `Pane` or `ProfileSpec` is refused as `usage`, and the reply repeats the value in both `message` and `detail`. This contradicts `error.rs:19-21` and `argv.rs:7-12`, and siblings may not pre-scan.
- **Decided:** two operator decisions are surfaced rather than blocking, because the run can continue on the defaults.
  - D1 is A-dup row 1 (how `Watch` signals idle); the default is to keep `Err(Timeout)`, documented.
  - D2 is A-dup row 5 (`HarnessKind`); the default is to keep the enum.
  - The relayed operator request was "Ask for decisions now before I go back to sleep", so both are listed with options and defaults.
- **Decided:** A-dup rows 2-4 and T-green's advisory 1 are recommended for the same rework pass, but they do not block.
- **Assumed:**
  - T's recorded Tier 1 output reflects the tree at af72b6d. S did not re-run Tier 1 or Tier 2.
  - The issue and epic texts are as fetched on 2026-10-09 at about 04:35 MDT; both were last edited at 03:30 MDT.
- **Hedged:**
  - REWORK 1 is doc-only. It is REWORK, not advisory, because the brief states it as a decision ("each trait carries the doc rule") and the gap is not recorded.
  - REWORK 2's code for a non-array, non-string `env` (`env-name-invalid` or `usage`) is left to F. Two parts are required: a string carrying `=` is `profile-secret-refused`, and no env refusal repeats its input.
- **Evidence:**
  - Read every production file in `crates/holler-pane/src`, all test files, the `methods.rs`, CHANGELOG and Cargo.lock diffs, every handoff, decisions.md, evidence.md, `gh issue view 637`, `633`, `639` and `661`, `CONTRIBUTING.md`, `.githooks/*`, the workspace `Cargo.toml`, and holler-proto's `lib.rs`.
  - Ran `git diff --name-only origin/main...HEAD` (all paths inside the radius) and `wc -l` (the largest file is 608 lines).
  - Grepped the added lines for privacy patterns (none found) and ran `gitleaks stdin` over the diff (no leaks).
  - Checked the repo history: no session links in the last 200 main commits.
  - Two scratchpad probes, with no worktree file touched:
    - a scratch crate showed that stable rustdoc 1.98.1 does not check `compile_fail,E0xxx` codes;
    - a scratch crate with a path dependency on `holler-pane` reproduced the bare-string `env` echo, through serde, `decode_params` and `PaneReply::failure`.

## F (Phase 6 rework, after S's REWORK) — 2026-10-09T05:05:00-06:00
- **Decided:** both S-required changes are made and the suite is green (921 passed, 0 failed workspace-wide; clippy `-D warnings`, machete, lint.sh, changelog-check, golden diff, rustdoc all clean). See the rework section of handoff-F.md.
  1. **REWORK 1:** the decision-7 doc rule is on `PaneStore`, `HerdrPort`, `HostPort`, `HarnessPort` and `Prober` (the last worded for `ProbeResult`, where a timeout is `ProbeResult::Error`).
  2. **REWORK 2:** `Pane.env` and `ProfileSpec.env` read through `argv::deserialize_env_names`, which reads a JSON value first. A string with `=` is `profile-secret-refused`; any other wrong shape is `env-name-invalid`; no refusal echoes input. The field type is unchanged.
- **Decided (D1=a, answered by the coordinator under the operator's delegation, not by the operator directly):** `Watch<T>` items are `Result<Option<T>, PaneError>`; `Ok(None)` is idle, `Err(Timeout)` means only that the store did not answer. This **amends the brief's fixed `Watch<T>` alias** (decision 7; the brief's line 147 still spells the old alias and is superseded by this entry) and the first F pass's watch contract (entry 9, "an idle `next()` yields `Err(Timeout)`"), which is superseded. S's default for no answer was (b); (a) is the option A-dup recommended and the coordinator chose.
- **Decided (D2=a, same source):** `HarnessKind` stays a closed enum. Reason: the epic's non-goals exclude a new harness, and a stored record with an unknown harness fails closed. Its doc names `HARNESS_IDS`; T pins that every serde name is in that list. This is also S's default.
- **Decided:** A-dup rows 2 and 3 and T-green advisory 1 are done as doc lines and one doctest. `decode_params` keeps its signature (the doc says an absent `params` decodes from `{}`), because the signature is pinned by T's tests and a doc line costs no behavior change. Row 6 (the `Spec*` rename) is skipped on the coordinator's instruction.
- **Decided (this run's correction of the first edit):** the two `delete` docs now say a missing record is answered before the generation is compared (`pane-not-found` / `profile-not-found` "whatever `expected_generation` is"). Without it a missing record with a stale generation could be `generation-conflict` in one store and `pane-not-found` in another, which is the disagreement A-dup row 2 was about. #639's text fixes the codes, not the order.
- **Assumed:** the D1/D2 answer carries the operator's authority. The coordinator's reply says the operator delegated "drive this issue to completion"; its own transcript shows it told the operator at 04:32 MDT that it took the recommended options and that the operator can overrule them before merge. I could not confirm the delegation with the operator myself. Both choices are reversible until #637 merges.
- **Hedged:**
  - **F edited two test lines** (`tests/common/mod.rs:124`, `tests/ports_test.rs:83-85`) although the role forbids F editing tests. Reason: the coordinator's D1 instruction said to amend them and they are type annotations the new alias forces. No assertion changed. T should re-check them.
  - The new `compile_fail,E0423` doctest proves only that the line does not compile; stable rustdoc ignores the error code (S advisory 1).
  - The remaining test work (env echo, D1 idle behavior, D2 list test, delete rule, absent params, doubles through `next_generation`) is T's and not done.
  - **Non-array `env` shapes are `env-name-invalid`, not `usage`.** S allowed either and suggested this one; the cost is that the fixed `Display` ("an environment variable name must be non-empty and contain no whitespace") reads oddly for a map-shaped `env`. A `usage` refusal with its own fixed sentence would be clearer, and nothing else depends on the choice; T pins whichever the handoff states.
  - **How this pass ran.** The coordinator's D1/D2 reply reached the F agent while it was reading. The agent's transcript shows two interleaved executions from that point: one made the edits and ran the gates (its `StructuredOutput` tool was not available to it), the other re-verified the whole tree independently and made the three small corrections above (the `delete` check order in two docs, one misplaced comment in `deserialize_env_names`). The runs never wrote the same file at the same time; the edit tool's staleness check refused the second run's first edits, which is how it was noticed. Every gate in handoff-F.md was re-run by the second run on the final tree.
- **Evidence:**
  - Read: handoff-S.md, handoff-A-dup.md, handoff-T-green.md, handoff-F.md, the brief, decisions.md, evidence.md, and `argv.rs`, `error.rs`, `reply.rs`, `ports.rs`, `pane.rs`, `profile.rs`, `probe.rs`; the coordinator's reply to the D1/D2 message and the session transcript it was sent in.
  - Ran: the gates listed in handoff-F.md.
  - Scratchpad only (no worktree file touched): (1) a scratch crate (path dependency on `holler-pane`) that sent 19 `env` shapes, each as a `ProfileSpec` and as a `Pane`, through serde (value and text), `decode_params` and `PaneReply::failure`, checking the code, that no reply contains the secret, and the reply's parse-back; it also checked the new `Watch` item type and the two `decode_params` doc claims; (2) a detached copy of `holler-pane` with `RefusalCode`'s field made `pub`, where only the new doctest fails (the other two still pass), and a three-line program that shows the real error is E0423.

## T (Phase 7 re-entry, GREEN + Tier 2 after S's REWORK) - 2026-10-09, 05:14 MDT

- **Decided:** wrote `tests/rework_test.rs` (7 tests) for S REWORK 2, D1, D2 and A-dup rows 2 and 3, and moved both store doubles onto `next_generation` with a missing-record-first `delete` (A-dup rows 2, 4). Dropped a planned test of the `cas_put` double as redundant. Kept F's two test type-annotation edits (types only).
- **Assumed:** a test of a type alias (D1) can only be a contract pin through a consumer function; there is no production behavior to mutate.
- **Hedged:** the empty-iterator assertions in `ports_test.rs` were left as "stream ended", which stays true under the new item type.
- **Evidence:** 64 holler-pane tests + 3 doctests pass; workspace 928 passed, 0 failed; removing the `deserialize_with` on `Pane.env` fails the env test; lint, changelog, machete, clippy, golden, doc all clean. Verdict PASS, no production change needed.
