# Decisions — #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

## A (Phase 3, up-front plan review) — 2026-10-09T10:30:57-06:00
- **Decided:** PASS on docs/handoffs/676-brief.md at 31fec64, with 0 blocks and 5 warns (see handoff-A.md). The plan extends `holler-pane/src/error.rs` (the one `PaneCode` table, `PaneCode::parse`, `is_valid_code`) and does not build a second code table in the CLI. `output.rs::exit_code` is the only exit-code decision point, and every pane/profile path reaches it. Dependency direction holds: no `lib.rs`, manifest, `holler-proto` or golden-file change. I accepted `ErrorClass::exit_code` in the types crate on the strength of decision 4 (#638's envelope helper pairs `ok: false` with 1, 2 or 3 and cannot depend on `holler-cli`) and the `is_valid_code` precedent.
- **Assumed:** #638 calls `class_of` (as the issue says) and needs the class-to-number mapping, not only `ok` versus exit 0. When #649 fills `Wiring::connect`, it may return a refusal-class code (ADR-0021 §10 reads the Herdr version on connect). This is the basis for W-2.
- **Hedged:**
  - W-1 is a warn, not a block. Under the open-code rule every open code exits 3. ADR-0021 I3's post-act mismatch ("exits 1 with a code") has no closed failure code, so #644/#645 must choose between `unavailable`, a closed-set amendment, and exit 3. The gap already existed in the frozen contract (`RefusalCode`/`Refused` are named as refusals), and this story only makes it explicit. The fix is one ADR sentence and one Risks bullet for the operator.
  - W-2 (the `main.rs:253-255` doc says "exit 1") is residue that AC 3's grep misses. It is accurate today, because `connect` cannot fail yet, so it is a warn.
  - W-5 (legacy "refusal" wording at exit 1) is left to #646, because the legacy CLI has no dominant pattern for the word.
- **Evidence:**
  - Files read: `error.rs` (all), `output.rs` (all), `holler-pane/src/lib.rs`, `main.rs:100-125`, `main.rs:220-290`, `pane/wiring.rs:1-80`, `prompt_target.rs:10-95`, `output_api.rs` (all), `error_test.rs` (all), `spec_flags.rs:1-12`, the doc lines in `pane/mod.rs`, `profile/mod.rs` and `args.rs`, ADR-0021 `:160-511`, ADR-0003 (all), the CHANGELOG `[Unreleased]` section, `docs_cli_test.rs` (matcher at `:121`) and `lint.sh:43-52`.
  - Issues read: #676, #633, #638, #644, #645 and #646 via `gh issue view`.
  - Greps: exit-code constants across `holler-cli` (no shared vocabulary), "refused or failed" and exit-1 wording across crates and docs, and `ErrorClass`/`class_of` collisions (none).
  - Sizes: `wc -l` on every touched file.

## T (Phase 4, author / RED) — 2026-10-09
- **Decided:** Authored `crates/holler-pane/tests/error_class_test.rs` (4 tests) and extended `output_api.rs` (3 new tests, 3 updated, header fixed). The exit-code numbers are pinned twice: through `class_of` over all `ALL_CODES` (catches a code added without a class) and through literal spot checks that do not call `class_of` (so a wrong table and a matching wrong test cannot both pass). I rustfmt-formatted `output_api.rs` (it was clean at HEAD).
- **Assumed:** A compile-time RED on the new `class_of`/`ErrorClass` API is unavoidable and accepted by the brief's test plan; the assertion-level RED is shown with a throwaway skeleton in `error.rs`, reverted.
- **Hedged:** `a_malformed_code_is_a_failure` passes against the all-Failure skeleton, so it is a regression guard, not a RED driver; it fails only if F routes malformed codes into the open-code (Refusal) arm. AC 3-5 (text residue, ADRs, CHANGELOG) have no new test; they are covered by the guard commands at Phase 6.
- **Evidence:** error E0432 from both targets as authored; skeleton run: holler-pane 3 failed / 1 passed, holler-cli `output_api` 6 failed / 15 passed (assertion output in handoff-T-red.md); `git status` shows only the two test files changed after the revert.

## F (Phase 6 of the script, implement) — 2026-10-09T10:47:54-06:00
- **Decided:**
  - `ErrorClass`, `ErrorClass::exit_code` (const, 2/3/1) and `class_of(&str)` go in `holler-pane/src/error.rs`. `class_of` does a `PaneCode::parse` let-else, then one exhaustive `match` over `PaneCode` with grouped arms and no `_`. An open well-formed code is a Refusal and a malformed code is a Failure.
  - `output.rs::exit_code` is now `class_of(code).exit_code()`.
  - Doc residue fixed in `output.rs`, `pane/mod.rs`, `profile/mod.rs` and `pane/args.rs`.
  - ADR-0021 §9 gets the 24-row table nested in the exit-codes bullet, plus item 5's "(done in #676)". ADR 0003 gets its line-97 sentence. The CHANGELOG gets its entry.
  - Applied A's doc warns that fall inside the blast radius:
    - W-1: the open-code sentence in §9, and the I3 code recorded under "Deferred to named stories" for #644 and #645.
    - W-3: ADR 0003's `Clarified by: #676` header line.
    - W-4: the `class_of` doc says it is not `PaneError::classify`.
  - The `not_implemented` doc in `output.rs` no longer calls a stub's error a refusal.
- **Assumed:**
  - The brief's literal "Not changed: `main.rs`" (decision 7, AC 7) outranks A's W-2, which asked O to widen the blast radius. O never amended the brief.
  - A `//!` doc line in a test file (`spec_flags.rs:3`) is T's to change, even though AC 3 lists it ("you stage no test files").
- **Hedged:**
  - `main.rs:253-255` still says "(exit 1, ...)" for a wiring error. It is accurate today because `Wiring::connect` cannot fail, and #649 must fix it. This is noted in handoff-F "Known issues" so the PR body or #649 carries it.
  - `spec_flags.rs:3-4` is flagged for T at T-green with suggested text.
  - I reported `archChanged: true` because the change adds public items to `holler_pane::error`. The addition was planned and A-reviewed at Phase 3.
- **Evidence:**
  - Tests: `cargo test -p holler-pane --test error_class_test` 4/4. `cargo test -p holler-cli --test pane_verbs` 64/64 (`output_api` 21/21). `cargo test --workspace` exit 0, with 1048 passed, 0 failed and 5 ignored.
  - Guards: `cargo build`, `cargo clippy --workspace --all-targets -D warnings`, `cargo machete`, `scripts/lint.sh` (exit 0), `changelog-check.sh`, `test-hooks.sh`, `docs_cli_test`, `cargo doc -D warnings` (holler-pane) and `rustfmt --check` on every changed `.rs` file all clean.
  - `grep -rn "refused or failed" crates/ docs/adr/` returns nothing.
  - The markdown-it GFM render shows a 25-row table inside the list item.
  - Source facts are in `evidence.md`.

## T (Phase 7, verify / GREEN) — 2026-10-09
- **Decided:** The suite is GREEN with no blocking issue. The one repair is the `//!` doc line at `spec_flags.rs:3` (no test code), which F flagged and AC 3 requires. No other test was wrong, so none changed.
- **Assumed:** Not re-running `test-hooks.sh` and `cargo doc` is safe: neither covers anything changed after F, and this phase edits only a test-file comment.
- **Hedged:** AC 4's ADR content (table wording, close-calls paragraph) is left to S's audit; T checked only that the docs tests and lints pass and that the table has the 22 closed-code rows.
- **Evidence:** `cargo test --workspace` exit 0 (1048 passed, 0 failed, 5 ignored). Mutating `output.rs::exit_code` to the old rule fails 6 `pane_verbs` tests; moving `PaneInOtherProfile` to the Failure arm fails `every_closed_code_has_the_decided_class`. Both mutations were reverted and `git status` shows only `spec_flags.rs` changed. clippy, machete, lint.sh, changelog-check, docs_cli_test and wire_selftest all clean.

## A (Phase 8 of the script, anti-duplication gate) — 2026-10-09T11:04:52-06:00
- **Decided:** PASS on the diff 55dba00..38eac7a, with 0 blocks and 4 warns (see handoff-A-dup.md).
  - F extended `holler-pane/src/error.rs`, the object the Reuse map named.
  - `class_of` has exactly one exhaustive `match` over `PaneCode`, with no `_` arm and 22 variants (1 usage, 14 refusal, 7 failure). Its lookup is `PaneCode::parse` plus `is_valid_code`.
  - `output.rs::exit_code` is one line and holds no table.
  - The tests reuse `error_body`, `with_sink` and `one_envelope` and iterate `ALL_CODES`.
  - There is no new module, no re-export and no manifest or `main.rs` change, and `error.rs` loses no line.
- **Assumed:** Two things stand as Phase 3 accepted them: brief decision 4 (the exit numbers live on `ErrorClass` in `holler-pane`) and the separate test file. AC 4 requires the ADR table, so the table itself is not drift. Only the fact that no test checks it is a warn.
- **Hedged:**
  - W-1 (no conformance test pins the ADR-0021 §9 table, and the ADR's instruction for moving a close call at `:397` omits the table) is a warn, not a block. The repo's docs-conformance pattern (`docs_errors_test.rs`, the hold/grant exit checks, `docs_rows.rs`) does not cover every ADR table.
  - W-2 (four `output_api.rs` routing tests assert close-call codes as literal 3s, so "one arm and one test row" understates the cost of a move) is a warn. AC 2 mandated three of the four.
  - W-3 (`main.rs:253-255`, for #649) and W-4 (`prompt_target.rs:58-61`, for #646) are carried from Phase 3 and need no change in #676.
- **Evidence:**
  - Files read: `error.rs` (all), `output.rs` (all), `output_api.rs` (all), `error_class_test.rs` (all), the `spec_flags.rs` diff, the ADR-0021, ADR-0003 and CHANGELOG diffs, `main.rs:245-272`, `pane/wiring.rs:31-32`, `prompt_target.rs:50-95`, `holler-pane-testkit/src/lib.rs`, `docs_errors_test.rs:1-59`, `docs_rows.rs:1-50`, and `error_test.rs:19` and `:80-86`.
  - Greps: definitions and uses of `ErrorClass`/`class_of` (one definition), exit-code constants across crates, classifier-like enums, "refused or failed" repo-wide (none), and exit wording in ADR-0021 and ADR-0003 (consistent with the table).
  - Diff checks: `git diff --quiet` on `lib.rs`, `main.rs`, every manifest and `Cargo.lock` (unchanged); 0 removed lines in `error.rs`; `wc -l` on the touched files (all ≤ 710); the newer `origin/main` commits overlap nothing in this branch.
