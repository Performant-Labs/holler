# Brief: #637 slice a, the `holler-pane` crate, the empty crates and the method names

Repo: Performant-Labs/holler. Issue: #637 (slice a of epic #633's skeleton; slices b and c are #669 hub plumbing and #670 CLI
skeleton). Rigor: in-session. UI surface: no. Kind: feature.

**Branch:** `issue-637-implementation`. **Design (D):** N/A (no UI). **Decision record:** the contract section of epic #633 is
fixed and ADR-0021 (#634) ratifies it later; this story builds against the epic text and does not wait for #634.

**Revision 5.** The single-story brief (revisions 1-4, in git at `182a69f`) drew three architecture BLOCKs and was split on
2026-10-09, with the operator's approval, into #637 (this), #669 and #670. This brief carries only slice a. Everything about
`holler-hub`, `holler-cli`, ADR 0003 and the CLI fixture is out of scope here. The issue text of #637 was amended the same day
and is the source of truth with the epic; where they differ from this brief, the issue wins.

## Problem

Thirteen wave-3 stories run in parallel. They can only do so if every shared type, trait, signature and error code is created
once, up front. This slice creates the **library** part: the `holler-pane` crate (types with serde, traits with fixed
signatures, the grid parser, the `Argv` and `EnvVarName` guards, the error taxonomy, the reply type, stubs), the four empty
crates, and the `pane/*` and `profile/*` method names. It creates no behaviour, no CLI and no hub code.

## Evidence (verbatim, as of `27da2da`)

The workspace takes every crate under `crates/`; a new crate carries `[lints] workspace = true`:
```
Cargo.toml:1-3
[workspace]
members = ["crates/*"]
resolver = "2"
```
```
Cargo.toml (workspace.lints)
unwrap_used = "deny"  expect_used = "deny"  panic = "deny"  unreachable = "deny"
cognitive_complexity = "deny"  too_many_lines = "deny"  struct_excessive_bools = "deny"
dead_code = "deny" (rustc)
clippy.toml: cognitive-complexity-threshold = 15 ; too-many-lines-threshold = 100
```
`cargo machete` fails CI on an unused dependency; a dependency with a feature list needs a `# for <consumer>` marker; file size
gate (`scripts/lint.sh` check 4): warn at 600, **fail at 900** lines. `dead_code = "deny"`, so every item in the new crate must be
`pub` (reachable from `lib.rs`) or used.

**The tree is not rustfmt-clean and CI has no fmt step.** On origin/main 176 of 195 `.rs` files fail `cargo fmt --check` (4004
hunks). So the formatting gate is: every **new** `.rs` file passes `rustfmt --check --edition 2021`; existing files are not
reformatted (repo practice, cb7fe39). `cargo fmt --check` on the whole workspace is **not** an acceptance criterion.

**The v2 wire catalog is closed**, 22 rows, and `find()` is the codec's `method_not_found` source:
```
crates/holler-proto/src/methods.rs:48-49,87-99
/// The complete, closed v2 method catalog (22 rows).
pub const CATALOG: &[Method] = &[ ... ];
pub fn find(name: &str) -> Option<&'static Method> { CATALOG.iter().find(|m| m.name == name) }
```
`holler_proto::vocab::SessionName` (`vocab.rs:70-98`, re-exported at `lib.rs:71-73`) is the ADR 0005 name grammar and has `parse`
but **no serde**; `holler-proto` declares itself async-free (`lib.rs:28-30`). Time is `holler_proto::clock::now_millis() -> i64`
(`clock.rs:35`). Timestamps elsewhere in the codebase are mixed (RFC 3339 in `holds.json` and `docs.rs`, u64 seconds in the token
store): no dominant pattern, so i64 milliseconds is a choice (decision 3).

## Acceptance criteria

Names are the test names T should author (RED first).

1. `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo machete`,
   `bash scripts/lint.sh` and `bash scripts/changelog-check.sh` pass; no file over 900 lines; `CHANGELOG.md` has an
   `## [Unreleased]` entry linking #637. Every new `.rs` file passes `rustfmt --check --edition 2021`; no existing file is
   reformatted.
2. `grid_parse_table`: `r2c1`, `c1r2`, `2,1` each give `{row:2,col:1}`; upper case and surrounding ASCII whitespace are accepted,
   interior whitespace is refused; a bare pair is never read as col,row; `r2`, `c1`, `r2c1c3`, `2,1,3`, `21`, `c1r2c3` and the empty
   string give `grid-ambiguous`; `r0c1`, `c0r1`, `0,1`, `r2c0` and values above `u16::MAX` give `grid-out-of-range`;
   `parse(format(x)) == x` for every cell of a 12 x 12 grid; serde writes `{"row":2,"col":1,"pos":"r2c1"}` in that key order and
   reads the same back (a `pos` that disagrees with `row`/`col` is refused).
3. `Argv` round-trips as a JSON array of strings; a bare JSON string where an `Argv` is expected is refused with `command-not-argv`
   (`argv_bare_string_refused`). `EnvVarName` refuses a string containing `=`, an empty string and whitespace (`env-name-invalid`);
   no `ProfileSpec` field can hold an environment value.
4. Serde round-trip tests: `Pane` (full), `Pane` without `profile` (loads as `None`), `Profile`, `ProfileSpec`. A `cas_put` with a
   stale `expected_generation` returns `PaneError::Conflict` whose `code()` is `generation-conflict` (against a trivial in-test
   `PaneStore`).
5. `error_codes_unique_and_kebab`: `ALL_CODES` are unique and each satisfies the one validator `error::is_valid_code`
   (`^[a-z]+(-[a-z]+)*$`); every variant's `code()` and `Display` are covered; `PaneError::refused(code, message)` rejects an
   invalid code; a `PaneReply` built from a `PaneError` round-trips through JSON and an unknown code parses to `Refused`.
6. `ports_compile_in_test_impl`: one test file implements every port (`PaneStore`, `ProfileStore`, `ProfileScope`, `HerdrPort`
   including `version()`, `HostPort`, `HarnessPort`, `Prober`; each `Send + Sync`), calls `ProfileScope::resolve` with and without
   a pane and `edit_spec` with and without a profile (trait-object `act`), and uses `run_probe`/`ProbeResult`; a later signature
   change breaks it. `pane_name_grammar` and `profile_name_slug` table tests (leading/trailing separators trimmed, a name with no
   alphanumerics refused when parsed, two names with the same slug have the same `slug()`).
7. `pane_methods_not_in_wire_catalog`: `methods::find("pane/list")` and `find("profile/list")` are `None`, `CATALOG.len()` is 22,
   and `PANE_METHODS`/`PROFILE_METHODS` list exactly the names in decision 8. No golden file changes (`scripts/golden-diff-summary.sh`
   shows no drift).
8. `git diff --name-only origin/main...HEAD` lists only paths in the Blast radius below (grep-able check for S).

## Files

Production (new unless noted):
- `crates/holler-pane/` (`Cargo.toml`, `src/lib.rs`, `error.rs`, `reply.rs`, `pane.rs`, `generation.rs`, `ports.rs`, `grid.rs`,
  `profile.rs`, `argv.rs`, `probe.rs`, and the empty declared stubs `profile_snapshot.rs`, `profile_diff.rs`, `tx_apply.rs`,
  `tx_launch.rs`, `tx_switch.rs`, `reconcile.rs`, `findings.rs`, `import.rs`); `lib.rs` declares every module so no later story edits
  it. Dependencies: `serde`, `serde_json` (consumed by `PaneReply.data`), `holler-proto` (path). No async runtime.
- `crates/holler-adapter-herdr/`, `holler-adapter-host/`, `holler-adapter-opencode/`, `holler-pane-testkit/`: `Cargo.toml` +
  `src/lib.rs` (a crate doc comment only). Each `Cargo.toml` has `[lints] workspace = true` and no dependency.
- `crates/holler-proto/src/methods.rs` (edit): `PANE_METHODS`, `PROFILE_METHODS` (name lists), `is_pane_method()`,
  `is_profile_method()`; outside `CATALOG`. `vocab.rs`: **no edit**.
- `CHANGELOG.md`: an `## [Unreleased]` entry linking #637. `Cargo.lock`: entries for the new crates only.

Tests: `crates/holler-pane/tests/*.rs` (default autotests there), and a `methods.rs` unit test in `holler-proto` (or
`crates/holler-proto/tests/`, whichever neighbours the existing catalog tests).

Reuse map (extend, do not duplicate): `holler_proto::vocab::SessionName` for `PaneName` (do not copy the grammar);
`holler_proto::clock::now_millis` for time; the `holler-proto` crate layout and error-type style as the pattern for
`holler-pane/src/error.rs`; `tempfile`/`rstest` only if already workspace dev-dependencies.

## Decisions already made (MO)

Accepted by the operator 2026-10-08 (decisions 0-8 of the original brief) and 2026-10-09 (the split, the freeze-less approach).
Numbers follow the original brief; those that belong to slices b and c are omitted here.

2. **Error codes live in `holler-pane/src/error.rs`**, not in `vocab.rs`. `PaneError` has one variant per closed code, a
   `code() -> &'static str` and `Display`; `pub const ALL_CODES` lists them; one validator `pub fn is_valid_code(&str) -> bool`
   (`^[a-z]+(-[a-z]+)*$`) is the only one in the workspace (#670's `output::ErrorCode::new` calls it).
   - domain: `not-implemented`, `usage`, `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`,
     `generation-conflict` (the CAS conflict, variant `Conflict`), `probe-failed`, `profile-conflict`, `profile-not-found`,
     `profile-exists`, `profile-has-live-panes`, `pane-not-in-profile`, `pane-in-other-profile`, `profile-secret-refused`,
     `herdr-version-unsupported`;
   - infrastructure: `timeout` (`Timeout { op }`), `pane-not-found` and `session-not-found` (`NotFound { what }`),
     `store-corrupt`, `unavailable` (`Unavailable { what }`: hub, Herdr socket or harness unreachable);
   - `profile-drift`: listed for convenience; it is a reconcile **finding kind** (#647/#665 put it in `findings.rs`), not an
     error a port returns.
   - **open variant** `Refused { code: Cow<'static, str>, message: String }` with `code(&self) -> &str`: an adapter crate or a
     verb returns a code it owns without editing the enum, and a reply code read off the wire that is not in `ALL_CODES` lands
     here. Built only through `PaneError::refused(code, message) -> Result<PaneError, ...>` which calls `is_valid_code`, so the
     validation is enforced, not conventional. Not in `ALL_CODES`.
   The owning story of each closed code is a doc comment on its variant (#644/#663: `probe-failed`, `profile-conflict`,
   `profile-not-found`; #643/#663: `pane-not-in-profile`; #661: `pane-in-other-profile`, `profile-secret-refused`,
   `profile-exists`; #662: `profile-has-live-panes`; #640: `herdr-version-unsupported`; #665: `profile-drift`; #638-#642:
   infrastructure variants). A later story fills behaviour, never the list.
3. **`holler-pane` depends on `serde`, `serde_json` and `holler-proto` (path); no async runtime.** `PaneName` is a newtype over
   `SessionName` (serde through `SessionName::parse`). `ProfileName` is a display name (spaces allowed, trimmed, non-empty, no
   control characters, at most 64 chars) with the one `ProfileName::slug() -> String` in `profile.rs` (lower-case, runs of
   non-alphanumerics become one `-`, leading and trailing `-` trimmed; a name with no alphanumerics is refused when the name is
   parsed, because the slug is persisted and unique-checked by #661). Every timestamp in the records (`last_observed.at`,
   `Parked.since`, `Profile.created`/`updated`, log entries) is **milliseconds since the Unix epoch as `i64`**. `GridPos` accepts
   upper and lower case and surrounding ASCII whitespace, refuses interior whitespace; zero is `grid-out-of-range`; a missing
   half, a repeated label or a mix such as `r2c1c3` is `grid-ambiguous`; bounds are `u16`.
7. **Ports (provisional until compiled):** the signatures below are the brief's best design, not a proof. The implementer makes them
   compile together with AC 6 and may adjust a signature to do so, recording each change in decisions.md; they freeze when #637
   merges, after which a change goes through the epic's amend-first rule. `HerdrPort` and `HarnessPort` stay provisional until
   spikes #636 and #635 report.
   - Every port is synchronous and blocking and `Send + Sync`; each trait carries the doc rule "call from `spawn_blocking` (or a
     thread) in async code; return within I5's bound (default 10 s) or with `PaneError::Timeout`".
   - `PaneStore { get(&self, &PaneName) -> Result<Option<Pane>, PaneError>; list(&self) -> Result<Vec<Pane>, PaneError>;
     cas_put(&self, &Pane, expected_generation: u64) -> Result<Pane, PaneError>; watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> }`
     with `Cursor(u64)` a store-wide change sequence number and `Watch<T> = Box<dyn Iterator<Item = Result<T, PaneError>> + Send>`.
   - `ProfileStore { get; list; cas_put(&Profile, expected_generation); delete(&ProfileName, expected_generation); watch(Cursor);
     log(&ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError>; rename(&ProfileName, &ProfileName, expected_generation)
     -> Result<Profile, PaneError> }` (`rename` is PROPOSED, with #665; the test impl returns `NotImplemented`).
   - `ProfileScope { resolve(&self, profile: &ProfileName, pane: Option<&PaneName>) -> Result<ResolvedScope, PaneError>;
     edit_spec(&self, profile: Option<&ProfileName>, &PaneName, edit: &SpecEdit, act: &mut dyn FnMut() -> Result<(), PaneError>) -> Result<Option<Profile>, PaneError> }`.
     `resolve` with no pane means every pane of the profile; a named pane outside the profile is `pane-not-in-profile`, a missing
     profile `profile-not-found`. `edit_spec` with `None` runs only `act` and touches no profile. `ResolvedScope { profile:
     Profile, panes: Vec<Pane> }` and `SpecEdit` (set or remove the pane's entry) are minimal types defined in `profile.rs`.
   - `HerdrPort { ensure_pane(&self, &HerdrSpec) -> Result<HerdrPane, _>; send_text(&self, &PaneId, &str); send_keys(&self, &PaneId, &[Key]);
     read(&self, &PaneId, max_lines: usize) -> Result<String, _>; close(&self, &PaneId); snapshot(&self) -> Result<HerdrSnapshot, _>; version(&self) -> Result<String, _> }`;
     `HostPort { ensure_session; run(&self, &PaneName, &Argv); stop_owned; ps }`; `HarnessPort { serve; health; create_session; list_sessions;
     abort; attach_tui; select_session; shown_session }`, each taking and returning the minimal data types defined in `ports.rs`.
   - `trait Prober: Send + Sync { fn run_probe(&self, &Argv, &[String], Duration) -> ProbeResult }`, a `SystemProber` calling the free
     `run_probe` stub (signature fixed by the epic; #663 builds the body), so #638's fake and AC 6 have something to swap.
   - `Ports { pane_store, profile_store, herdr, host, harness, scope, prober }` (one `&dyn` each) is defined here; `VerbCtx` is
     #670's.
8. **Methods and replies.** `PANE_METHODS = [pane/get, pane/list, pane/cas_put, pane/watch]`; `PROFILE_METHODS = [profile/get,
   profile/list, profile/cas_put, profile/delete, profile/watch, profile/log, profile/rename]` (`profile/rename` PROPOSED). They
   are hub control methods, outside `CATALOG`, so the v2 wire protocol is unchanged and a body connection still gets
   `method_not_found`. The params structs and `PaneReply { ok: bool, data: Option<serde_json::Value>, error: Option<{code,
   message}> }` live in `holler-pane/src/reply.rs`, shared by #639/#661 (server) and #649 (client); `holler-proto` holds names
   only because it cannot depend on `holler-pane`. No `schema_version` in `PaneReply` (it is not the CLI's envelope). Parse-back:
   a closed code maps to its variant (structured fields such as `Timeout { op }` and `NotFound { what }` travel in an optional
   `detail` string), anything else to `Refused`. `*/watch` are long-poll: one `{events, cursor}` reply per request.
9. **`Pane.hold` is doc-commented as a pane-record state, not a prompt hold**; #646's brief must put any prompt refusal derived from
   pane state at `send_prompt` (the stack's choke-point rule), not in the verb.

## Out of scope

Everything in `holler-hub` (#669) and `holler-cli` (#670), including `cli.rs`, `main.rs`, `output.rs`, the verbs, ADR 0003 and the
CLI fixture; any behaviour behind a stub; any adapter logic; registry persistence; the hub's real handlers (#639, #661);
ADR-0021 and `docs/protocol/v2.md` (#634).

## Test plan

RED first (T): the `holler-pane` tests cannot compile until the crate exists, which is the RED; confirm RED by `cargo test -p
holler-pane` failing to build or failing on named tests (no RED is a missing target). GREEN: F creates the crates and types until
the whole workspace is green. Then `cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt --check --edition 2021` on
each new `.rs` file (not the whole tree), `cargo machete`, `bash scripts/lint.sh`, `bash scripts/changelog-check.sh`,
`bash scripts/golden-diff-summary.sh` (no drift).

## Risks

- `dead_code = "deny"`: everything must be `pub` and reachable, or used; `cargo machete`: no unused dependency.
- Name collisions with `holler_proto` root re-exports (`Envelope`, `Target`, `List`, `Delete`): path-qualify, and do not re-export
  a name that exists in `holler-proto`.
- `serde` on `SessionName` is absent: `PaneName` implements `Serialize`/`Deserialize` by hand via `parse`/`as_str`.
- The ports reference types that spikes #635/#636 may change (`HerdrPort`, `HarnessPort`): keep their data types minimal.

## Blast radius

`Cargo.lock`; `crates/holler-pane/**`; `crates/holler-adapter-herdr/**`, `crates/holler-adapter-host/**`,
`crates/holler-adapter-opencode/**`, `crates/holler-pane-testkit/**` (skeletons only); `crates/holler-proto/src/methods.rs` and a test
beside the existing catalog tests; `CHANGELOG.md`; `docs/handoffs/637*` (pipeline artifacts). Not changed: workspace `Cargo.toml` (the
`crates/*` glob covers the new crates), `vocab.rs`, any golden file, `holler-hub`, `holler-cli`, `docs/`.
