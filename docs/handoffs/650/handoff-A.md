# Handoff-A: #650 - Import today's panes from the launcher's files  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-650-implementation` (base `origin/main` at `263a2d7`; the branch's two commits touch only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/650-brief.md` (as amended at `c25a467`; outside brief gate round 1 BLOCK, round 2 PASS)   **Reuse map:** the brief's own evidence, E1 to E30 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK: 2 `block`, 6 `warn`. The plan's shape is sound, but two rules would bake drift into tests that T writes from the brief. The brief's acceptance pins both, so T and F cannot fix them within it:

1. **Secrets.** A `source-unreadable` finding about `sessions.toml` or the launcher table can print the offending source line, a secret included. A TOML error's text quotes that line verbatim, and the plan's rule (untrusted text through `quoted` / `embedded`) escapes and cuts text but keeps its content.
2. **Vocabulary.** The plan reports a server that answers its health check unhealthy as `observe-failed`. That merged, stable code means a port call returned an error, and reconcile names an unhealthy server `server-down` or `server-wedged`.

I checked the plan's load-bearing claims in source, and these hold:

- `holler-cli` already depends on `holler-body` and `toml` (`crates/holler-cli/Cargo.toml`), so no manifest changes. `holler-pane` stays on `serde`, `serde_json` and `holler-proto`, as ADR-0021 section 5 requires.
- `crate::profile::create::insert_profile` is reachable from `pane::import` (`pub mod create` in the frozen `profile/mod.rs`; the function is `pub(crate)`). The real run reuses the one create write instead of copying its race mapping.
- The split between the crates is #662's split: the plan in `holler-pane` (ADR-0021 section 5 lists `import.rs`), the writes and undo in `holler-cli` (as `profile create` does).
- The hub accepts a new record whose profile exists. Its membership rule refuses only a move between profiles (`crates/holler-hub/src/panes/store.rs:338-357`), so the planned profile-first order is required and sufficient.
- `ImportFinding` in `holler-pane` can take `command-not-argv` from the closed table (`PaneCode::as_str` is `pub(crate)`) and `observe-failed` from `FindingKind::code` (a `const fn`), so no code is spelled twice.
- A separate enum is justified: `findings.rs` is #647's file and its kinds describe reconcile.
- The import's undo is justified in writing (decision 5) and is not a parallel path: it deletes where `create`'s detaches.
- The op-name constants (`host.ps`, ...) are private per module everywhere in the codebase (each adapter, each fake, reconcile), so the import's own copies follow the pattern.
- `doctor/rig.rs` is 409 lines, so the unrecorded seed fits.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | "The output": untrusted text only through `findings::quoted` / `findings::embedded`. Acceptance 7's `source-unreadable` for `sessions.toml`, a bad `pane.env` line, and (scope) the launcher table whose shape differs | cross-cutting: secrets | A TOML error's text quotes the offending source line. `ConfigError::Toml` keeps only that text. A `sessions.toml` spawn row's `env` table holds values, and the launcher table's unknown-key case is the one the plan expects on the real fleet's file. A malformed `pane.env` line has no safe "variable" to name. | Name the file and the position only, never an error's rendered text: `toml::de::Error::span()` and `message()` for the launcher table; `ConfigError::Invalid`'s row and field for `sessions.toml`, and only "not valid TOML" for `ConfigError::Toml`; the line number alone for a `pane.env` line that does not parse. Add an acceptance case with a secret on the offending line. |
| 2 | block | Join item 4, the record's `harness.health`, acceptance 6: a server that is "not healthy" gets `observe-failed` naming `harness.health` | pattern consistency: finding vocabulary | `observe-failed` means a port call returned an error, and its message names that error's code (`findings.rs:46-50`). Reconcile emits it only on `Err`. It names an answering, unhealthy server `server-down` or `server-wedged` (`reconcile/observe.rs:167-181`) and records `Unhealthy(<code>)`. The plan reuses a merged, stable code for another meaning (ADR-0021 `:512`), against its own rule to keep reconcile's spelling. | Classify an unhealthy server as reconcile does. One `list_sessions` call tells wedged from down and is never trusted. Report `FindingKind::ServerWedged/ServerDown.code()` and record `Health::Unhealthy(<code>)`. Keep `observe-failed` for a call that returned an error. Amend item 4, the "only when healthy" sentence, the record line and acceptance 6. |
| 3 | warn | Decision 9: `--herdr-session` is required "with the same meaning" as on launch | pattern consistency | Neither the join nor the record reads the flag, so it is required but does nothing. Launch uses it as the session part of the target it matches in the snapshot (`tx_launch.rs:456`). ADR-0021 `:439` says it exists because the frozen `HerdrPort` cannot tell the engine its session. | The join's candidates are the snapshot entries whose `session` equals the flag. A label that matches only in another session is `pane-unlabelled`, and ambiguity is counted within the session. Add one case with a same-labelled pane in a second fake session. |
| 4 | warn | Real run item 4: a failed undo answers `profile-conflict` "with the commands that reconcile (never `holler pane close`)"; acceptance 10 | error handling: the remedy | No verb removes a record without acting on the pane. `create`'s remedy (`profile delete --keep-panes`, `create.rs:249-260`) restores create's prior state because its records existed before it ran. For the import, the kept records stay detached, and a re-run skips them as `already-imported`, so they never join `fleet`. | Name the remedy in the brief so T does not have to invent it. After a profile-delete failure: `holler profile delete 'fleet'`, then run the import again. With kept records: `profile delete 'fleet' --keep-panes`, re-run the import, then `profile delete 'fleet' --keep-panes` and `profile create 'fleet' --from-current`. Reuse `single_quoted` and `JoinFailed::body`'s shape, and file a follow-up for a removal that touches only the record. |
| 5 | warn | "The output": `findings: [{"pane", "level", "code", "message"}]` | naming | Doctor's findings, the report the cutover runs next, call the same field `kind` (`findings.rs:214`, serialized as the code; its rig filters `f["kind"]`, `doctor/rig.rs:350`). Renaming a `data` key after merge is a breaking change (ADR-0021 section 9). | Call it `kind`, or record in the brief why it is `code`. |
| 6 | warn | Acceptance 12: ADR-0021 section 9's import row lists `profile-exists`, `import-blocked`, `usage`; sections 10 and 13 "say what this story decided" | ADR | The verb also answers `profile-conflict` (acceptance 10), `generation-conflict` (a `cas_put(record, 0)` that loses a race, its undo complete) and `herdr-version-unsupported`. `usage` is common to every verb. The open-codes row (`:584`) lists every merged open code. The launcher table is a new on-disk format, and section 13 names only what it maps to. | List those codes in the row, as `profile create`'s row does. Add `import-blocked` (#650) to the open-codes row. Write the table's shape in section 13 (`[<pane>]` with `port`, `cwd`, `command` as an argv; the port never derived from the name). |
| 7 | warn | "The output": the dry run prints the profile by rebuilding `profile show`'s spec lines, because `push_spec` is private | anti-duplication | It is a second copy of one text format (`show.rs:134-142`, with `text` at `:194-198`). The codebase shares a verb's helper by widening it to `pub(crate)` in its own file: `insert_profile`, `detach`, `single_quoted`, `pane_list`, `count`. `show.rs` is not frozen. | Widen `push_spec` and `text` to `pub(crate)` and call them, adding `profile/show.rs` to the blast radius as a visibility-only edit. If the copy stays, add a test that the import's spec block equals `profile show`'s for the same spec. |
| 8 | warn | "What the engine observes": per pane `host.ps`, `harness.health`, `harness.list_sessions`, in sequence | concurrency (I5) | With N wedged servers, a dry run takes about N times the 10 s bound. Reconcile, the other whole-fleet pass, runs a scoped thread per pane, so a pass takes about one chain's bound (`reconcile.rs:7-8`). ADR-0021 section 12 accepts the per-call bound, so this is consistency, not a violation. | Run each pane's observation on its own scoped thread as reconcile does, keeping name order when the plan is assembled. Otherwise, state in the brief that the import accepts N times the per-call bound. |

## Details

**1. TOML errors print the source line.**

- How the line gets in:
  - `toml` 0.8.23's `from_str` deserializes through `toml_edit` 0.22.27, which attaches the raw input to every error: a syntax error through `TomlError::new` (`parser/mod.rs:27`), a type or unknown-key error through `set_raw` (`de/mod.rs:187-260`).
  - `TomlError`'s `Display` then writes `{line_num} | {content}`, the whole offending line (`toml_edit-0.22.27/src/error.rs:101-110`).
  - `holler_body::config::parse` keeps only that text, as `ConfigError::Toml(e.to_string())` (`crates/holler-body/src/config.rs:273`). The type's own doc says it "already names the offending key/line" (`:141-144`).
- Why it can be a secret: the grammar allows `env = { KEY = "value" }` on a spawn row (`config.rs:224`), and the brief expects spawn rows ("`session_id` is optional only on a `spawn` row").
- The launcher table is parsed by new CLI code. With unknown keys denied, which the plan implies ("if its shape differs, the dry run reports `source-unreadable`"), an extra key's error quotes that key's line. A table exported from the real launcher could hold one with a value.
- The sanitizers do not stop this. `embedded` keeps 200 characters, enough for the line. `quoted` keeps 64, still the start of it.
- `embedded` is `pub(crate)` in `holler-pane`, so the CLI's `sources.rs` cannot reach it. The brief's rule presumes both are available wherever a message is built.
- The established pattern for a file that cannot be read is "naming the file and the problem but never quoting its content" (ADR-0021 `:298-300`, the stores). Section 9 adds "Every message is one line and never echoes a secret" (`:513`).
- `toml::de::Error` gives what is needed without the snippet: `message()` (`toml-0.8.23/src/de.rs:64`) and `span()` (`:70`).
- `pane.env`: for a line that does not parse as `[export ]NAME=value`, there is no safe "variable" to name. A bare pasted secret is a whole line with no `=`. Name the variable only when the line parsed and `EnvVarName::parse` accepted the name.
- Where the messages get built is F's choice. Either way, the CLI should hand on the file, line and field, never an error's text.

**2. `observe-failed` for an unhealthy server.**

- Reconcile's server chain (`crates/holler-pane/src/reconcile/observe.rs:151-184`):
  - `health` `Err` is `observe-failed` (`Server::Unknown`).
  - `Ok(true)` reads the list and trusts it.
  - `Ok(false)` calls `list_sessions` once, only to tell `server-wedged` (a `timeout`) from `server-down`, and trusts nothing it lists.
- Doctor records `Health::Unhealthy("server-down" | "server-wedged")` for the last case (`reconcile.rs:153-161`). A failed check leaves the stored health alone.
- The brief's own vocabulary rule is "a condition that reconcile already names keeps reconcile's spelling". Reconcile names this condition, and not `observe-failed`.
- `FindingKind::code`'s doc puts finding codes under section 9's stability ("A code is stable once merged"). Section 9 says a code is never "reused for another meaning once it has merged" (`ADR-0021.md:512`).
- The recommended fix keeps E17's trust rule intact: only a healthy server's list counts.
- The import's `observe-failed` messages should use reconcile's form, `<port>.<method> failed (<code>): <message>`. That form is built by `failed_message` (`reconcile.rs:438-444`), which is private to a frozen file. The import's copy of that one format string is therefore deliberate; the brief should say so, so the Phase 7 gate does not flag it.

**3. `--herdr-session`.**

- The real Herdr adapter fills every snapshot entry's `session` from its one configured session (`crates/holler-adapter-herdr/src/adapter.rs:262`). A wrong flag value would then make every pane `pane-unlabelled`, which fails closed.
- `FakeHerdr` can hold panes in several sessions, so the filter is testable.
- Without a filter, the flag's field is never read. It is a `pub` field of a `pub` struct in a library crate, so `dead_code` will not catch it.

**4. The undo's remedy.**

- What the undo leaves when it stops:
  - At step (c), the profile delete: every record is gone and `fleet` has no member. `holler profile delete 'fleet'` (no `--keep-panes` needed) followed by a re-run converges.
  - At step (a) or (b): `fleet` and some import records remain.
- Nothing in today's verb set can restore "nothing recorded": `pane close` is the only verb that deletes a record, and it stops and closes the live pane.
- The four-step path in the table ends with every record a member of `fleet`, provided no record belongs to another profile (`--from-current` refuses one).
- Settle the remedy in the brief now, since acceptance 10 pins that the message carries it.

## Notes for O

The two `block` findings change acceptance criteria, so they need the brief amended. Then start a **fresh** run, not `resumeFromRunId`, which would replay this verdict.

1. **Secrets (finding 1).**
   - Add to "The output", beside the `pane.env` rule: a `source-unreadable` message names the file and the position only, never an error's rendered text.
     - Launcher table: the line and column from `toml::de::Error::span()`, and `message()`.
     - `sessions.toml`: `ConfigError::Invalid`'s row and field. For `ConfigError::Toml`, only that the file is not valid attach-config TOML.
     - `pane.env`: for a line that does not parse as `[export ]NAME=value`, the line number only.
   - Add to acceptance 7: a secret-looking value on the offending line, both of `sessions.toml` (a spawn row's `env` table with a syntax error on that line) and of the launcher table (an unknown key), appears in no finding, no output and no envelope.
2. **Unhealthy server (finding 2).** Replace "the health check or the list failed or the server is not healthy ... `observe-failed`" with reconcile's classification:
   - `health` `Err`, or `list_sessions` `Err` on a healthy server: `observe-failed` in reconcile's message form, `harness.health` `unknown`.
   - `health` `Ok(false)`: one `list_sessions` call that only tells a `timeout` (`server-wedged`) from anything else (`server-down`), its list never trusted. The warning carries that `FindingKind` code, and `harness.health` is `{"unhealthy": "<code>"}` as doctor records it.
   - Either way the record has no `session_of_record`.
   - Update "What the engine observes" ("only when the health call answered healthy" becomes "trusted only when healthy"), the record's `harness.health` line, the new-code list (two codes reused from reconcile) and acceptance 6.
   - If you prefer to keep the call set, add one import-owned code for "answered unhealthy" instead. Its spelling must differ from reconcile's two, which claim more than the import observed.
3. **While amending, settle the warns that change acceptance:**
   - finding 3: the `--herdr-session` filter and its case;
   - finding 4: the remedy text acceptance 10 pins;
   - finding 5: `kind` or `code`;
   - finding 6: the section 9 row and section 13's table shape, which acceptance 12 pins.
4. **Left to F (no amendment needed):** findings 7 and 8.

## Patterns referenced

- `crates/holler-pane/src/reconcile/observe.rs`, `crates/holler-pane/src/reconcile.rs`: the trust rule, the server classification, `observe-failed` and its message, recorded health, a thread per pane.
- `crates/holler-cli/src/profile/create.rs` and `delete.rs`: checks first, `insert_profile`, the undo and its remedy, the shared `pub(crate)` helpers.
- `crates/holler-pane/src/findings.rs`: `FindingKind`'s single-source codes, the sanitizers, the `Finding` JSON shape.
- `crates/holler-body/src/config.rs`, plus `toml` 0.8.23 / `toml_edit` 0.22.27: what `load` refuses and what its errors carry.
- `docs/adr/ADR-0021.md` sections 5, 7, 8 (launch as built), 9, 12 and 13; `crates/holler-pane/src/tx_launch.rs` (`--herdr-session`, the record launch writes).
