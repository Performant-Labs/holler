# [feat(cli): holler profile rename, export and import; doctor code profile-drift](https://github.com/Performant-Labs/holler/issues/665)

Part of epic #633. This is the complete statement of story #665, written as one text against `main` at `bda1069`. The issue was filed 2026-10-08 and 2026-10-09 as PROPOSED, before most of the epic merged (E1); **the operator confirmed it on 2026-10-11 at 19:40 MDT** ("confirmed, build it", E2). This brief supersedes the issue's text where they differ (the list is "Where this brief is the decision"). It is **off the critical path**: nothing waits for it.

What the epic built since the issue was filed changes the story in five ways, each checked against the code (the evidence is at the end, as E1 to E77):

- The stubs the issue says #637 and #670 pre-created exist (E3, E41, E70). The `profile/rename` method, `ProfileRenameParams`, the CLI client's `rename`, and the `ProfileChange::Renamed` variant are all merged; only the hub-side transaction, the three verbs and the finding are missing.
- The four codes the issue names (`profile-secret-refused`, `profile-exists`, `command-not-argv`, `profile-not-found`) are all in the closed list, and so is `profile-drift` (E21). **No new error code is needed**, and no open code either.
- The hub's two registries are separate stores with separate files and locks; ADR-0021 already fixes the lock order for this one operation (E63) and says no transaction spans them (E64). That sentence is amended first (below).
- `profile-drift` is an error code but not yet a `FindingKind`; reconcile's module doc already defines "once per episode" as once per pass (E30). The issue's "raised once per episode" is read that way (D10).
- The operator guide `docs/pane-control.md` (#652) and its docs test mark the three verbs and the finding PROPOSED, by a line rule (E48, E49, E51, E53, E55, E56). Building the verbs makes that rule false, so the guide and the test change in the same story (D11).

## Where this brief is the decision

Decisions D2 to D5, D7 to D10, D12 and D13 are **decided by the preparer, to be confirmed at the plan check**. D1, D6 and D11 follow from rules the repository already states. Each lists what was rejected.

**D1. The contract is amended first, in the first commit, before any test (the epic's amend-first rule).** ADR-0021 and ADR-0003 change as "The contract amendment" below says, exact text included. The first commit of the build is that amendment and nothing else; a test that quotes the ADR is written against it.

**D2. Rename is one hub-side transaction under both locks, panes saved first (decided by the preparer, to be confirmed at the plan check).**
- *Lock order* is ADR-0021's: the profile lock, then the pane lock, never the reverse; the pane registry never calls the profile registry while holding its own lock (E63). The membership hook of a pane write (`check_membership`) takes the profile lock and releases it before the pane lock, so there is no nesting to deadlock on.
- *Checks* run in this order and write nothing: a registry that failed to load (either one) is `store-corrupt`; OLD not found by slug is `profile-not-found`; NEW equal to OLD's stored name is `usage`; NEW's slug filed under a different live profile is `profile-exists`, whatever the generation; a stale or ahead generation is `generation-conflict` (the existing `check_name` then `next_generation` order of `cas_put`, E5, E6).
- *The change.* The record takes NEW's name and slug and the generation + 1 (`holler_pane::next_generation`), keeps `created` and its specs, and takes the one stamp (`entry::stamp`, E8) as `updated`. Every pane record that is a member of OLD (`holler_pane::profile_diff::is_member`, by slug, E32, not by string equality) and does not already name NEW takes `Pane.profile = NEW` and generation + 1; **nothing else of the record changes**, so no pane is detached, relaunched, probed or touched live. The write does not go through `Store::cas_put`, so `refuse_profile_move` (E10) is not involved.
- *Saves.* Both documents are built under both locks. **The pane document is saved first, the profile document second.** Memory, rings, heads and wake-ups of both registries change only after both saves succeeded. If the second save fails, the first document is saved back (the previous bytes), the call answers `unavailable` and nothing changed; if that restore fails too, the hub logs one `error` event, `profile_rename_restore_failed`, and the next write to either registry rewrites its file from memory (the pane store writes its whole table on every save, E9).
- *Why panes first.* A crash or a failed restore between the two saves leaves the member panes naming NEW while the profile is still OLD at its generation. Running the **same** `holler profile rename OLD NEW` again then completes it (no member is left to move, OLD is found, NEW is free). Profile-first would leave panes naming a profile that no longer exists, and the rerun would be `profile-not-found` with no verb that repairs it.
- *Residual, documented, not fixed.* A pane join that names OLD, checked before the rename and written after it, can store a pane naming a profile that is now a tombstone: the same check-then-write gap `profile delete` has today. The hub files have no fsync (ADR-0021 section 7), so a crash can also lose either file's last write.
- *Rejected.* (a) The CLI moving the panes with one `cas_put` each, as `profile delete --keep-panes` does (E27): N + 1 writes, every one a place to stop half-way, and the issue says the transaction is hub-side. (b) One combined file for both registries: a persistence-layout change that needs a migration. (c) Profile first: see above. (d) A "rename intent" record that a restart replays: new durable state, for a window the retry already closes. (e) Holding only the profile lock and writing the panes through `pane/cas_put`: a pane writer can slip in between two member writes, so a reader can see a half-moved profile.
- *Shape, not names.* `PaneState` gains one crate-private method that does the pane half under the pane lock and takes a closure for "now save the profile document" (the `Store` type is private to `panes`, E11), and `profile/store.rs` gains the matching `Store::rename`. The build chooses the names. `profile/rename.rs` keeps `dispatch`, which decodes `ProfileRenameParams` (E19) through `panes::handlers::run`, the pipeline every other `profile/*` method uses (E77), and calls the transaction with both states (E4, E13).

**D3. `profile/rename` is the full transaction; the in-process port method is the record half (decided by the preparer, to be confirmed at the plan check).** `ProfileState::rename` (the `ProfileStore` port, E4) cannot reach the pane registry, and the fakes cannot either. So the port method renames the **record** (name, slug, generation, one `renamed` entry, the events), and the doc of `ProfileStore::rename` in `holler-pane/src/profile.rs` (E18) says: "a registry that holds no pane registry has no members; the hub's `profile/rename` also moves every member pane". The hub's `profile/rename` handler calls the full transaction, and the CLI client reaches only that (E40). The conformance suite pins the record half, on the fake and on `ProfileState`; the members-follow guarantee is pinned by the hub integration test through the real handler. *Rejected:* (a) `ProfileState` holding a handle on the pane store (a cyclic wiring: `PaneState::dispatch` already takes the profile state, and every `ProfileState::load` caller in the tests would change); (b) leaving the port `not-implemented` on the hub (half of the port stays dead and the suite cannot test rename on the real registry); (c) coupling `FakeProfileStore` to `FakePaneStore` (the two fakes are independent by design, E72).

**D4. The log (decided by the preparer, to be confirmed at the plan check; see question 3).** A rename is, as the feed already says, a deletion of the old name followed by a put of the new one (E17). Applied to the logs, which are filed by slug (E7): OLD's slug entry becomes a tombstone whose log gets **one** `deleted` entry (so the load check "a tombstone's log ends with its deletion" holds without a change, E7), and NEW's slug entry gets **exactly one** `renamed { from: <OLD's stored display name> }` entry at the new generation, with the request's actor and the one stamp. No entry is copied, moved or rewritten, so each existing log is a prefix of itself afterwards ("the log intact"). If a deleted profile of NEW's slug left a log, it goes on, as it does across a re-create (E5). A rename that only changes the spelling (same slug) is one entry on the same slug entry and one put event. The reading of the issue's "one change-log entry" is: the renamed profile's log gains one. *Rejected:* (a) copying or moving OLD's log under NEW (duplicates history, and breaks the non-decreasing `at` of a log whose slug had history); (b) a new `ProfileChange` variant (the type is frozen and on the wire); (c) relaxing `entry::check` so a tombstone may end otherwise (weakens a fail-closed load check).

**D5. The events and the order (decided by the preparer).** The profile feed gets the tombstone of OLD, then the put of NEW, at consecutive cursors; a same-slug rename gets one put. The pane feed gets one put per moved member, in pane-name order, at consecutive cursors. The reply `data` is the renamed record (ADR-0021 section 6 already says so, E63).

**D6. No new port method, no wrapper change (the epic's rule for this story).** The only reads are `ProfileStore::list` and the existing `Ports` fields (E33); `ProfileStore::rename` already exists and its CLI client already sends `profile/rename` (E40). The wrappers of `wiring/bind.rs` (`BindingHerdr`, `LazyHerdr`, `BindingHost`, `Wired`, E39) wrap the Herdr, host and harness ports, not the stores, so there is nothing to forward and no per-method forwarding test to write. If the build finds that it needs a method on any port, it stops and returns to the plan check: that is a contract amendment with its wrapper test, not a detail.

**D7. The file is JSON, one document, with the guards doing the validation (decided by the preparer).** The shape, pretty-printed with two spaces and a trailing newline, keys in this order:

```json
{
  "format": "holler-profile",
  "version": 1,
  "name": "Fleet",
  "panes": [
    {
      "pane": "demo-c1r1",
      "herdr": {"workspace": "main", "grid": "r1c1"},
      "host": {"cwd": "/srv/demo"},
      "harness": {"kind": "opencode", "port_policy": "fixed:8095"},
      "model": {"provider": "demo-provider", "model_id": "demo-model", "effort": "medium"},
      "role": "agent",
      "env": ["DEMO_API_KEY"],
      "context": {"soft": 100000, "hard": 150000},
      "command": ["opencode", "serve"],
      "check": ["curl", "-s", "http://127.0.0.1:8095/v1/models"],
      "expect": ["demo-model"]
    }
  ]
}
```

(`opencode_agent`, `command` and `check` are left out when absent, as `ProfileSpec`'s serde leaves them out; `env` and `expect` are always present.) Each pane object is the spec's own serde value with **one** change: `herdr.grid` is the string `rRcC`, not the record's `{row, col, pos}` object. The file has no `slug`, `generation`, `created`, `updated` or log (specs only). It carries no value of any environment variable, by construction: a spec holds `EnvVarName`s, and the verb reads nothing from the process environment. *Rejected:* (a) TOML: the guards that raise `profile-secret-refused`, `command-not-argv` and the grid codes report through `serde_json::Error` and `holler_pane::reply::decode_params` (E20, E23); a TOML path would need a second mapping from its errors to the same codes; (b) the record's own serde form for the grid (the issue says "export writes `rRcC`", and `GridPos`'s deserializer reads only the object, E25); (c) embedding the log or the generation (the issue says no).

**D8. Import reads, validates, and only then touches the store (decided by the preparer).** In this order, each failure writing nothing: (1) FILE must be a readable regular file of at most 1 MiB, else `usage`; (2) the text must be one JSON object with exactly the members `format`, `version`, `name`, `panes`, `format` equal to `holler-profile` and `version` equal to 1, else `usage` (an unknown member, a wrong marker and a newer version are `usage`; the message never quotes the file's content); (3) NAME is `--name` when given, else the file's `name`, through `ProfileName::parse` (`usage`); (4) each pane's `herdr.grid`, when it is a string, goes through `GridPos::parse` (`grid-ambiguous` or `grid-out-of-range`; `r2c1`, `c1r2`, `2,1` and `R2C1` all load as row 2, column 1; an object is read by `GridPos`'s own serde); (5) the specs decode through `holler_pane::reply::decode_params` into `ProfileSpec`s, so the typed guards answer: an env entry with a value (`NAME=value`, or a bare string with `=`) is `profile-secret-refused`, a command or check that is a string is `command-not-argv`, a blank env name is `env-name-invalid`, and an unknown member is `usage` (**no second scan** of the raw text for secrets, as the issue's amendment says); (6) the name is free: `ProfileStore::get` by name (which finds by slug), then `insert_profile` (E26) for the race, both `profile-exists`. The new profile is **detached**: no pane is read or written, and a spec may name a pane that does not exist or belongs to another profile (E17's rule for specs). It is stored at generation 1 with the actor `holler profile import`. *Rejected:* standard input as `-`; a `--force` or merge into an existing profile; a scan for `=` in the raw text; joining the named panes.

**D9. Export (decided by the preparer).** `holler profile export NAME [--out FILE]` reads the profile (`profile-not-found`) and builds the document. With `--out` it writes FILE by `holler_proto::atomic_file::write_atomic` at mode 0600 (a temporary file in FILE's directory, then a rename), **replacing** an existing file; a path that cannot be written is `usage` (exit 2) with the I/O error kind, never the content. Without `--out` the document is the result: in text mode it is the whole stdout, and in JSON mode it is `data.document`. Export writes nothing to the hub. *Rejected:* refusing to overwrite (it needs a new open code or a misleading `usage`, and the operator can remove a file), a `--force` flag the issue does not list, mode 0644.

**D10. `profile-drift`, and what "once per episode" means (decided by the preparer, to be confirmed at the plan check; see question 4).**
- *What it is.* A pane **record** that differs from its profile's spec on at least one field of `profile_diff`'s table (`SpecField::ALL`, 16 fields). "Live" is what `profile show` calls live (E34): the registry record, the way the pure comparison of E32 reads it; the pass calls no extra port.
- *How it is found.* `diff_profile(profile, members)` (E32), literally, where the members are the records for which `is_member(record, profile)` holds, so `profile show` and the finding agree by construction on one fixture. A pane in scope whose row is `Differs` gets one finding. `Missing` (a spec with no pane) and `Extra` (a member with no spec) are **not** drift: they are `profile apply`'s business and would make every freshly created profile report until it is applied. A profile whose specs name one pane twice yields that pane's first differing row only.
- *Episode.* An episode of drift for a pane is a span of passes in which its record differs from its spec. The engine keeps no memory between passes and says so (E30; the guide says "a pass keeps no memory between runs, so every tick reports every standing finding again", E52). So the rule is: **one `profile-drift` finding per drifted pane per pass**, carrying every differing field, and the same finding again in every later pass for as long as the difference stands; a repaired pane yields none; a pane that drifts again begins a new episode. This is "once per episode" in the engine's own sense (once per pass), and it is the same rule every other kind follows. A second finding is never raised for a second field of the same pane.
- *Rejected.* (a) Reporting a drift once and then muting it: that needs a stored "reported" marker, which is a new field on a frozen record, and a write by doctor, and doctor's read-only contract (E36's pins) forbids writes other than the observation record; (b) a table in the hub of what was reported (the hub is the store only, ADR-0021 section 7); (c) in-process memory in a long-lived `pane watch` loop (no such loop exists, and the guide says the periodic pass is the operator's scheduler); (d) one finding per field.
- *Shape.* `FindingKind::ProfileDrift`, code `profile-drift`, **appended last** to `FindingKind::ALL` as it stands on the base the build starts from (E29; other stories append kinds too, E73), no remedy (`None`: which side is right, the pane or the spec, is the operator's choice, and the module's rule is that no remedy carries a profile name, E28), `FixState::NotFixable`. `Finding::about(kind, pane, message)` supplies the pane and its position. The message is `differs from profile <quoted name> on <field>, <field> (see holler profile show)`: the profile name goes through `findings::quoted`, the fields are `SpecField::as_str` paths in table order, **no value** is quoted (a value could be a path or a command). The comparison lives in a new file `crates/holler-pane/src/reconcile/drift.rs`; `reconcile.rs` gets one `mod` line and one call after the stray-session step (E31).
- *Reads.* One `ports.profile_store.list()` per pass, made **only when a pane in scope names a profile**, so a pass over panes with no profile makes no profile-store call, as today (E36). A failed `list` is one `observe-failed` finding naming `profile_store.list` and the error's code (the existing `failed_message`, E31), and no drift is concluded for that pass; the rest of the pass is unaffected and the exit code stays 0.
- *Scope.* Whole fleet: every profile's members. `--profile P`: the panes the scope returns. `--pane X`: that pane only.
- *Consequence for the test rig.* The doctor rig builds its profiles with `sample_profile` (E37), whose specs say port policy `fixed`, grid `r1c1` (E38), while a record's snapshot says `fixed:<port>` and the seeded cell: every profiled seed would drift. The rig's `profiles_of` therefore builds each profile from the stored records (`profile_from_panes`), so "a freshly built rig is a healthy fleet" stays true, and the existing doctor tests pass unchanged.

**D11. The guide and its docs test change together, and `import` is two verbs (follows from #652's brief and E48).**
- *The `import` collision.* Two verbs are named `import`: `holler pane import` (#650) and `holler profile import` (#665). Every test already keys on the pair (namespace, verb): the clap walk and heading forms (E49), `STORY_GROUPS` (E46), `STUBS` (E47), the ADR rows (`holler pane import` and `holler profile import` are different row prefixes) and the fixture leaf set. The old **line rule** (every line containing `profile import` must say PROPOSED) never matched `pane import`, because the substring `profile import` is absent from it; it is deleted with the other two, and the new test asserts both `import` headings exist and that no line naming `profile import`, `profile rename`, `profile export` or `profile-drift` says PROPOSED.
- *The exact edits are listed in "The guide and its test: the exact edits".*

**D12. Rigor is `second-opinion` (the issue says `in-session`; the preparer raises it, see question 1).** The story adds a cross-registry state transition on the hub (two files, two locks, a recovery rule), a new reconcile finding kind and a new on-disk file format, and the pipeline's default for non-trivial work is `second-opinion`.

**D13. Who writes what, as whom.** The actors are the constants `holler profile rename` and `holler profile import` (both under the 64-character limit); export writes nothing to the hub and has none. The three verbs print through `output::emit` (E43, E44): text success on stdout, text errors as `error: <message>` on stderr, JSON as one envelope; the exit code is the error class (`usage` 2, refusals 3, failures 1).

## Scope

One story, three parts that can be built and tested in this order (A is the longest, C touches the shared files and so goes last; none depends on another, so the plan check may split them, see question 2):

**A. `holler profile rename OLD NEW`** (positionals with the clap ids `old` and `new`, so the guide's heading form is `holler profile rename OLD NEW`, E49). The CLI reads OLD (`ProfileStore::get`, `profile-not-found`), refuses NEW equal to OLD's stored name (`usage`), and calls `ProfileStore::rename(old, new, generation read, actor)`; everything else is the hub's (D2 to D5). Output, text: `renamed profile <OLD> (<old slug>) to <NEW> (<new slug>), generation <G>`; JSON `data`: `{"profile": <the renamed record>, "from": {"name", "slug"}}`.

**B. `holler profile export NAME [--out FILE]` and `holler profile import FILE [--name NAME]`** (ids `name` and `file`, so the heading forms are `holler profile export NAME` and `holler profile import FILE`). Export text with `--out`: `exported profile <NAME> (<slug>) to <FILE>: <N> specs`; JSON `data`: `{"name", "slug", "specs", "out", "document"}` where `out` is the path or `null` and `document` is the document object when `out` is `null`, else `null`. Import text: `imported profile <NAME> (<slug>): <N> specs, generation 1`; JSON `data`: `{"profile": <the stored record>}`. A path is printed through the escaping of `profile::show::text` so a control character never reaches a terminal. The shared document type and the (de)serialization live in `export.rs` and are reached from `import.rs` as `super::export::...`, the way `create`, `delete` and `show` share helpers (the frozen `profile/mod.rs` admits no new module, E42).

**C. The doctor finding `profile-drift`** (D10), and the guide, tests, ADR rows, fixture, CHANGELOG entry that change with the verbs and the finding.

Out of scope: a `profile log` verb (the log is readable only through the port today); renaming a **pane**; any change to `profile apply` (it still only reports a healthy pane that differs); a profile that a pane names but that does not exist (a different inconsistency; this story does not report it; it is noted as a follow-up in "Not verified"); `--adopt`-style repair of drift; repairing what a crash between the two saves left by anything but rerunning the same rename.

## The contract amendment (first commit)

ADR-0021 and ADR-0003 change before any test. Every edit below is exact; where the text of another story's edit sits next to it, the clash table says what to do. ADR rows are grouped by owning story and the three #665 rows stay adjacent to each other and separated by a blank line from other stories' rows (`adr_0003_rows_of_different_stories_are_never_adjacent`, E46).

**ADR-0003** (E57, E58):
1. Lines 72 to 74 (the three rows) become, in this order and with the same column for `#665`:
   `holler profile rename OLD NEW` / `holler profile export NAME [--out FILE]` / `holler profile import FILE [--name NAME]`, each followed by `#665` (and no "proposed" clause). `docs_cli_test` parses these rows against the clap tree, which is what keeps them true.
2. In the line that begins `- **\`holler pane\` and \`holler profile\`** are pane control`, delete the sentence "The rows of `profile rename`, `export` and `import` are **proposed** (#665): the operator confirms them first." Nothing else in the line changes.
3. No header line is added (the `Amended by` of #670 covers the namespaces, and the header is a hot spot, see the clash table).

**ADR-0021** (E59 to E67):
1. Line 3 (status): `**Status:** accepted`. Nothing remains PROPOSED once this story merges; the build greps `PROPOSED` and `proposed` in the file and fixes any other line that is now false.
2. Section 2 table, the `ProfileStore` row: replace "and **PROPOSED** `rename(from, to, expected_generation, actor)`" by "and `rename(from, to, expected_generation, actor)` (#665; on the hub the record half, the members follow in `profile/rename`, section 8)", and the last cell "the hub (#661); `rename` answers `not-implemented` until #665 is confirmed" by "the hub (#661; `rename`, #665)".
3. Section 3, line 246: "plus **PROPOSED** `rename`, `export` and `import` (#665)" becomes "plus `rename`, `export` and `import` (#665)". After the paragraph that ends "A profile's **live panes** are the pane records whose `profile` names it (by slug)." add:

   > **Profile files as built (#665).** `holler profile export NAME [--out FILE]` writes a profile as one JSON document, `{"format": "holler-profile", "version": 1, "name", "panes": [...]}`, whose panes are the specs with `herdr.grid` as `rRcC`; it holds no slug, generation, timestamp or log, and no environment value (a spec holds names). `holler profile import FILE [--name NAME]` reads the same document, accepts a position in any form `GridPos::parse` accepts, and creates a **detached** profile (no pane joins) at generation 1: a file with an environment value is `profile-secret-refused` (the typed `EnvVarName` decode refuses it; no second scan), a command or check that is a string is `command-not-argv`, a taken name or slug is `profile-exists`, and a file that is not a document of this format is `usage`. `profile-drift` (#665) is a reconcile finding, raised for a pane whose record differs from its profile's spec on any field of `profile_diff`'s table: the comparison `profile show` makes, so they agree; a spec with no pane and a member with no spec are not drift. One finding per drifted pane per pass naming every differing field and no value, with no remedy; a pass keeps no memory (section 8), so "once per episode" is once per pass. A pass reads `ProfileStore::list` once, and only when a pane in scope names a profile; a failed read is an `observe-failed` finding.

4. Section 6, line 401: "and **PROPOSED** `profile/rename`." becomes "and `profile/rename` (#665)." The `data` table row "`profile/rename` (PROPOSED) | the renamed record" becomes "`profile/rename` | the renamed record".
5. Section 7, line 450: "(only the PROPOSED `profile/rename`)" becomes "(only `profile/rename`, #665)".
6. Section 8, line 483: "No store transaction spans the two registries." becomes "No store transaction spans the two registries except `profile/rename` (#665, below)." and, as the paragraph after the existing "Apply as built (#664)" paragraph, add:

   > **Rename as built (#665).** `holler profile rename OLD NEW` sends `profile/rename` with the generation of OLD it read, and the hub does the whole change as one transaction under the profile lock and then the pane lock (section 7). The checks come first and write nothing, in this order: either registry failed to load (`store-corrupt`); OLD not found by slug (`profile-not-found`); NEW equal to OLD's stored name (`usage`); NEW's slug filed under another live profile (`profile-exists`, whatever the generation); a stale or ahead generation (`generation-conflict`). The change: the profile record takes NEW's name and slug and generation + 1 and keeps its `created` and specs; every pane record that is a member of OLD (by slug) and does not already name NEW takes `Pane.profile` = NEW and generation + 1, and nothing else of it changes, so no pane is detached, relaunched or touched live. The logs are filed by slug: OLD's gets one `deleted` entry and NEW's exactly one `renamed {from}` entry at the new generation, with the request's actor; no entry is copied or rewritten, and a deleted profile's older log under NEW's slug goes on. The profile feed gets the deletion of OLD then the put of NEW (one put when only the spelling changed), the pane feed one put per member. Both documents are saved, the panes' first; both registries' memory, rings and cursors change only after both saves succeeded, and a failed second save saves the first document back (`unavailable`, nothing changed). A crash between the saves leaves the members naming NEW while OLD stands at its generation, and running the same rename again completes it. A pane join that names OLD, checked before the rename and written after it, is the same check-then-write gap `profile delete` has. The reply is the renamed record. The in-process `ProfileStore::rename` of a registry with no pane registry (the fakes, `ProfileState` on its own) renames the record only.

7. Section 9, the failure-mode row "| PROPOSED `profile rename`, `export`, `import` | ... |" becomes three rows:
   - "| `profile rename` | `profile-not-found`, `profile-exists`, `generation-conflict`, `store-corrupt`, and `usage` for a new name equal to the old |"
   - "| `profile export` | `profile-not-found`, and `usage` for a file that cannot be written |"
   - "| `profile import` | `profile-exists`, `profile-secret-refused`, `command-not-argv`, `env-name-invalid`, `grid-ambiguous`, `grid-out-of-range`, and `usage` for a file that cannot be read or is not a document of this format |"
   The class-table row for `profile-drift` (line 813) keeps its class and its sentence, and gains "Raised by reconcile since #665." at the end of the Reason cell. Section 9's "Close calls" sentence, which says `profile-drift` is "never an error today", stays true.
8. "Decisions taken": item 4 stays as the record of 2026-10-09; add the **next free number** at merge (other stories append items; renumber at the rebase): "**`profile rename`, `export`, `import`, `profile/rename`, `ProfileStore::rename` and the `profile-drift` finding (#665) are confirmed and built** (operator, 2026-10-11; decided in the plan check under the operator's standing rules, not operator rulings, so the operator may overturn them: the log of a rename (a `deleted` entry on the old name's log, one `renamed` entry on the new), the file format, and 'once per episode' read as once per pass)."

**`holler-pane/src/profile.rs`** (E18): the doc comment of `ProfileStore::rename` drops "PROPOSED" and "Until #665 is confirmed an implementation answers `not-implemented`" and says: "Rename `from` to `to` if `from` is still at `expected_generation`: a record, one `renamed` log entry, a deletion of the old name then a put of the new in the feed. The hub's `profile/rename` also moves every member pane (ADR-0021 section 8); a registry that holds no pane registry has no members." No signature changes.

## The guide and its test: the exact edits

`docs/pane-control.md` (794 lines) and `crates/holler-cli/tests/docs_pane_control_test.rs` (523 lines) change in the same commit, the guide first. The guide's own sections 3, 5, 6, 7, 9 and 12 are touched; **#648 edits sections 4 and 12 of the same file in flight**, so the section 12 edit is the last in the merge order (clash table).

Guide:
1. **Section 3, the opening paragraph** (E50, lines 106 to 108): "`holler profile` has `create`, `delete`, `list`, `show` and `apply`, and three verbs that are PROPOSED and not built (`rename`, `export` and `import` under `profile`, #665), at the end of this section." becomes "`holler profile` has `create`, `delete`, `list`, `show`, `apply`, `rename`, `export` and `import`."
2. **Section 3, delete the subsection "The PROPOSED profile verbs"** (E51, lines 378 to 384) and add, after the `holler profile apply NAME` subsection, three subsections with these exact headings (the docs test computes them from the clap tree, E49): `### \`holler profile rename OLD NEW\``, `### \`holler profile export NAME\``, `### \`holler profile import FILE\``. Each has: one paragraph of what it does and refuses, one fenced `bash` example whose first token is `holler` (so `docs_cli_test` parses it): `holler profile rename fleet fleet-2`, `holler profile export fleet --out fleet.json`, `holler profile import fleet.json --name fleet-copy`, and an "Ends with" line naming the codes of D2/D8/D9 with their exit class (refusals 3, usage 2, `generation-conflict` 1). The import subsection carries the document example of D7 in a fence tagged `json` (a `json` fence outside section 9 is not scanned) and says that the file's positions accept `r2c1`, `c1r2` and `2,1`, that an environment value in the file is refused and never echoed, and that the profile is created **detached**: `holler profile apply` is what makes its panes match. The export subsection says the file holds names and non-secret values only. The rename subsection says that the panes stay attached and are not touched live, that the members' record generation moves by one, and that a rerun completes an interrupted rename.
3. **Section 5, the finding table** (E52): add as the last row, in `FindingKind::ALL` order: `| \`profile-drift\` | A pane's record differs from its profile's spec on at least one field; the message names the fields, not the values. | none |`. In the paragraph after the table, "and four name none" becomes the number of `none` rows of the table **as it stands at the build** (five if nothing else has landed: the table's `none` rows today are `herdr-version-unsupported`, `unregistered-herdr-pane`, `stray-session` and `unmanaged-stalled`, E52). Delete the sentence at line 497 (E53) and add in its place:

   > **`profile-drift`** is raised for a pane whose record differs from its profile's spec on any field `holler profile show` compares, and it is the same comparison, so `show` and `doctor` agree. A spec with no pane and a pane with no spec are not drift. A pass reports one `profile-drift` finding per drifted pane, naming every differing field and no value, and it reports the same finding again in every later pass for as long as the difference stands (a pass keeps no memory, above): that is what "once per episode" means here. No Holler verb picks a side for you: `holler profile show NAME` lists the values, and `holler pane relaunch` changes the pane (and, with `--profile`, its spec with it). When a pass cannot read the profiles it says `observe-failed`, naming `profile_store.list`, and concludes nothing about drift.

4. **Section 6** (E54): the opening paragraph and bullets gain `rename`, `export` and `import` bullets in the style of the existing ones (one line each, pointing at the section 3 subsections), and no line of them says PROPOSED.
5. **Section 7**: after the paragraph that lists the accepted forms, add one sentence: "A profile file (`holler profile import`) takes a position in any of these forms; `holler profile export` writes `rRcC`." The `text` block of grid examples is unchanged.
6. **Section 9, the `profile-drift` row** (E55): `| \`profile-drift\` | failure (1) | A doctor finding (section 5); never an error a verb returns. |` (the class cell comes from `class_of` and is unchanged).
7. **Section 12 "Later"** (E56): delete the last bullet (the #665 one) and change the previous bullet's trailing `;` to `.`.

Test (`docs_pane_control_test.rs`):
1. Delete `PROPOSED_PROFILE_VERBS` and `PROPOSED_FINDING` (E48) and the `if ns == "profile" && PROPOSED_PROFILE_VERBS.contains(&name) { continue; }` of `heading_forms` (E49), so the headings of the three verbs are required.
2. In `every_verb_is_named_and_proposed_lines_say_so` (E49) delete the `proposed_words` loop and rename the test `every_verb_is_named_in_a_span_or_heading`; the `holler roster --profile` guard stays (it is #648's to delete).
3. Add `profile_io_and_drift_are_built_not_proposed`: for each of `profile rename`, `profile export`, `profile import` and `profile-drift`, at least one line of the guide contains it and **no** line containing it contains `PROPOSED`; section 3 has the headings of D11's list, `holler pane import ...` and `holler profile import FILE` as two different headings; the section 5 row for `profile-drift` ends with `none |`; section 12 contains no `#665`. The line check is a function over a string, and a second test feeds it a synthetic guide with a PROPOSED line to prove it fails (the negative control).
4. The existing tests that read `FindingKind::ALL` and `ALL_CODES` (E49 and the section 5 and section 9 tests) now require `profile-drift` in sections 5 and 9, because it is in both lists; the guide edits above satisfy them.

Other test, fixture and stub edits this story owns:
- `crates/holler-cli/tests/fixtures/cli-surface.txt` (E45): under the `# #665` header, delete the line `# (proposed: the operator confirms #665 first)` and replace the three bare lines by at least these: `profile rename | Demo Renamed`, `profile rename | "Some Profile" "Other Name" --format=json`, `profile export | Demo`, `profile export | Demo --out /tmp/demo.json --json`, `profile import | /tmp/demo.json`, `profile import | /tmp/demo.json --name Copy --format=json`. The header `# #665` stays exactly (`the_fixture_groups_new_lines_by_owning_story`, E46).
- `crates/holler-cli/tests/pane_verbs/process/stub.rs` (E47): delete the three `("profile", ..., 665)` entries and **keep the comment line, changed to exactly `// #665`** (it now reads `// #665 (proposed: the operator confirms it)`).
- `crates/holler-cli/tests/profile_verbs/{rename,export,import}.rs` (9 lines each, stub cases) are replaced by the verbs' real tests.

## Acceptance (each is a test)

The issue's two acceptance lines are expanded below. "Both formats" means the text run and the JSON run on freshly seeded rigs (`run_both`, E72), with equal exit codes and a JSON output that passes the test kit's envelope checker. A text refusal is checked by its exit code (the class) and the `error: ` line on stderr; a JSON refusal by `error.code` verbatim. Tests marked (control) are negative controls: they must fail against the named wrong implementation.

**Hub: the transaction** (new `crates/holler-hub/tests/profile_rename_test.rs`, real registries in a temporary state dir, through `profile::dispatch` with both states or `PaneDeps`; E13):
1. **The record.** After a rename the profile is found by NEW (and by any spelling of NEW's slug), is gone under OLD, has generation + 1, NEW's name and slug, the old `created`, the stamp as `updated` and equal specs; the reply `data` equals the stored record.
2. **Every member follows and stays attached.** With three members of OLD, one pane of another profile and one pane in none: the members' `profile` is NEW, each at generation + 1, and each record is otherwise byte-equal to before (compare the serialized records with `profile` and `generation` removed); the other two panes are byte-equal including generation. (control) A transaction that renames the profile only fails this.
3. **Membership is by slug.** A member stored as `NIGHT-SHIFT` of `Night Shift` follows; a pane of a different slug does not.
4. **The log is intact and has one new entry.** `log(OLD)` is the old entries followed by exactly one `deleted` entry (generation + 1, the request's actor, the stamp); `log(NEW)` is exactly one `renamed {from: <OLD's stored display name>}` entry at the new generation, the request's actor and the same stamp; the first part of `log(OLD)` is byte-equal to what it was; `at` never decreases within either. A profile deleted earlier under NEW's slug keeps its older log, and the `renamed` entry follows it.
5. **The feeds.** A watch opened before the rename yields, from the profile feed, the deletion of OLD then the put of NEW at consecutive cursors, and from the pane feed exactly one put per moved member, in name order, at consecutive cursors; a member that already named NEW yields none; a same-slug rename yields one profile put and one log entry.
6. **Refusals change nothing, in the stated order, each with its code over the wire** (the `PaneReply` code, and the files byte-identical, memory identical, no cursor taken, no log entry): `profile-not-found` (whatever the generation); `usage` for NEW equal to OLD; `profile-exists` for a NEW whose slug is a different live profile (whatever the generation, and including another spelling of that slug); `generation-conflict` for a stale and for an ahead generation. A name that differs from OLD only in spelling is accepted.
7. **Fail closed.** With `panes.json` unreadable (or `profiles.json`), the rename answers `store-corrupt` and the other file is byte-identical.
8. **Failed saves.** With the pane save failing, the call answers `unavailable` and both files and both memories are unchanged. With only the profile save failing (after the registry loaded, replace `profiles.json` by a directory, or the technique of `profile_persistence_test.rs`), the call answers `unavailable`, `panes.json` has its previous bytes again, memory is unchanged, and the same rename with the same generation succeeds once the file can be written. (control) A transaction that commits memory before the second save fails this.
9. **Restart.** After a rename, loading both registries again from the files gives the same state, and the tombstone of OLD passes the load check; a watcher resuming across the restart is told of the deletion of OLD.
10. **Retry completes an interrupted rename.** From files in the crash state (the members name NEW, OLD is live at its generation, NEW is free), `rename OLD NEW` succeeds and ends consistent. (control) A profile-first order cannot reach this state, so the test builds the state by hand.
11. **Concurrency.** Two renames of OLD at the same generation (to different names) run at once: exactly one succeeds, the other is `generation-conflict` or `profile-not-found`, and every member names the winner. A member written through `pane/cas_put` while a rename runs either lands before (and is moved) or is refused after (`generation-conflict` or `profile-not-found`), never lost. The test has a timeout so a lock-order bug fails instead of hanging.
12. **The membership hook after a rename.** A pane write naming OLD is `profile-not-found`; naming NEW succeeds.
13. **The three `not-implemented` pins are replaced** (E14, E15, E16): `profile/rename` with no params is a `usage` error from the real handler like the other writes.

**Conformance suite and fake** (testkit crate, E68 to E71): `rename-is-not-implemented` is replaced by cases that the fake and `ProfileState` (the port, the record half) both pass: moves the record (name, slug, generation, `created` kept, `updated` stamped); logs `renamed` on the new name and closes the old log with `deleted`; the feed is deletion then put (one put for a same-slug rename); `profile-not-found`, `profile-exists`, `generation-conflict` and the exact-same-name `usage` each change nothing; the case at E69 uses a refused rename as its fifth refused write. `profile_store_conformance_test.rs`'s mutants fail the new cases (control), and `FakeProfileStore`'s `ProfileStoreOp::Rename` fault switch still applies.

**CLI `profile rename`** (`crates/holler-cli/tests/profile_verbs/rename.rs`, both formats): the success text and JSON of Scope A exactly; the call the fake received carries the generation the verb read and the actor `holler profile rename`; a name with spaces; `profile-not-found`, `profile-exists` and the same-name `usage` (exit 3, 3, 2); `generation-conflict` (exit 1) through the fake's `concurrent_put`; one positional is a usage envelope; no adapter or probe call (`assert_no_adapter_call`).

**CLI `profile export`** (`export.rs`): both formats; the document of D7 byte for byte for a profile with every field set, and for one without `opencode_agent`, `command` and `check`; positions are written `rRcC` for several cells; no `slug`, `generation`, `created`, `updated` or log in it; **no environment value**: with a canary variable set in the test's process environment and the profile's env names equal to it, the canary value appears in neither the file nor stdout nor stderr (control: a verb that copied the environment's values fails it); `--out` writes the file at mode 0600 and replaces an existing one, and leaves no temporary file; a missing parent directory is `usage` (exit 2) and writes nothing; no `--out`: the document is stdout in text and `data.document` in JSON; `profile-not-found`.

**CLI `profile import`** (`import.rs`, using `tempfile`, both formats): `r2c1`, `c1r2`, `2,1` and `R2C1` in a file all load as row 2, column 1 (compare the stored specs); a position given as the record's `{row, col}` object loads too; the stored profile is at generation 1, with the actor `holler profile import`, detached (the pane store's call log shows no read or write of a pane); a spec that names a pane of another profile or no pane is accepted; `--name` overrides the file's name and is itself parsed; **each refusal has its code in both formats and writes nothing** (the profile store's log shows no `cas_put`): `profile-secret-refused` (an env entry `NAME=value`, and a bare string with `=`), `command-not-argv` (a `command` and a `check` that are strings), `profile-exists` (the name taken, the slug taken in another spelling, `--name` taken), `grid-ambiguous` (`hj-c1r2`), `grid-out-of-range` (`r0c1`), `env-name-invalid` (a blank name), and `usage` (not JSON, an unknown member, a wrong `format`, a newer `version`, a missing `name` with no `--name`, a directory, a missing file, a file over 1 MiB, one missing positional); **the refused value is never printed**: a canary secret in the file appears in neither stdout nor stderr nor any envelope (control: a verb that echoed serde's message fails it); **round trip:** export of a profile with every field, then import under `--name Copy` into a fresh rig, gives a profile whose specs equal the original's (`==` on `Vec<ProfileSpec>`) and whose name is `Copy`.

**Doctor: `profile-drift`** (new `crates/holler-cli/tests/pane_verbs/doctor/drift.rs`, declared from `doctor.rs`, over the doctor rig; `holler-pane` has no dev-dependency on the test kit, so the pass is run through the rig as the other doctor tests do):
14. **It agrees with `profile show` on the same fixture.** One profile whose specs and members are: a pane that matches; a pane differing in one field; a pane differing in three (`herdr.grid`, `model.model_id`, `context.hard`); a spec with no pane; a member with no spec. A reconcile pass and `holler profile show --format=json` over the same ports give the same set of drifted panes and, for each, the same field paths in the same order; `Missing` and `Extra` rows produce no finding. (control) A comparison written separately and wrong on `env` as a set (the table treats `env` and `expect` as sets, E32) fails a case where only the order of `env` differs: no drift.
15. **Once per episode.** The pane differing in three fields yields exactly one finding, naming all three paths; two passes yield the same one finding each (no memory, no extra); after the record is repaired a pass yields none; after it drifts again a pass yields one. (control) A one-finding-per-field implementation fails the first assertion.
16. **A pass stays a read.** With a profiled pane in scope, a pass makes exactly one `profile_store.list` and no profile-store write; the pane-store calls are unchanged (`read_only.rs`'s allowlist, E36, is amended to allow `ProfileStoreOp::List` and nothing else); with no profiled pane in scope the profile-store log is empty, as today.
17. **Scope.** `--pane X` reports X only; `--profile P` reports P's panes only; a whole-fleet pass reports every profile's members.
18. **A failed read.** With `profile_store.list` failing, the pass has one `observe-failed` finding whose message names `profile_store.list` and the code, no `profile-drift`, every other finding of the pass unchanged, and the verb exits 0.
19. **Shape.** The finding has `kind` `profile-drift`, the pane and its position from the record, `remedy` `null`, `fix` `not-fixable`, `fix_error` `null`; the text line is `<pane> <pos> profile-drift: <message>` with no `(run: ...)`; the message quotes the profile name through `findings::quoted` (a name with a double quote and a backslash is escaped) and contains no field value (a cwd and a command in the spec do not appear).
20. **The registry of kinds.** `findings_test.rs`'s `CODES` list gains `profile-drift` last (E35) and the test still checks uniqueness, kebab-case and the serde form; `class_of("profile-drift")` is still a failure (1).
21. **The existing doctor tests pass unchanged** with the rig's profiles built from the stored records (D10): the every-incident rig has no profile and `calls.profiles` stays empty there.

**Process level** (`crates/holler-cli/tests/pane_verbs/process/`, the real binary against a `LiveHub`): `holler profile rename a b` on an empty hub exits 3 with `profile-not-found` (not `not-implemented`: the verb reaches the real handler); `holler profile import FILE` then `holler profile export NAME --out OUT` then a comparison of the two documents (equal), then `holler profile rename NAME OTHER` and `holler profile show OTHER` show the profile; `STUBS` has no #665 entry.

**Docs and surface:** the guide and its test as listed above; `docs_cli_test` parses every new example and ADR row; `cli_surface_test` (every fixture line parses, the leaf set equals the clap tree's); the ADR rows test (one row per verb, rows of two stories never adjacent). **Lint:** no `.rs` file reaches 900 lines (`scripts/lint.sh` check 4), clippy clean, `cargo fmt` per crate (`cargo fmt -p holler-hub` and so on; never `cargo fmt --all`), no new `unsafe`.

## Blast radius

Sizes are `wc -l` at `bda1069`; the lint fails a `.rs` file at 900 lines. A file near the limit after the change is split, not squeezed.

- **ADRs and docs:** `docs/adr/ADR-0021.md` (1121), `docs/adr/ADR-0003.md` (119), `docs/pane-control.md` (794), `CHANGELOG.md` (982: one bullet at the top of `## [Unreleased]` / `### Enhancements`, linking `/issues/665`, in the style of the entries there, saying what the three verbs and the finding do and that rename is one hub-side transaction).
- **holler-hub:** `crates/holler-hub/src/profile/rename.rs` (25, the dispatch and the transaction), `profile/store.rs` (415, `Store::rename`, expected to reach about 520), `profile/mod.rs` (216, the port method and the module docs), `profile/entry.rs` (207, only if a tombstone constructor is needed), `panes/mod.rs` (213) and `panes/store.rs` (382, the pane half: one multi-change commit). Tests: `crates/holler-hub/tests/profile_rename_test.rs` (new), `pane_dispatch_test.rs` (378) and `profile_handlers_test.rs` (577), where the `not-implemented` pins change.
- **holler-pane:** `src/findings.rs` (357), `src/reconcile.rs` (450: one `mod` line and one call), `src/reconcile/drift.rs` (new), `src/profile.rs` (408: a doc comment), `src/reply.rs` (200) and `crates/holler-proto/src/methods.rs` (219): comment-only edits that drop "PROPOSED" from `ProfileRenameParams` and `profile/rename`. Test: `tests/findings_test.rs` (45).
- **holler-pane-testkit:** `src/profile_store.rs` (360, the fake's `rename`), `src/conformance/profile_store.rs` (527), `src/conformance/profile_store/log.rs` (129) and `watch.rs` (144), `tests/profile_store_conformance_test.rs` (239, the mutants). **These are outside the issue's blast radius** and are added here because the suite pins `rename` as `not-implemented` (E68, E69, E71).
- **holler-cli (src):** `src/profile/rename.rs`, `export.rs`, `import.rs` (22 each), `src/profile/mod.rs` (51: its doc comment only, "wait for the operator to confirm them"; the dispatch is untouched), `src/pane/wiring/store.rs` (397: a doc comment, E40's header). No other source file.
- **holler-cli (tests):** `tests/profile_verbs/{rename,export,import}.rs` (9 each), `tests/profile_verbs/rig.rs` (323: only if a helper is needed, no `Ports` literal is added), `tests/pane_verbs/doctor.rs` (560: one `mod drift;`), `doctor/drift.rs` (new), `doctor/rig.rs` (434: `profiles_of`), `doctor/read_only.rs` (147: the profile-call assertion), `tests/pane_verbs/process/stub.rs` (233), a new process test file in the same directory and its `mod` line in `process/main.rs`, `tests/fixtures/cli-surface.txt` (194), `tests/docs_pane_control_test.rs` (523).
- **Not touched:** `crates/holler-cli/src/pane/wiring/bind.rs` (D6), `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/lib.rs`, the `Ports` literals (no field is added), `crates/holler-cli/src/cli.rs`, `docs/protocol/v2.md`, `docs/orchestrating.md`.

## Clashes with stories in flight

State on 2026-10-11 at 19:52 MDT, read only (branches by ref; the uncommitted state of #831 read with `git diff` in its worktree). Every branch below is moving; re-read this table at the plan check.

| Story | Shared files and hunks | What #665 does about it |
|---|---|---|
| **#647 part 2** (`agent-cannot-dispatch`, `unattached`; operator allowed the contract change, E76; its brief is being prepared in the worktree `0647-agent-cannot-dispatch`) | `findings.rs` (new kinds in the enum, `ALL`, `code`, `remedy`), `reconcile.rs` and `reconcile/observe.rs`, `tests/findings_test.rs` (`CODES`), ADR-0021, `docs/pane-control.md` section 5 (the table and the "N name none" sentence), `CHANGELOG.md`; it adds read-only port methods that `bind.rs` must forward | `profile-drift` is **appended last** on whatever base the build starts from; the `CODES` array, the section 5 row and the "none" count are recomputed at the rebase. The comparison is in its own new file, and `reconcile.rs` gets one `mod` line and one call, the smallest hunk. #665 adds no port method. |
| **#831** Claude Code harness (branch `issue-831-implementation`, uncommitted work in its worktree) | Appends `pane-waiting` and `claude-login-missing` to the same enum, `ALL`, `code` and `remedy` (E73); `reconcile.rs` and `reconcile/observe.rs` (wide, mechanical); ADR-0021 (a new section 15, "Decisions taken" items 22 to 25); `CHANGELOG.md`; it calls "once per pass" for its login kind, which is the reading D10 takes | Same as above: append last, rebase. The "Decisions taken" number is the next free one at merge. |
| **#839** DRIVEN follows the session of record (absorbs #828; brief on `issue-839-implementation`, no code) | `ports.rs` and **every `Ports { .. }` literal** (a new `hub` field; 15 literals), `reconcile.rs`, `reconcile/observe.rs`, `findings.rs` (the mismatch's doc and remedy), ADR-0021 sections 2, 8, 11 and "Deferred", `docs/pane-control.md` sections 3, 4, 10, 12, `CHANGELOG.md` (E74) | #665 adds **no `Ports` literal** and no `Ports` field; its tests use the existing rigs (`Rig::ports`, the doctor rig's `ports_with`). #839's literal edits therefore do not touch #665's new files. The hunks in `findings.rs` and `reconcile.rs` are adjacent but distinct. |
| **#648** roster (branch `issue-648-implementation`, brief only) | `docs/pane-control.md` sections 4 and 12, **`docs_pane_control_test.rs`: the same test function** `every_verb_is_named_and_proposed_lines_say_so` (it deletes the roster guard at lines 181 to 183) and `section_4_marks_the_roster_extension_later` (E75), `CHANGELOG.md`, ADR-0003 line 94 | The two edits to that one test function are adjacent lines and **will conflict**; whoever merges second resolves by taking both deletions. Section 12's last bullets conflict too. #665 merges after #648 (below). |
| **#835** import leaves panes it does not drive alone (branch `issue-835-implementation`, at T-green) | `docs_pane_control_test.rs`, `cli-surface.txt` (its `pane import` line), ADR-0003 (the `pane import` row, line 70, and ADR-0021 sections 2, 3, 9, 13), `docs/pane-control.md` (the import paragraphs), `CHANGELOG.md`, `wiring/bind.rs`, the `Ports` literals of three rigs | The ADR-0003 row edit (line 70) and #665's (lines 72 to 74) are separated by the blank line that the rows test requires, so they do not overlap. No `bind.rs` edit in #665. |
| **#667** `profile apply` scenario (brief only) | `docs/pane-control.md` section 12 (one bullet), `CHANGELOG.md`, `pane_integration/**` | Section 12 conflict, mechanical. |
| **#851** `--version` prints the commit (branch `issue-851-implementation`) | ADR-0003 header and the `--version` row, `stub.rs` (the `--version` assertion), `docs/pane-control.md` (a cutover paragraph), `docs/releasing.md` | `stub.rs` is also #665's file (the `STUBS` table, lines 18 to 31, against #851's lines near 178): different hunks. No header line is added to ADR-0003 by #665 on purpose. |

**Recommended merge order:** #835, then #831, then #648 (any order among these, each built or nearly built), then #839, then #647 part 2, then #667, then **#665 last**. Reasons: #665 is off the critical path and nothing waits for it; the story is new files plus a handful of one-line hunks, so it rebases cheaply onto any order, while the stories that append kinds, edit the same test function or change `Ports` have code in flight that a late #665 does not disturb. Its three guide hunks (sections 3, 5 and 12) and the `CODES` list are the only places a rebase needs thought. If #647 part 2 or #831 cannot merge first, #665 may go first and they append after `profile-drift`; the rebase is the same size.

## Depends on

Verified with `gh issue view` on 2026-10-11 (times in MDT):

- **Merged, and this story uses what each built:** #637 (the `holler-pane` crate, closed 2026-10-09 05:55), #638 (the test kit, 2026-10-09 15:37), #661 (the profile registry, 2026-10-09 15:47), #662 (`profile create`, `delete`, `list`, `show` and `profile_diff`, 2026-10-09 21:48), #669 (the control-method plumbing and `profile/rename.rs`, 2026-10-09 07:37), #670 (the CLI skeleton, 2026-10-09 09:54), #676 (exit classes, 2026-10-09 11:28), #664 (`profile apply`, 2026-10-11 12:14), #650 (`pane import`, 2026-10-11 12:24), #700 (`opencode_agent` in `profile_diff`, 2026-10-10 11:14), #832 (unmanaged, 2026-10-11 15:35), #649 (the wiring, 2026-10-11 17:35), #652 (the guide, 2026-10-11 18:13).
- **#647:** part 1 (the reconcile engine and `pane doctor`, PR #701) is merged and is what this story extends; the issue stays open for part 2 (the clash table). The issue's "#647 owns these files and has merged before this starts" is true for part 1 only.
- **Nothing is waited for.** The issue's own dependency line (#637, #638, #647, #661, #662, #669, #670) is satisfied.

## Pipeline

`rigor: second-opinion` (D12). Fresh worktree, branch `issue-665-implementation`. `uiSurface: false`, so D and U never run. Phase F runs on `claude-opus-5-5`. The first commit is the contract amendment (D1); the tests are written from this brief and confirmed RED before any code. The verify command is the workspace test of `.aftersight/pipeline.config.json`, which includes `docs_cli_test`, `cli_surface_test`, `docs_pane_control_test` and the new targets.

## Rules for this story

See the epic (#633): the contract section there is fixed, so this story builds against it, amending first where it must. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt` per crate, clippy clean, no new `unsafe`). Edit only the files in the Blast radius. No story touches a live fleet, a running pane or a real Herdr session: verification is the fakes and temporary directories, and the process tests use a hub on its own temporary state dir. This repository is public: no personal host, tailnet, address or account name in a file, a test or a message. Never print or commit a credential; a test canary is a made-up string. A file path in a test is a `tempfile` path. A secret never goes in a file that is not git-ignored without the operator's consent.

## Questions only the operator can answer

None blocks the build; each has a default that the brief already follows.

1. **Rigor.** The issue says `in-session`; the brief says `second-opinion` (D12) because of the two-file transaction, the new finding kind and the new file format. Default: `second-opinion`.
2. **One story or three?** The three parts (A rename, B export and import, C drift) share only the docs, the fixture and the CHANGELOG. Default: one story, because the issue is one story and the shared files are small.
3. **What "one change-log entry" means for a rename (D4).** The brief gives the renamed profile's log exactly one new entry (`renamed`) and closes the old name's log with a `deleted` entry, copying nothing. If the operator wants `profile log NEW` to show the old history too, the log of OLD is copied under NEW instead, which changes D4 and the load check. Default: as D4.
4. **"Raised once per episode" (D10).** The brief reads it as the engine reads it: once per pass per drifted pane, and again in every later pass while the difference stands. If the operator wants a drift reported once and then muted until it clears, that needs a stored marker on a frozen record and a write by doctor, which is a contract amendment of its own. Default: as D10.

## Not verified

- No test, build, clippy or lint was run for this brief: it prepares, it does not build. Every claim about behaviour is read from the code at `bda1069` (the excerpts), not executed.
- That the existing doctor tests would fail with drift enabled and the rig's `sample_profile`s: reasoned from E37 and E38 (port policy `fixed` against `fixed:<port>`, grid `r1c1` against the seeded cell), not run. D10 changes the rig anyway.
- That `write_atomic` and `serde_json` are reachable from `holler-cli`'s sources as written: `serde_json` and `holler-proto` are `[dependencies]` of the crate (read from its manifest); `write_atomic` is a public function of `holler_proto::atomic_file`, not yet called from the CLI.
- The technique for failing only the profile save in test 8 (replacing `profiles.json` by a directory after load) was not tried; `profile_persistence_test.rs` shows two other techniques (a mode-000 file, a read-only directory). A read-only `hub` directory fails both saves, which is the pane-save case.
- That `LiveHub` (the process tests' hub) serves `profile/*` over the control socket the way the wiring expects: the stub tests use it only for `control/status`. The process test in "Acceptance" is the first to need more, and may need the hub's own state dir seeded.
- The behaviour of `rename` under a real concurrent `pane watch` consumer, and the I5 bound of a call that holds both locks across two saves: the registries' own operating limits say a wedged save blocks every caller, with no timeout; the CLI client's 10 s bound applies.
- A pane that names a profile which does not exist (after the interrupted rename of D2, or the check-then-write gap) is not reported by any finding; a follow-up (`pane-profile-missing`) is suggested, not filed.
- The in-flight branches (E73 to E76) were read at the shas and times stated; #831's findings diff was uncommitted.

## AI assistance

Drafted by Claude (Sonnet 5.5) for story #665, from the issue and its operator comment, `main` at `bda1069` and the read-only state of the sibling branches named above, at the operator's direction, per `CONTRIBUTING.md`. The decisions marked "decided by the preparer" are the preparer's, to be confirmed at the plan check; the maintainer filing the story is accountable for it. This brief has not been through any review.

## Evidence (verbatim)

Cut on 2026-10-11 at 19:58 MDT from `main` at `bda1069`, except where the command names a branch or a worktree. Every excerpt below was produced by a command (shown under its heading) that pipes the source through `sed -n 'A,Bp'`; the line numbers on the left were added by `awk` after the cut and are the file's own. Nothing is retyped. Commands run from the repository root; the `../0831-claude-harness` path in E73 is a sibling worktree of the story's worktree. An excerpt of an open story's branch (E73 to E76) is a moving target and is labelled IN FLIGHT.

**E1: `gh issue view 665 --json body -q .body`, lines 1-12** — The issue as filed (2026-10-08/09): the scope, the acceptance lines and the pipeline line. The brief supersedes it where they differ.

Command: `gh issue view 665 --json body -q .body | sed -n '1,12p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	Part of epic #633. (added 2026-10-08, profiles) **PROPOSED: the operator confirms before any pane picks this up. Not on the critical path; nothing waits for it.**
2	
3	## Scope
4	- `holler profile rename OLD NEW`: one hub-side transaction that changes the name and slug and every member's `Pane.profile`, with one change-log entry (handler `profile/rename`, reserved by #637).
5	- `holler profile export NAME [--out FILE]` and `holler profile import FILE [--name NAME]`: a profile as a file (specs only; no generation, no log). Names and non-secret values only (I7): import refuses a file with an env value (`profile-secret-refused`) and an existing name (`profile-exists`). (amended 2026-10-08, features) `export` writes `command` and `check` as argv arrays, and `import` refuses a shell string with `command-not-argv`. Positions in the file go through the one `GridPos` parser (#637), so `r2c1`, `c1r2` and `2,1` (row,col) all load as row 2, col 1; export writes `rRcC`. (amended 2026-10-08, grid)
6	- All three take `--format=text|json`.
7	- A doctor finding `profile-drift` (a live pane differs from its spec), raised by reconcile through the comparison in `crates/holler-pane/src/profile_diff.rs` (#662).
8	
9	(amended 2026-10-09, split) Depends on #637 and #670 (CLI side), and #661 and #669 (hub side). The positionals and flags of `profile rename OLD NEW`, `export NAME [--out FILE]` and `import FILE [--name NAME]` are yours: they live in `rename.rs`, `export.rs` and `import.rs` with your ADR 0003 rows, `cli-surface.txt` lines and `crates/holler-cli/tests/profile_verbs/<verb>.rs` files. `ProfileStore::rename(old, new, expected_generation)` and `profile/rename` are declared (PROPOSED) by #637 with a `NotImplemented` stub; the hub-side transaction is yours. #669 pre-creates the empty `holler-hub/src/profile/rename.rs`, already declared in `profile/mod.rs`, so you edit none of #661's files. `ProfileDrift` is added to #647's `findings.rs` after #647 merges.
10	
11	(amended 2026-10-09, decisions) The `EnvVarName` guard in `holler-pane` raises `profile-secret-refused` for a `NAME=value` entry, so import's refusal comes from the typed decode: do not add a second scan. The writing methods take an `Actor` (#637).
12	
~~~~

**E2: `gh issue view 665 --json comments -q '.comments[-1].body'`, lines 1-3** — The operator's confirmation (comment, 2026-10-11 19:40 MDT).

Command: `gh issue view 665 --json comments -q '.comments[-1].body' | sed -n '1,3p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	Operator decision, 2026-10-11: **confirmed, build it.** `profile rename`, `export`, `import` and the `profile-drift` doctor finding are in scope for #633. It stays off the critical path: nothing waits on it.
2	
3	Filed by an AI agent (Claude Code) at the operator's direction.
~~~~

**E3: `crates/holler-hub/src/profile/rename.rs:1-25`** — The hub's `profile/rename` stub, pre-created by #669, which this story fills.

Command: `sed -n '1,25p' crates/holler-hub/src/profile/rename.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! `profile/rename` (PROPOSED, #665): one hub-side transaction that renames a profile
2	//! and moves every member pane's `Pane.profile` to the new name.
3	//!
4	//! Skeleton (issue #669): [`dispatch`] answers `not-implemented`. [`super::dispatch`]
5	//! already routes `profile/rename` here, so #665 fills this file and edits no file of
6	//! #661.
7	
8	use holler_pane::{PaneError, PaneReply};
9	use holler_proto::CorrelationId;
10	use serde_json::Value;
11	
12	use super::ProfileState;
13	use crate::pane_dispatch::reply_line;
14	use crate::panes::PaneState;
15	
16	/// Answer `profile/rename`: `not-implemented` until #665. `panes` is the pane
17	/// registry that the transaction updates alongside the profile.
18	pub async fn dispatch(
19	    cid: &CorrelationId,
20	    _obj: &Value,
21	    _profiles: &ProfileState,
22	    _panes: &PaneState,
23	) -> String {
24	    reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented))
25	}
~~~~

**E4: `crates/holler-hub/src/profile/mod.rs:155-216`** — The port's `rename` on `ProfileState` (a `NotImplemented` stub), `dispatch`, which already routes `profile/rename` with both registries' state, and `check_membership`, whose lock is released before the pane lock is taken.

Command: `sed -n '155,216p' crates/holler-hub/src/profile/mod.rs | awk -v s=155 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
155	    /// PROPOSED (#665): `not-implemented`, touching nothing, whatever the registry holds.
156	    /// #665 adds the rename to the store when it is confirmed (D12).
157	    fn rename(
158	        &self,
159	        _from: &ProfileName,
160	        _to: &ProfileName,
161	        _expected_generation: u64,
162	        _actor: &Actor,
163	    ) -> Result<Profile, PaneError> {
164	        Err(PaneError::NotImplemented)
165	    }
166	}
167	
168	/// Answer one `profile/*` request with its handler. `obj` is the whole request frame.
169	///
170	/// `panes` is the pane registry, which `profile/rename` (#665) reaches to move a profile's
171	/// member panes to the new name; no other method reads it. `profile/watch` is long-poll:
172	/// one `{events, cursor}` reply per request, after at most `WATCH_WAIT`.
173	pub async fn dispatch(
174	    method: &str,
175	    cid: &CorrelationId,
176	    obj: &Value,
177	    profiles: &ProfileState,
178	    panes: &PaneState,
179	) -> String {
180	    let store = Arc::clone(&profiles.store);
181	    match method {
182	        "profile/get" => handlers::get(cid, obj, store).await,
183	        "profile/list" => handlers::list(cid, obj, store).await,
184	        "profile/cas_put" => handlers::cas_put(cid, obj, store).await,
185	        "profile/delete" => handlers::delete(cid, obj, store).await,
186	        "profile/watch" => handlers::watch(cid, obj, store).await,
187	        "profile/log" => handlers::log(cid, obj, store).await,
188	        "profile/rename" => rename::dispatch(cid, obj, profiles, panes).await,
189	        // `forward` sends only the methods of `PROFILE_METHODS`, and each has an arm above.
190	        // A method added to that list before its handler answers like a stub.
191	        _ => reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented)),
192	    }
193	}
194	
195	/// The membership hook of a pane write (D8): `pane/cas_put` (#639) calls it before the
196	/// compare-and-swap, on the blocking pool and before the pane lock is taken, so no lock is
197	/// held inside another.
198	///
199	/// A pane outside any profile is accepted without reading the profile registry. A pane
200	/// that names a profile needs that profile to exist, found by slug as every lookup is:
201	/// `profile-not-found` when it does not, and the registry's own `store-corrupt` when it
202	/// has failed (fail closed, never accepted). Every write naming a profile is checked, not
203	/// only one that changes `Pane.profile`, because the hook does not see the stored pane
204	/// (D9). Moving a pane from one profile to another is the pane registry's refusal, made
205	/// inside its compare-and-swap (see the module docs).
206	pub fn check_membership(pane: &Pane, profiles: &ProfileState) -> Result<(), PaneError> {
207	    let Some(name) = &pane.profile else {
208	        return Ok(());
209	    };
210	    match profiles.store.get(name)? {
211	        Some(_) => Ok(()),
212	        None => Err(PaneError::ProfileNotFound {
213	            what: name.to_string(),
214	        }),
215	    }
216	}
~~~~

**E5: `crates/holler-hub/src/profile/store.rs:190-269`** — `cas_put`, `delete` and `commit`: one write is one lock, one check order, one stamp, one entry, one save, then memory.

Command: `sed -n '190,269p' crates/holler-hub/src/profile/store.rs | awk -v s=190 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
190	    /// Store `profile` as `actor` if the stored record is still at `expected` (0 for a new
191	    /// profile), and log the write. The stored record is `profile` with the name's slug,
192	    /// the next generation and the hub's stamps: the submitted slug, generation, `created`
193	    /// and `updated` are ignored. Returns the stored record.
194	    pub(crate) fn cas_put(
195	        &self,
196	        profile: &Profile,
197	        expected: u64,
198	        actor: &Actor,
199	    ) -> Result<Profile, PaneError> {
200	        let mut guard = self.lock();
201	        let table = guard.as_mut().map_err(|err| err.clone())?;
202	        let slug = profile.name.slug();
203	        let previous = table.entries.get(&slug);
204	        let stored = previous.and_then(ProfileEntry::profile);
205	        check_name(stored, &profile.name, &slug)?;
206	        let generation = next_generation(stored.map_or(0, |stored| stored.generation), expected)?;
207	        let at = entry::stamp(previous, now_millis());
208	        let record = Profile {
209	            slug: slug.clone(),
210	            generation,
211	            created: stored.map_or(at, |stored| stored.created),
212	            updated: at,
213	            ..profile.clone()
214	        };
215	        let change = stored.map_or(ProfileChange::Created, |stored| ProfileChange::Updated {
216	            summary: entry::summary(&stored.panes, &record.panes),
217	        });
218	        let event = ProfileEvent {
219	            cursor: table.next_cursor()?,
220	            name: record.name.clone(),
221	            profile: Some(Box::new(record.clone())),
222	        };
223	        let logged = log_entry(at, generation, actor, change);
224	        let next = ProfileEntry::next(previous, slug, event, logged);
225	        self.commit(table, next)?;
226	        Ok(record)
227	    }
228	
229	    /// Delete the profile filed under the slug of `name` as `actor` if it is still at
230	    /// `expected`, and log the delete at the deleted generation + 1. A missing profile is
231	    /// `profile-not-found` whatever `expected` is: that is checked before the generation.
232	    /// The tombstone keeps the stored display name, whatever spelling `name` used.
233	    pub(crate) fn delete(
234	        &self,
235	        name: &ProfileName,
236	        expected: u64,
237	        actor: &Actor,
238	    ) -> Result<(), PaneError> {
239	        let mut guard = self.lock();
240	        let table = guard.as_mut().map_err(|err| err.clone())?;
241	        let slug = name.slug();
242	        let previous = table.entries.get(&slug);
243	        let Some(stored) = previous.and_then(ProfileEntry::profile) else {
244	            return Err(PaneError::ProfileNotFound {
245	                what: name.to_string(),
246	            });
247	        };
248	        let generation = next_generation(stored.generation, expected)?;
249	        let at = entry::stamp(previous, now_millis());
250	        let event = ProfileEvent {
251	            cursor: table.next_cursor()?,
252	            name: stored.name.clone(),
253	            profile: None,
254	        };
255	        let logged = log_entry(at, generation, actor, ProfileChange::Deleted);
256	        let next = ProfileEntry::next(previous, slug, event, logged);
257	        self.commit(table, next)
258	    }
259	
260	    /// Save the table with `change` applied, then apply it to memory and wake the watchers.
261	    /// Twin of `panes::store::Store::commit`.
262	    fn commit(&self, table: &mut Table, change: ProfileEntry) -> Result<(), PaneError> {
263	        self.save(table, &change)?;
264	        table.head = change.event.cursor;
265	        table.ring.push(change.event.clone());
266	        table.entries.insert(change.slug.clone(), change);
267	        self.changed.notify_all();
268	        Ok(())
269	    }
~~~~

**E6: `crates/holler-hub/src/profile/store.rs:390-415`** — `check_name` (the slug is the identity; a display name changes only through a rename) and `log_entry`.

Command: `sed -n '390,415p' crates/holler-hub/src/profile/store.rs | awk -v s=390 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
390	/// The name rule of a write (D6): the slug is a profile's identity, so a write of `name`
391	/// while its slug is filed under another live name is `profile-exists`, whatever the
392	/// generation. A display name changes only through a rename (#665). A tombstone holds no
393	/// name: after a delete, a create may spell the name another way, and its log goes on.
394	fn check_name(stored: Option<&Profile>, name: &ProfileName, slug: &str) -> Result<(), PaneError> {
395	    match stored {
396	        Some(stored) if stored.name != *name => Err(PaneError::ProfileExists {
397	            what: format!(
398	                "{:?} has the slug {slug:?} of the stored profile {:?}",
399	                name.as_str(),
400	                stored.name.as_str()
401	            ),
402	        }),
403	        _ => Ok(()),
404	    }
405	}
406	
407	/// The log entry of a write by `actor`, stamped `at`, that left the profile at `generation`.
408	fn log_entry(at: i64, generation: u64, actor: &Actor, change: ProfileChange) -> ProfileLogEntry {
409	    ProfileLogEntry {
410	        at,
411	        generation,
412	        actor: actor.clone(),
413	        change,
414	    }
415	}
~~~~

**E7: `crates/holler-hub/src/profile/entry.rs:88-123`** — The checks an entry adds on load (a tombstone's log ends with `Deleted`; a live record's log ends at its generation) and `ProfileEntry::next`.

Command: `sed -n '88,123p' crates/holler-hub/src/profile/entry.rs | awk -v s=88 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
88	/// The checks an entry adds to `persist`'s when it is read (see the module docs). The
89	/// messages name no content: the loader reports a failed check by its position only.
90	fn check(slug: &str, event: &ProfileEvent, log: &[ProfileLogEntry]) -> Result<(), &'static str> {
91	    if event.name.slug() != slug {
92	        return Err("the entry's event names a profile of another slug");
93	    }
94	    let Some(last) = log.last() else {
95	        return Err("the entry's change log is empty");
96	    };
97	    let deleted = matches!(last.change, ProfileChange::Deleted);
98	    match event.profile.as_deref() {
99	        Some(record) if record.name != event.name => {
100	            Err("the entry's event and record name the profile differently")
101	        }
102	        Some(record) if deleted || last.generation != record.generation => {
103	            Err("the entry's change log does not end with a write at its record's generation")
104	        }
105	        None if !deleted => Err("the entry's change log does not end with its deletion"),
106	        _ => Ok(()),
107	    }
108	}
109	
110	impl ProfileEntry {
111	    /// The entry that a write filed under `slug` leaves: its `event`, and the log of
112	    /// `previous` (the entry filed under the slug before, a tombstone's included) with
113	    /// `logged` appended. The log therefore goes on across a delete and a re-create.
114	    pub(crate) fn next(
115	        previous: Option<&Self>,
116	        slug: String,
117	        event: ProfileEvent,
118	        logged: ProfileLogEntry,
119	    ) -> Self {
120	        let mut log = previous.map_or_else(Vec::new, |entry| entry.log.clone());
121	        log.push(logged);
122	        Self { slug, event, log }
123	    }
~~~~

**E8: `crates/holler-hub/src/profile/entry.rs:170-181`** — `stamp`: the one time a write takes, never before the entry's last `at`.

Command: `sed -n '170,181p' crates/holler-hub/src/profile/entry.rs | awk -v s=170 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
170	/// The time a write to `entry` (`None` for a slug never written) is stamped with (D4):
171	/// `now`, unless the entry already holds a later time, the last `at` of its log or its
172	/// record's `updated`. The one stamp is the log entry's `at`, the record's `updated`, and a
173	/// create's `created`, so a log's `at` never decreases and a record's `updated` never goes
174	/// backwards, even when the clock steps back.
175	pub(crate) fn stamp(entry: Option<&ProfileEntry>, now: i64) -> i64 {
176	    entry.map_or(now, |entry| {
177	        let logged = entry.log.last().map_or(now, |last| last.at);
178	        let updated = entry.profile().map_or(now, |record| record.updated);
179	        now.max(logged).max(updated)
180	    })
181	}
~~~~

**E9: `crates/holler-hub/src/panes/store.rs:163-237`** — The pane registry's `cas_put`, `delete`, `commit` and `save`: one change per write, a failed save changes nothing.

Command: `sed -n '163,237p' crates/holler-hub/src/panes/store.rs | awk -v s=163 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
163	    /// Store `pane` if the stored record is still at `expected` (0 for a new pane). The
164	    /// stored record is `pane` with only its generation replaced: the submitted generation
165	    /// is ignored and nothing else is inferred. Returns the stored record.
166	    ///
167	    /// The membership rule runs here, after the generation and under the lock (#661, see
168	    /// [`refuse_profile_move`]), so it holds on every path into the registry, the port and
169	    /// `pane/cas_put` alike, and no writer can slip in between the check and the write.
170	    pub(crate) fn cas_put(&self, pane: &Pane, expected: u64) -> Result<Pane, PaneError> {
171	        let mut guard = self.lock();
172	        let table = guard.as_mut().map_err(|err| err.clone())?;
173	        let current = table.record(&pane.name);
174	        let generation = next_generation(current.map_or(0, |stored| stored.generation), expected)?;
175	        refuse_profile_move(current, pane)?;
176	        let stored = Pane {
177	            generation,
178	            ..pane.clone()
179	        };
180	        let change = PaneEvent {
181	            cursor: table.next_cursor()?,
182	            name: pane.name.clone(),
183	            pane: Some(Box::new(stored.clone())),
184	        };
185	        self.commit(table, change)?;
186	        Ok(stored)
187	    }
188	
189	    /// Remove the record named `name` if it is still at `expected`. A missing record is
190	    /// `pane-not-found` whatever `expected` is: that is checked before the generation.
191	    pub(crate) fn delete(&self, name: &PaneName, expected: u64) -> Result<(), PaneError> {
192	        let mut guard = self.lock();
193	        let table = guard.as_mut().map_err(|err| err.clone())?;
194	        let Some(current) = table.record(name).map(|stored| stored.generation) else {
195	            return Err(PaneError::PaneNotFound {
196	                what: name.to_string(),
197	            });
198	        };
199	        next_generation(current, expected)?;
200	        let change = PaneEvent {
201	            cursor: table.next_cursor()?,
202	            name: name.clone(),
203	            pane: None,
204	        };
205	        self.commit(table, change)
206	    }
207	
208	    /// Save the table with `change` applied, then apply it to memory and wake the watchers.
209	    fn commit(&self, table: &mut Table, change: PaneEvent) -> Result<(), PaneError> {
210	        self.save(table, &change)?;
211	        table.head = change.cursor;
212	        table.ring.push(change.clone());
213	        table.entries.insert(change.name.clone(), change);
214	        self.changed.notify_all();
215	        Ok(())
216	    }
217	
218	    /// Write the document that `table` with `change` applied describes. A failure is
219	    /// logged and answered `unavailable`; the file is as it was.
220	    fn save(&self, table: &Table, change: &PaneEvent) -> Result<(), PaneError> {
221	        let doc = Doc {
222	            version: persist::VERSION,
223	            cursor: change.cursor,
224	            entries: table.entries_with(change),
225	        };
226	        persist::save_doc(&self.path, &doc).map_err(|problem| {
227	            log_fault(
228	                "pane_registry_write_failed",
229	                &self.path,
230	                &problem,
231	                WRITE_EFFECT,
232	            );
233	            PaneError::Unavailable {
234	                what: problem.what(LABEL, &self.path),
235	            }
236	        })
237	    }
~~~~

**E10: `crates/holler-hub/src/panes/store.rs:338-359`** — `refuse_profile_move`: compares by slug, runs inside `cas_put` under the pane lock.

Command: `sed -n '338,359p' crates/holler-hub/src/panes/store.rs | awk -v s=338 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
338	/// The membership rule of a write (#661; ADR-0021 §8 and "Decisions taken", item 2): a
339	/// pane stored in one profile cannot be written into another, `pane-in-other-profile`.
340	/// Leaving (`None`), joining from `None` and keeping the profile are allowed, so a move
341	/// takes two writes, leave and then join. Profiles are compared by slug, the profile
342	/// registry's identity, so another spelling of the same profile keeps it. Whether the
343	/// profile exists is the `pane/cas_put` hook's check (`crate::profile::check_membership`).
344	fn refuse_profile_move(stored: Option<&Pane>, next: &Pane) -> Result<(), PaneError> {
345	    let current = stored.and_then(|pane| pane.profile.as_ref());
346	    match (current, next.profile.as_ref()) {
347	        (Some(current), Some(other)) if current.slug() != other.slug() => {
348	            Err(PaneError::PaneInOtherProfile {
349	                what: format!(
350	                    "{} is in profile {:?}, not {:?}",
351	                    next.name,
352	                    current.as_str(),
353	                    other.as_str()
354	                ),
355	            })
356	        }
357	        _ => Ok(()),
358	    }
359	}
~~~~

**E11: `crates/holler-hub/src/panes/mod.rs:105-124`** — `PaneState` holds its store behind a private `Arc<Store>`; `Store` is private to `panes`, so `profile/` cannot reach it today.

Command: `sed -n '105,124p' crates/holler-hub/src/panes/mod.rs | awk -v s=105 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
105	/// The pane registry's state, shared by every control connection behind one `Arc`
106	/// ([`crate::pane_dispatch::PaneDeps`]).
107	///
108	/// It does not derive `Clone`. A copy per connection would split the registry, and a
109	/// split registry cannot make a compare-and-swap hold: share the `Arc`, never the
110	/// value.
111	pub struct PaneState {
112	    store: Arc<Store>,
113	}
114	
115	impl PaneState {
116	    /// Load the registry from `<state dir>/hub/panes.json`, once, before the first
117	    /// connection. It never fails: a missing file is an empty registry, and a file that
118	    /// cannot be loaded leaves the registry failed. Every method then answers
119	    /// `store-corrupt`, and the file is never rewritten (see `persist.rs` and `store.rs`).
120	    pub fn load(state: &HubState) -> Self {
121	        Self::load_with(state, PaneStoreOptions::default())
122	    }
123	
124	    /// [`PaneState::load`] with `options` in place of the defaults.
~~~~

**E12: `crates/holler-hub/src/panes/mod.rs:186-213`** — `panes::dispatch`: the pane side gets the profile registry for the membership check.

Command: `sed -n '186,213p' crates/holler-hub/src/panes/mod.rs | awk -v s=186 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
186	/// Answer one `pane/*` request with its handler. `obj` is the whole request frame.
187	///
188	/// `profiles` is the profile registry. `pane/cas_put` calls
189	/// [`crate::profile::check_membership`] with it before the compare-and-swap, so a refusal
190	/// is the reply and nothing is written. The parameter is an `&Arc`, so the handler can move
191	/// a handle into the blocking pool, where the check runs. #661's check reads the profile
192	/// registry through its blocking port, which must never run on an executor thread.
193	/// `pane/watch` is long-poll: one `{events, cursor}` reply per request, after at most
194	/// [`WATCH_WAIT`].
195	pub async fn dispatch(
196	    method: &str,
197	    cid: &CorrelationId,
198	    obj: &Value,
199	    panes: &PaneState,
200	    profiles: &Arc<ProfileState>,
201	) -> String {
202	    let store = Arc::clone(&panes.store);
203	    match method {
204	        "pane/get" => handlers::get(cid, obj, store).await,
205	        "pane/list" => handlers::list(cid, obj, store).await,
206	        "pane/cas_put" => handlers::cas_put(cid, obj, store, Arc::clone(profiles)).await,
207	        "pane/delete" => handlers::delete(cid, obj, store).await,
208	        "pane/watch" => handlers::watch(cid, obj, store).await,
209	        // `forward` sends only the methods of `PANE_METHODS`, and each has an arm above.
210	        // A method added to that list before its handler answers like a stub.
211	        _ => reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented)),
212	    }
213	}
~~~~

**E13: `crates/holler-hub/src/pane_dispatch.rs:66-91`** — `forward`: the profile dispatcher already receives both handles.

Command: `sed -n '66,91p' crates/holler-hub/src/pane_dispatch.rs | awk -v s=66 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
66	}
67	
68	/// Answer one `pane/*` or `profile/*` request: route `method` to its registry's
69	/// dispatcher, giving it both state handles.
70	///
71	/// The `Arc` fields are passed as they are. They deref-coerce to the `&PaneState` and
72	/// `&ProfileState` of today's dispatchers, and a story whose handler must move an
73	/// owned handle into `spawn_blocking` can widen its own `dispatch` to `&Arc<_>`
74	/// without editing this file.
75	pub(crate) async fn forward(
76	    method: &str,
77	    cid: &CorrelationId,
78	    obj: &Value,
79	    deps: &PaneDeps,
80	) -> String {
81	    if is_pane_method(method) {
82	        panes::dispatch(method, cid, obj, &deps.panes, &deps.profiles).await
83	    } else if is_profile_method(method) {
84	        profile::dispatch(method, cid, obj, &deps.profiles, &deps.panes).await
85	    } else {
86	        // The caller's arm only forwards a method in one of the lists, so this is not
87	        // reached. It answers like every other unknown method rather than panicking.
88	        encode_error(
89	            cid,
90	            Code::MethodNotFound,
91	            format!("unknown control method: {method}"),
~~~~

**E14: `crates/holler-hub/tests/pane_dispatch_test.rs:155-180`** — A hub test that pins `profile/rename` as `not-implemented`: it changes with this story.

Command: `sed -n '155,180p' crates/holler-hub/tests/pane_dispatch_test.rs | awk -v s=155 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
155	}
156	
157	/// What each profile method answers to a request that carries no `params`, now that the
158	/// registry is real: a decode error, an empty list, or an empty batch. Only
159	/// `profile/rename` (#665) is still `not-implemented`. Never `method_not_found`.
160	fn assert_answers_an_empty_profile_request(
161	    method: &str,
162	    outcome: Result<Option<Value>, PaneError>,
163	) {
164	    match method {
165	        "profile/list" => assert_eq!(outcome, Ok(Some(json!([]))), "{method}"),
166	        "profile/watch" => assert_eq!(
167	            outcome,
168	            Ok(Some(json!({"events": [], "cursor": 0}))),
169	            "{method}"
170	        ),
171	        "profile/get" | "profile/cas_put" | "profile/delete" | "profile/log" => assert!(
172	            matches!(outcome, Err(PaneError::Usage { .. })),
173	            "{method} without params must be a usage error, got {outcome:?}"
174	        ),
175	        "profile/rename" => assert_eq!(outcome, Err(PaneError::NotImplemented), "{method}"),
176	        other => panic!("{other} is a new profile method: add its expectation here"),
177	    }
178	}
179	
180	#[tokio::test]
~~~~

**E15: `crates/holler-hub/tests/profile_handlers_test.rs:472-478`** — The second pin of `not-implemented` (a corrupt registry).

Command: `sed -n '472,478p' crates/holler-hub/tests/profile_handlers_test.rs | awk -v s=472 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
472	    );
473	    assert_eq!(dir_listing(&rig.state.hub_dir), listing);
474	
475	    // profile/rename is #665's: it still answers not-implemented, corrupt registry or not.
476	    let rename = rig.call("profile/rename", Some(json!({}))).await;
477	    assert_eq!(rename, Err(PaneError::NotImplemented));
478	}
~~~~

**E16: `crates/holler-hub/tests/pane_dispatch_test.rs:325-345`** — The third pin: `profile::dispatch` called directly with both handles.

Command: `sed -n '325,345p' crates/holler-hub/tests/pane_dispatch_test.rs | awk -v s=325 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
325	#[tokio::test]
326	async fn profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub() {
327	    let (deps, _dir) = fresh_deps();
328	    let cid = CorrelationId::parse("h-direct-2").unwrap();
329	    // The handles arrive in the opposite order to `panes::dispatch`: the profile
330	    // store first, the pane store second (`profile/rename` reaches the panes). With no
331	    // params `profile/get` is a usage error from the real registry; `profile/rename` is
332	    // still #665's stub.
333	    for method in ["profile/get", "profile/rename"] {
334	        let line = profile::dispatch(method, &cid, &json!({}), &deps.profiles, &deps.panes).await;
335	        let reply: Value = serde_json::from_str(&line).unwrap();
336	        let outcome = pane_outcome(&reply, method);
337	        match method {
338	            "profile/get" => assert!(
339	                matches!(outcome, Err(PaneError::Usage { .. })),
340	                "{method}: {reply}"
341	            ),
342	            _ => assert_eq!(outcome, Err(PaneError::NotImplemented), "{method}: {reply}"),
343	        }
344	    }
345	}
~~~~

**E17: `crates/holler-pane/src/profile.rs:279-321`** — `ProfileChange` (`Renamed { from }` exists), `ProfileLogEntry` and `ProfileEvent` ("a rename is a deletion of the old name followed by a put of the new one").

Command: `sed -n '279,321p' crates/holler-pane/src/profile.rs | awk -v s=279 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
279	/// What one write did to a profile, as the log records it.
280	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
281	#[serde(rename_all = "snake_case", deny_unknown_fields)]
282	pub enum ProfileChange {
283	    Created,
284	    /// The spec set changed; `summary` says how, in one line.
285	    Updated {
286	        summary: String,
287	    },
288	    /// The profile was renamed from this name.
289	    Renamed {
290	        from: ProfileName,
291	    },
292	    Deleted,
293	}
294	
295	/// One entry of a profile's append-only change log: who, when, the new generation and
296	/// what changed.
297	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
298	#[serde(deny_unknown_fields)]
299	pub struct ProfileLogEntry {
300	    /// When the write happened (milliseconds since the Unix epoch).
301	    pub at: i64,
302	    /// The generation the profile had after the write.
303	    pub generation: u64,
304	    pub actor: Actor,
305	    pub change: ProfileChange,
306	}
307	
308	/// One change to the profile store, as `ProfileStore::watch` yields it. A rename
309	/// is a deletion of the old name followed by a put of the new one.
310	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
311	#[serde(deny_unknown_fields)]
312	pub struct ProfileEvent {
313	    /// The store-wide sequence number of this change; hand it back as the `since` of
314	    /// the next watch to resume without a gap or a repeat.
315	    pub cursor: Cursor,
316	    /// The profile the change concerns.
317	    pub name: ProfileName,
318	    /// The record after the change; `None` when the profile was deleted.
319	    #[serde(default)]
320	    pub profile: Option<Box<Profile>>,
321	}
~~~~

**E18: `crates/holler-pane/src/profile.rs:323-375`** — The `ProfileStore` port, with `rename(from, to, expected_generation, actor)` declared and marked PROPOSED.

Command: `sed -n '323,375p' crates/holler-pane/src/profile.rs | awk -v s=323 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
323	/// The registry of profiles, kept by the hub (#661) and faked by the test kit (#638).
324	///
325	/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
326	/// thread) in async code. Every method returns within I5's bound (default 10 s) or
327	/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
328	///
329	/// **Writes** are compare-and-swap on the profile's `generation` (see
330	/// [`crate::generation`]) and carry the [`Actor`] that made them; each appends one
331	/// [`ProfileLogEntry`]. A stale generation is `generation-conflict`.
332	pub trait ProfileStore: Send + Sync {
333	    /// The profile named `name`, or `None`.
334	    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError>;
335	
336	    /// Every profile.
337	    fn list(&self) -> Result<Vec<Profile>, PaneError>;
338	
339	    /// Store `profile` if the stored one is still at `expected_generation` (0 for a
340	    /// new profile); returns the stored record with its bumped generation.
341	    fn cas_put(
342	        &self,
343	        profile: &Profile,
344	        expected_generation: u64,
345	        actor: &Actor,
346	    ) -> Result<Profile, PaneError>;
347	
348	    /// Delete the profile `name` if it is still at `expected_generation`. A stale
349	    /// generation is `generation-conflict`; a profile that does not exist is
350	    /// `profile-not-found`, whatever `expected_generation` is (a missing profile is
351	    /// checked first, so no store has to guess which of the two to answer).
352	    fn delete(
353	        &self,
354	        name: &ProfileName,
355	        expected_generation: u64,
356	        actor: &Actor,
357	    ) -> Result<(), PaneError>;
358	
359	    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
360	    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError>;
361	
362	    /// The change log of the profile `name`, oldest first.
363	    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError>;
364	
365	    /// PROPOSED (#665): rename `from` to `to` if `from` is still at
366	    /// `expected_generation`. Until #665 is confirmed an implementation answers
367	    /// `not-implemented`.
368	    fn rename(
369	        &self,
370	        from: &ProfileName,
371	        to: &ProfileName,
372	        expected_generation: u64,
373	        actor: &Actor,
374	    ) -> Result<Profile, PaneError>;
375	}
~~~~

**E19: `crates/holler-pane/src/reply.rs:192-200`** — `ProfileRenameParams`, the params of `profile/rename`.

Command: `sed -n '192,200p' crates/holler-pane/src/reply.rs | awk -v s=192 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
192	/// Params of `profile/rename` (PROPOSED, #665).
193	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
194	#[serde(deny_unknown_fields)]
195	pub struct ProfileRenameParams {
196	    pub from: ProfileName,
197	    pub to: ProfileName,
198	    pub expected_generation: u64,
199	    pub actor: Actor,
200	}
~~~~

**E20: `crates/holler-pane/src/reply.rs:110-117`** — `decode_params`: the public decode that maps a guard's coded serde error back to its closed code.

Command: `sed -n '110,117p' crates/holler-pane/src/reply.rs | awk -v s=110 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
110	///
111	/// A request that carries no `params` decodes from `{}`, as
112	/// `holler_proto::typed_params` does: the caller passes an empty JSON object for it,
113	/// so `pane/list`, `profile/list` and the `*/watch` methods (whose `since` defaults
114	/// to `Cursor(0)`) accept a request without `params`.
115	pub fn decode_params<T: DeserializeOwned>(params: Value) -> Result<T, PaneError> {
116	    serde_json::from_value(params).map_err(|e| PaneError::from_decode(&e))
117	}
~~~~

**E21: `crates/holler-pane/src/error.rs:33-66`** — The closed codes: `profile-secret-refused`, `profile-exists`, `command-not-argv`, `profile-not-found` and `profile-drift` all exist.

Command: `sed -n '33,66p' crates/holler-pane/src/error.rs | awk -v s=33 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
33	use std::fmt;
34	
35	use serde::de::{self, Deserializer};
36	use serde::Deserialize;
37	
38	/// The number of closed codes.
39	const CODE_COUNT: usize = 22;
40	
41	/// The closed set of codes as a plain enum: the one table that
42	/// [`ALL_CODES`], [`PaneError::code`] and the wire parse-back all derive from, so
43	/// the three cannot drift (the same single-source rule as `holler_proto::Code`).
44	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
45	pub(crate) enum PaneCode {
46	    NotImplemented,
47	    Usage,
48	    GridAmbiguous,
49	    GridOutOfRange,
50	    CommandNotArgv,
51	    EnvNameInvalid,
52	    GenerationConflict,
53	    ProbeFailed,
54	    ProfileConflict,
55	    ProfileNotFound,
56	    ProfileExists,
57	    ProfileHasLivePanes,
58	    PaneNotInProfile,
59	    PaneInOtherProfile,
60	    ProfileSecretRefused,
61	    HerdrVersionUnsupported,
62	    Timeout,
63	    PaneNotFound,
64	    SessionNotFound,
65	    StoreCorrupt,
66	    Unavailable,
~~~~

**E22: `crates/holler-pane/src/error.rs:142-148`** — `ALL_CODES`: no story edits the list; `profile-drift` is listed for convenience.

Command: `sed -n '142,148p' crates/holler-pane/src/error.rs | awk -v s=142 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
142	/// The set is closed: the invariants of this crate (one variant per code, unique,
143	/// valid under [`is_valid_code`]) are tested against it, and no story edits it.
144	/// `profile-drift` is listed for convenience: it is a reconcile finding kind
145	/// (#647/#665 put it in `findings.rs`), not an error a port returns.
146	pub const ALL_CODES: &[&str] = &CLOSED_CODES;
147	
148	const CLOSED_CODES: [&str; CODE_COUNT] = {
~~~~

**E23: `crates/holler-pane/src/argv.rs:162-197`** — `deserialize_env_names`: the typed refusal of a value (`profile-secret-refused`), so import adds no second scan.

Command: `sed -n '162,197p' crates/holler-pane/src/argv.rs | awk -v s=162 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
162	    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
163	        deserialize_parsed(deserializer, AgentKey::parse)
164	    }
165	}
166	
167	/// Read an `env` field: a JSON array of [`EnvVarName`]s.
168	///
169	/// Used by `Pane.env` and `ProfileSpec.env` (beside `#[serde(default)]`) instead of
170	/// the derived `Vec` impl, whose type error would repeat the value it was given. The
171	/// field is read as a JSON value first, as [`Argv`] is, so no refusal echoes input:
172	///
173	/// - an array: each element goes through [`EnvVarName::parse`], codes unchanged (a
174	///   non-string element is `env-name-invalid`);
175	/// - a bare string (the shape a shell `NAME=value` has): `profile-secret-refused` if
176	///   it contains `=`, else `env-name-invalid`;
177	/// - any other shape: `env-name-invalid`.
178	pub(crate) fn deserialize_env_names<'de, D: Deserializer<'de>>(
179	    deserializer: D,
180	) -> Result<Vec<EnvVarName>, D::Error> {
181	    let refuse = |e: PaneError| de::Error::custom(e.coded_message());
182	    match Value::deserialize(deserializer)? {
183	        Value::Array(items) => items
184	            .into_iter()
185	            .map(|item| match item {
186	                Value::String(name) => EnvVarName::parse(&name),
187	                _ => Err(PaneError::EnvNameInvalid),
188	            })
189	            .collect::<Result<Vec<_>, _>>()
190	            .map_err(refuse),
191	        // A bare string with a `=` is a `NAME=value` entry that lost its list.
192	        Value::String(text) if text.contains('=') => Err(refuse(PaneError::ProfileSecretRefused)),
193	        // Anything else is not a list of names: a bare string without a `=` (the
194	        // wrong shape even when it is a valid name), a map, a number, `null`.
195	        _ => Err(refuse(PaneError::EnvNameInvalid)),
196	    }
197	}
~~~~

**E24: `crates/holler-pane/src/grid.rs:1-14`** — `GridPos`: the accepted input forms and the output forms.

Command: `sed -n '1,14p' crates/holler-pane/src/grid.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! `GridPos`: the one grid-position type of pane control (epic #633, decision 7).
2	//!
3	//! Grid notation is ROWCOL and 1-based. The type, its parser, the `rRcC` display
4	//! and the serde form are defined here once; the Herdr adapter (#640) is the only
5	//! place that converts a `GridPos` to Herdr's own order and base.
6	//!
7	//! - **Input** ([`GridPos::parse`]): `r<row>c<col>` or `c<col>r<row>` (labelled, in
8	//!   either order) or a bare `<row>,<col>`, which is always row first and never
9	//!   col,row. Upper case and surrounding ASCII whitespace are accepted; interior
10	//!   whitespace is refused. A missing half, a repeated label, a mix such as `r2c1c3`
11	//!   or anything unreadable is `grid-ambiguous`; a zero, or a number above
12	//!   `u16::MAX`, is `grid-out-of-range`.
13	//! - **Output**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first.
14	
~~~~

**E25: `crates/holler-pane/src/grid.rs:127-170`** — `GridPos`'s serde: it writes `{row, col, pos}` and reads only that object, never the string `r2c1`; so a file's string goes through `GridPos::parse`.

Command: `sed -n '127,170p' crates/holler-pane/src/grid.rs | awk -v s=127 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
127	}
128	
129	impl Serialize for GridPos {
130	    /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
131	    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
132	        let mut out = serializer.serialize_struct("GridPos", 3)?;
133	        out.serialize_field("row", &self.row)?;
134	        out.serialize_field("col", &self.col)?;
135	        out.serialize_field("pos", &self.to_string())?;
136	        out.end()
137	    }
138	}
139	
140	/// The serde form before it is checked.
141	#[derive(Deserialize)]
142	#[serde(deny_unknown_fields)]
143	struct RawGridPos {
144	    row: u16,
145	    col: u16,
146	    /// Derived from `row` and `col`; optional on input, and refused when it
147	    /// disagrees (a stale or transposed label must not be trusted silently).
148	    #[serde(default)]
149	    pos: Option<String>,
150	}
151	
152	impl<'de> Deserialize<'de> for GridPos {
153	    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
154	        let raw = RawGridPos::deserialize(deserializer)?;
155	        let cell = GridPos::from_cells(raw.row, raw.col)
156	            .map_err(|e| de::Error::custom(e.coded_message()))?;
157	        if let Some(label) = raw.pos {
158	            if GridPos::parse(&label).ok() != Some(cell) {
159	                let err = PaneError::GridAmbiguous {
160	                    what: format!(
161	                        "pos {} disagrees with row {} and column {}",
162	                        excerpt(&label),
163	                        raw.row,
164	                        raw.col
165	                    ),
166	                };
167	                return Err(de::Error::custom(err.coded_message()));
168	            }
169	        }
170	        Ok(cell)
~~~~

**E26: `crates/holler-cli/src/profile/create.rs:200-221`** — `insert_profile`, written for reuse by a caller outside `create`: the create write and the race mapping to `profile-exists`.

Command: `sed -n '200,221p' crates/holler-cli/src/profile/create.rs | awk -v s=200 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
200	/// Store `profile` as a new record (expected generation 0) and return it as stored: the one
201	/// create write of every form of `profile create` (#662 B7), so the race mapping exists once.
202	///
203	/// A `generation-conflict` means another writer created the name between the caller's read and
204	/// this write (the hub's store answers `next_generation(1, 0)`): to the operator that is a
205	/// taken name, `profile-exists`. Every other error passes through, the store's own
206	/// `profile-exists` (the slug is filed under another spelling) included. It takes the port and
207	/// answers a `PaneError`, with no CLI type, so a caller outside this verb can use it.
208	pub(crate) fn insert_profile(
209	    store: &dyn ProfileStore,
210	    profile: &Profile,
211	    actor: &Actor,
212	) -> Result<Profile, PaneError> {
213	    store
214	        .cas_put(profile, 0, actor)
215	        .map_err(|error| match error {
216	            PaneError::Conflict => taken(&profile.name),
217	            other => other,
218	        })
219	}
220	
221	/// `profile-exists` for `name`: `"<name>" (slug <slug>)`.
~~~~

**E27: `crates/holler-cli/src/profile/delete.rs:1-20`** — The ADR's "no transaction spans the two registries" in practice: `delete --keep-panes` writes panes, then the profile.

Command: `sed -n '1,20p' crates/holler-cli/src/profile/delete.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! `holler profile delete NAME [--keep-panes]` (story #662, epic #633): delete a profile, refused
2	//! while any pane belongs to it; with `--keep-panes`, each such pane is detached first and keeps
3	//! running.
4	//!
5	//! A profile's live panes are its members, the pane records whose `profile` has its slug
6	//! (`holler_pane::profile_diff::is_member`, ADR-0021 section 3). No transaction spans the two
7	//! registries (ADR-0021 section 8), so `--keep-panes` writes in the order that never leaves a
8	//! pane naming a profile that is gone: each member leaves first (its `Pane.profile` becomes none
9	//! by compare-and-swap), then the profile is deleted. A failed detach stops the verb and nothing
10	//! is re-attached: detaching is what was asked for, running the delete again converges, and a
11	//! re-attach could fail too (#662 Decision 8).
12	//!
13	//! The verb reads and writes the two registries and nothing else: no adapter, no probe, and
14	//! nothing from the process environment.
15	//!
16	//! `detach`, `single_quoted` and `pane_list` are shared with `create` (as
17	//! `super::delete::...`), because the frozen `profile/mod.rs` admits no new module.
18	
19	use clap::Args;
20	use holler_pane::profile_diff::is_member;
~~~~

**E28: `crates/holler-pane/src/findings.rs:1-26`** — The findings module doc: #665 adds only the `ProfileDrift` kind; "no remedy carries ... a profile".

Command: `sed -n '1,26p' crates/holler-pane/src/findings.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! The findings of a reconcile pass and `holler pane doctor` (#647): what a pass found, the
2	//! one table of what each kind tells the operator to run, and the one sanitizer for the
3	//! untrusted text a finding quotes. #665 adds only the `ProfileDrift` kind after #647 has
4	//! merged.
5	//!
6	//! - [`FindingKind`]: twelve kinds, each with a stable kebab-case code that is also its JSON
7	//!   form ([`FindingKind::code`] is the one source of both). A finding is a report, not an
8	//!   error: its code is never a `RefusalCode`, and two codes (`herdr-version-unsupported`,
9	//!   and #665's `profile-drift`) equal closed error codes.
10	//! - [`Finding`]: one finding, with what it is about (a pane, a position, a Herdr pane, a
11	//!   session, harness servers), a one-line message, its remedy and what `--fix` did.
12	//! - **Remedies** ([`FindingKind::remedy`]) name only the verb that owns each repair
13	//!   (`pane relaunch`, `pane reset`, `pane doctor`), built from constant words and a
14	//!   [`PaneName`], whose grammar has no space, shell metacharacter or leading `-`. No remedy
15	//!   carries a session id, a Herdr id, a port, a profile or any adapter text, and none is a
16	//!   raw OpenCode, Herdr or tmux command: only Holler reaches those (epic #633). A finding no
17	//!   Holler verb repairs (a stray session, an unregistered Herdr pane, an unsupported Herdr)
18	//!   has no remedy, and its message says why.
19	//! - **Untrusted text** (session and Herdr ids, the record's host names, adapter messages)
20	//!   reaches a message only through [`quoted`] (one value) or `embedded` (an adapter's
21	//!   message), so no control sequence reaches a terminal.
22	//!
23	//! **Serde.** A report is output and never read back, so unlike the records (`lib.rs`) it
24	//! leaves nothing out: every absent value is `null`, and the JSON shape of `holler pane doctor`
25	//! is fixed (it is part of ADR 0003's versioned `--json` surface).
26	
~~~~

**E29: `crates/holler-pane/src/findings.rs:78-150`** — `FindingKind::ALL`, `code` and the one `remedy` table (an exhaustive `match`).

Command: `sed -n '78,150p' crates/holler-pane/src/findings.rs | awk -v s=78 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
78	impl FindingKind {
79	    /// Every kind, in declaration order.
80	    pub const ALL: &'static [FindingKind] = &[
81	        FindingKind::HerdrVersionUnsupported,
82	        FindingKind::ObserveFailed,
83	        FindingKind::UnregisteredHerdrPane,
84	        FindingKind::HerdrPaneMissing,
85	        FindingKind::TmuxSessionMissing,
86	        FindingKind::ServerWedged,
87	        FindingKind::ServerDown,
88	        FindingKind::NoSessionOfRecord,
89	        FindingKind::SessionOfRecordMissing,
90	        FindingKind::TuiForeignSession,
91	        FindingKind::ShownDrivenMismatch,
92	        FindingKind::StraySession,
93	        FindingKind::UnmanagedStalled,
94	    ];
95	
96	    /// The kind's stable kebab-case code, which is also its JSON form. A code is stable
97	    /// once merged (ADR-0021 section 9).
98	    pub const fn code(self) -> &'static str {
99	        match self {
100	            FindingKind::HerdrVersionUnsupported => "herdr-version-unsupported",
101	            FindingKind::ObserveFailed => "observe-failed",
102	            FindingKind::UnregisteredHerdrPane => "unregistered-herdr-pane",
103	            FindingKind::HerdrPaneMissing => "herdr-pane-missing",
104	            FindingKind::TmuxSessionMissing => "tmux-session-missing",
105	            FindingKind::ServerWedged => "server-wedged",
106	            FindingKind::ServerDown => "server-down",
107	            FindingKind::NoSessionOfRecord => "no-session-of-record",
108	            FindingKind::SessionOfRecordMissing => "session-of-record-missing",
109	            FindingKind::TuiForeignSession => "tui-foreign-session",
110	            FindingKind::ShownDrivenMismatch => "shown-driven-mismatch",
111	            FindingKind::StraySession => "stray-session",
112	            FindingKind::UnmanagedStalled => "unmanaged-stalled",
113	        }
114	    }
115	
116	    /// The command a finding of this kind tells the operator to run, for the pane `pane`
117	    /// (`None` when it is about no pane) once `--fix` left it at `fix`; `None` when no Holler
118	    /// verb repairs it. The one table of remedies:
119	    ///
120	    /// - a read that failed: run the pass again, `holler pane doctor [<pane>]`;
121	    /// - the pane's process or its harness is gone or wrong: `holler pane relaunch <pane>`;
122	    /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
123	    ///   one (choosing an existing session is the operator's, never a guess);
124	    /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
125	    ///   it cannot or failed, nothing once it is fixed.
126	    pub fn remedy(self, pane: Option<&PaneName>, fix: FixState) -> Option<String> {
127	        match self {
128	            FindingKind::HerdrVersionUnsupported
129	            | FindingKind::UnregisteredHerdrPane
130	            | FindingKind::UnmanagedStalled
131	            | FindingKind::StraySession => None,
132	            FindingKind::ObserveFailed => Some(doctor_command(pane, false)),
133	            FindingKind::HerdrPaneMissing
134	            | FindingKind::TmuxSessionMissing
135	            | FindingKind::ServerWedged
136	            | FindingKind::ServerDown
137	            | FindingKind::TuiForeignSession => pane.map(relaunch_command),
138	            FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
139	                pane.map(reset_command)
140	            }
141	            FindingKind::ShownDrivenMismatch => match fix {
142	                FixState::Fixable | FixState::Skipped => {
143	                    pane.map(|pane| doctor_command(Some(pane), true))
144	                }
145	                FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
146	                FixState::Fixed => None,
147	            },
148	        }
149	    }
150	}
~~~~

**E30: `crates/holler-pane/src/reconcile.rs:36-44`** — "Once per episode is once per pass: a pass reports every current finding exactly once and keeps no memory between passes."

Command: `sed -n '36,44p' crates/holler-pane/src/reconcile.rs | awk -v s=36 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
36	//! - A record is written only when one of those values changed, or on its first
37	//!   observation (`at == 0`): the hub's rule for a periodic writer (`panes/mod.rs`). So
38	//!   `at` is when the current values were first observed, not a freshness stamp. A write
39	//!   that loses a race with another writer is reported and never retried: that writer's
40	//!   record wins.
41	//!
42	//! "Once per episode" is once per pass: a pass reports every current finding exactly once
43	//! and keeps no memory between passes.
44	
~~~~

**E31: `crates/holler-pane/src/reconcile.rs:188-256`** — `reconcile` and `resolve`: the scope, the records and where the findings of a pass are joined.

Command: `sed -n '188,256p' crates/holler-pane/src/reconcile.rs | awk -v s=188 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
188	/// Run one reconcile pass over `ports` (see the module docs).
189	///
190	/// It fails only when it cannot run the pass: a named pane or profile that does not exist
191	/// or a pane outside the profile (`pane-not-found`, `profile-not-found`,
192	/// `pane-not-in-profile`), or a pane store or profile scope that cannot be read. Everything
193	/// else the pass observes, a failed call included, is a finding of the report.
194	pub fn reconcile(ports: Ports<'_>, request: &ReconcileRequest<'_>) -> Result<Report, PaneError> {
195	    let (scope, records) = resolve(ports, request)?;
196	    let (herdr, checked) = observe_all(ports, request, &scope);
197	    let whole_fleet = request.profile.is_none() && request.pane.is_none();
198	    let mut findings = herdr_findings(herdr.as_ref(), &scope, &records, whole_fleet);
199	    findings.extend(strays(&scope, &checked, &records));
200	    if let (true, Some(after), Some(Ok(snapshot))) = (
201	        whole_fleet,
202	        request.unmanaged_stalled_after_secs,
203	        herdr.as_ref().map(|seen| &seen.snapshot),
204	    ) {
205	        let unmanaged = unmanaged::from_snapshot(ports, snapshot, &records, request.now_ms);
206	        findings.extend(unmanaged::stalled_findings(&unmanaged, after));
207	    }
208	    let mut panes = Vec::with_capacity(checked.len());
209	    for pane in checked {
210	        findings.extend(pane.findings);
211	        panes.push(pane.summary);
212	    }
213	    panes.sort_by(|a, b| a.name.cmp(&b.name));
214	    findings.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
215	    // A finding observed twice (Herdr listing one pane twice) is reported once.
216	    findings.dedup();
217	    Ok(Report {
218	        scope: ScopeSummary {
219	            profile: request.profile.cloned(),
220	            pane: request.pane.cloned(),
221	        },
222	        fix_requested: request.fix,
223	        herdr: HerdrSummary {
224	            version: herdr.and_then(|seen| seen.version.ok()),
225	        },
226	        hosts: hosts(&scope),
227	        panes,
228	        findings,
229	    })
230	}
231	
232	/// The panes in scope, and every record (the stray rule judges against all of them). The
233	/// membership refusals are `ProfileScope::resolve`'s, and a named pane with no record is
234	/// `pane-not-found`.
235	fn resolve(
236	    ports: Ports<'_>,
237	    request: &ReconcileRequest<'_>,
238	) -> Result<(Vec<Pane>, Vec<Pane>), PaneError> {
239	    let scope = match (request.profile, request.pane) {
240	        (Some(profile), pane) => ports.scope.resolve(profile, pane)?.panes,
241	        (None, Some(name)) => {
242	            let pane = ports
243	                .pane_store
244	                .get(name)?
245	                .ok_or_else(|| PaneError::PaneNotFound {
246	                    what: name.to_string(),
247	                })?;
248	            vec![pane]
249	        }
250	        (None, None) => {
251	            let records = ports.pane_store.list()?;
252	            return Ok((records.clone(), records));
253	        }
254	    };
255	    Ok((scope, ports.pane_store.list()?))
256	}
~~~~

**E32: `crates/holler-pane/src/profile_diff.rs:268-323`** — `is_member`, `diff_spec` and `diff_profile`: the one comparison `profile show`, `profile apply` and `profile-drift` share.

Command: `sed -n '268,323p' crates/holler-pane/src/profile_diff.rs | awk -v s=268 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
268	/// Whether `pane` is a member of `profile`: its `profile` has `profile.slug()`. Slugs are
269	/// compared on both sides, since the slug is a profile's identity (a pane whose profile
270	/// is `SOME-PROFILE` is a member of `Some Profile`), like the test kit's fakes do.
271	///
272	/// This is what "a live pane of a profile" means (ADR-0021 section 3): `profile list`'s
273	/// live count, `profile show`'s comparison and `profile delete`'s refusal
274	/// `profile-has-live-panes` all use it.
275	pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
276	    pane.profile
277	        .as_ref()
278	        .is_some_and(|own| own.slug() == profile.slug())
279	}
280	
281	/// Every field where `live` differs from `spec`, in `SpecField::ALL` order; empty when
282	/// they match. The names are not compared (the caller pairs them). `env` and `expect`
283	/// are compared as sets (sorted, duplicates dropped); every other field by equality.
284	/// Pure: it compares `spec` with `spec_from_pane(live)`, so
285	/// `diff_spec(&spec_from_pane(p), p)` is always empty.
286	pub fn diff_spec(spec: &ProfileSpec, live: &Pane) -> Vec<FieldDiff> {
287	    let observed = spec_from_pane(live);
288	    SpecField::ALL
289	        .into_iter()
290	        .filter_map(|field| {
291	            let (want, got) = (field.value(spec), field.value(&observed));
292	            field.differs(&want, &got).then_some(FieldDiff {
293	                field,
294	                spec: want,
295	                live: got,
296	            })
297	        })
298	        .collect()
299	}
300	
301	/// The comparison of `profile` with the panes in `live` (the caller decides which panes
302	/// count as live; `profile show` passes the profile's members). One row per spec, in the
303	/// profile's order (`Matches` or `Differs` against the pane of `live` with that name,
304	/// else `Missing`), then one `Extra` row per pane of `live` that no spec names, in the
305	/// order of `live`.
306	pub fn diff_profile(profile: &Profile, live: &[Pane]) -> Vec<PaneDiff> {
307	    let rows = profile.panes.iter().map(|spec| {
308	        match live.iter().find(|pane| pane.name.as_str() == spec.pane) {
309	            Some(pane) => compared(spec, pane),
310	            None => row(&spec.pane, PaneStatus::Missing),
311	        }
312	    });
313	    let extras = live
314	        .iter()
315	        .filter(|pane| {
316	            !profile
317	                .panes
318	                .iter()
319	                .any(|spec| spec.pane == pane.name.as_str())
320	        })
321	        .map(|pane| row(pane.name.as_str(), PaneStatus::Extra));
322	    rows.chain(extras).collect()
323	}
~~~~

**E33: `crates/holler-pane/src/ports.rs:412-423`** — `Ports`: `profile_store` is already in the bundle, so reconcile needs no new port or method.

Command: `sed -n '412,423p' crates/holler-pane/src/ports.rs | awk -v s=412 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
412	/// One `&dyn` of each port: what a verb holds. `Ports` is `Copy`, so it is passed
413	/// by value or by reference freely, and `Ports<'static>` is `Send + Sync`.
414	#[derive(Clone, Copy)]
415	pub struct Ports<'a> {
416	    pub pane_store: &'a dyn PaneStore,
417	    pub profile_store: &'a dyn ProfileStore,
418	    pub herdr: &'a dyn HerdrPort,
419	    pub host: &'a dyn HostPort,
420	    pub harness: &'a dyn HarnessPort,
421	    pub scope: &'a dyn ProfileScope,
422	    pub prober: &'a dyn Prober,
423	}
~~~~

**E34: `crates/holler-cli/src/profile/show.rs:78-107`** — `profile show`'s `view`: members by `is_member`, rows by `diff_profile`.

Command: `sed -n '78,107p' crates/holler-cli/src/profile/show.rs | awk -v s=78 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
78	/// The profile named in `args` and its comparison with its members. A bad name is
79	/// `usage`, a profile that does not exist is `profile-not-found`, and a store error passes
80	/// through with its own code.
81	fn view(args: &ProfileShow, ports: Ports<'_>) -> Result<ProfileView, PaneError> {
82	    let name = ProfileName::parse(&args.name)?;
83	    let profile = ports
84	        .profile_store
85	        .get(&name)?
86	        .ok_or_else(|| PaneError::ProfileNotFound {
87	            what: format!("{:?}", name.as_str()),
88	        })?;
89	    let members: Vec<Pane> = ports
90	        .pane_store
91	        .list()?
92	        .into_iter()
93	        .filter(|pane| is_member(pane, &profile.name))
94	        .collect();
95	    let comparison = diff_profile(&profile, &members)
96	        .into_iter()
97	        .map(|diff| ComparisonRow {
98	            probe: last_probe(&diff, &members),
99	            diff,
100	        })
101	        .collect();
102	    Ok(ProfileView {
103	        live: members.len(),
104	        profile,
105	        comparison,
106	    })
107	}
~~~~

**E35: `crates/holler-pane/tests/findings_test.rs:1-28`** — The test that pins the kinds in order and says "#665 appends `profile-drift` and edits this list".

Command: `sed -n '1,28p' crates/holler-pane/tests/findings_test.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
2	//! The finding vocabulary of reconcile and `holler pane doctor` (#647 AC 25): twelve kinds
3	//! with stable kebab-case codes, each equal to the kind's JSON form. A code is stable once
4	//! merged (ADR-0021 section 9), so the list is pinned in order; #665 appends
5	//! `profile-drift` and edits this list.
6	
7	use std::collections::BTreeSet;
8	
9	use holler_pane::error::is_valid_code;
10	use holler_pane::findings::FindingKind;
11	
12	const CODES: [&str; 13] = [
13	    "herdr-version-unsupported",
14	    "observe-failed",
15	    "unregistered-herdr-pane",
16	    "herdr-pane-missing",
17	    "tmux-session-missing",
18	    "server-wedged",
19	    "server-down",
20	    "no-session-of-record",
21	    "session-of-record-missing",
22	    "tui-foreign-session",
23	    "shown-driven-mismatch",
24	    "stray-session",
25	    "unmanaged-stalled",
26	];
27	
28	#[test]
~~~~

**E36: `crates/holler-cli/tests/pane_verbs/doctor/read_only.rs:94-100`** — The assertion that a pass makes no profile-store call, which a drift check changes.

Command: `sed -n '94,100p' crates/holler-cli/tests/pane_verbs/doctor/read_only.rs | awk -v s=94 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
94	        calls.panes.iter().all(|op| reads.contains(op)),
95	        "{:?}",
96	        calls.panes
97	    );
98	    assert!(calls.profiles.is_empty(), "{:?}", calls.profiles);
99	    assert_eq!(calls.probes, 0);
100	}
~~~~

**E37: `crates/holler-cli/tests/pane_verbs/doctor/rig.rs:411-434`** — `record_of` and `profiles_of`: the rig's profiles are `sample_profile`s, not derived from its records.

Command: `sed -n '411,434p' crates/holler-cli/tests/pane_verbs/doctor/rig.rs | awk -v s=411 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
411	fn record_of(seed: &Seed, live: &Live) -> Pane {
412	    let mut pane = sample_pane(seed.name).expect("a sample pane");
413	    pane.herdr = live.herdr.clone();
414	    pane.harness.port = live.port;
415	    pane.session_of_record = Some(live.session.clone());
416	    pane.role = seed.role;
417	    pane.profile = seed
418	        .profile
419	        .map(|p| ProfileName::parse(p).expect("a profile name"));
420	    pane
421	}
422	
423	fn profiles_of(seeds: &[Seed]) -> Vec<holler_pane::Profile> {
424	    let mut members: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
425	    for seed in seeds {
426	        if let Some(profile) = seed.profile {
427	            members.entry(profile).or_default().push(seed.name);
428	        }
429	    }
430	    members
431	        .iter()
432	        .map(|(name, panes)| sample_profile(name, panes).expect("a sample profile"))
433	        .collect()
434	}
~~~~

**E38: `crates/holler-pane-testkit/src/fixture.rs:92-122`** — `sample_spec`: port policy `fixed`, grid `r1c1`; a record's snapshot says `fixed:<port>`, so every rig profile would drift.

Command: `sed -n '92,122p' crates/holler-pane-testkit/src/fixture.rs | awk -v s=92 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
92	/// A valid, deterministic `ProfileSpec` for the pane named `pane`: workspace
93	/// `scratch`, grid `r1c1`, directory `/srv/demo`, the OpenCode harness with the port
94	/// policy `fixed`, the sample pane's model and context ceilings, an agent, the server's
95	/// default OpenCode agent (no `opencode_agent`), and no env, no command, no check and
96	/// no expect. Two calls with one name return equal specs.
97	///
98	/// `pane` is plain text and is not checked, because a spec may name a pane that has no
99	/// record (a detached spec).
100	pub fn sample_spec(pane: &str) -> ProfileSpec {
101	    ProfileSpec {
102	        pane: pane.to_owned(),
103	        herdr: SpecHerdr {
104	            workspace: SCRATCH.to_owned(),
105	            grid: SAMPLE_GRID,
106	        },
107	        host: SpecHost {
108	            cwd: SAMPLE_CWD.to_owned(),
109	        },
110	        harness: SpecHarness {
111	            kind: HarnessKind::Opencode,
112	            port_policy: SAMPLE_PORT_POLICY.to_owned(),
113	        },
114	        model: sample_model(),
115	        opencode_agent: None,
116	        role: PaneRole::Agent,
117	        env: Vec::new(),
118	        context: sample_context(),
119	        command: None,
120	        check: None,
121	        expect: Vec::new(),
122	    }
~~~~

**E39: `crates/holler-cli/src/pane/wiring/bind.rs:1-24`** — The wrappers (`BindingHerdr`, `BindingHost`, `LazyHerdr`, `Wired`) forward the Herdr, host and harness ports; the stores are not wrapped.

Command: `sed -n '1,24p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! The wiring's own port types (#649): what sits between the verbs and the adapters so that the
2	//! adapters can be built from configuration that may be wrong, and so that the OpenCode adapter's
3	//! one lookup can answer for a pane that is being launched.
4	//!
5	//! - [`LazyHerdr`] builds the Herdr adapter on its first use and keeps the result (D2): building
6	//!   it pings Herdr, so a verb that never reaches Herdr never needs one.
7	//! - [`Wired`] holds a port, or the configuration error that keeps it from being built, which
8	//!   each of its methods then answers (W-4, W-E): [`WiredHarness`] for OpenCode, [`WiredHost`]
9	//!   for tmux. A misconfigured port fails closed at its first use, never at `connect`, and is
10	//!   never replaced by another (the default tmux server, or `Unwired`).
11	//! - [`BindingHerdr`] and [`BindingHost`] forward every method, and bind the pane id that each
12	//!   `ensure_pane` returns to the name that the next `ensure_session` names (D8 (a)).
13	//! - [`tui_session`] is the lookup itself (D8).
14	//!
15	//! **Every method is forwarded (D31 W-4, D33).** Each wrapper here overrides every method of its
16	//! port, the ones the port gives a default included: `HarnessPort`'s `send_prompt` and
17	//! `session_activity`, and `HerdrPort`'s `observe_pane` and `observe_panes` (#832). A default
18	//! never answers in place of the inner port, and `observe_panes` goes to the inner port's own
19	//! `observe_panes`, so a list of panes stays one read of Herdr (ADR-0021 section 14), never one
20	//! `observe_pane` per pane.
21	//!
22	//! **Why the bindings (D8).** The launch engine attaches the TUI by the Herdr pane id before the
23	//! pane's record exists (E34): `ensure_pane` (which knows the pane id), then `ensure_session` and
24	//! `run` (which know the name), then the server, then `attach_tui(pane id)`, and only then the
~~~~

**E40: `crates/holler-cli/src/pane/wiring/store.rs:196-212`** — The CLI's `ProfileStore::rename`: it already sends `profile/rename`.

Command: `sed -n '196,212p' crates/holler-cli/src/pane/wiring/store.rs | awk -v s=196 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
196	    }
197	
198	    fn rename(
199	        &self,
200	        from: &ProfileName,
201	        to: &ProfileName,
202	        expected_generation: u64,
203	        actor: &Actor,
204	    ) -> Result<Profile, PaneError> {
205	        PROFILE_RENAME.request(&ProfileRenameParams {
206	            from: from.clone(),
207	            to: to.clone(),
208	            expected_generation,
209	            actor: actor.clone(),
210	        })
211	    }
212	}
~~~~

**E41: `crates/holler-cli/src/profile/rename.rs:1-22`** — The CLI stub of `profile rename` (`export.rs` and `import.rs` have the same shape).

Command: `sed -n '1,22p' crates/holler-cli/src/profile/rename.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! `holler profile rename`: a stub (story #670). Story #665 owns the real verb: it replaces this
2	//! file and adds the verb's own positionals and flags to the arguments below, edits its own
3	//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
4	//!
5	//! Proposed, not confirmed: #665 waits for the operator, so this verb and its ADR 0003 row
6	//! may be dropped.
7	
8	use clap::Args;
9	
10	use crate::output::{emit_error, not_implemented, VerbCtx};
11	
12	/// The story that owns this verb.
13	const STORY: u32 = 665;
14	
15	/// Rename a profile. (proposed, #665: the operator confirms it first)
16	#[derive(Args, Debug)]
17	pub struct ProfileRename {}
18	
19	/// Run `holler profile rename`: refuse, naming the story that owns it.
20	pub fn run(_args: &ProfileRename, ctx: &mut VerbCtx<'_>) -> i32 {
21	    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
22	}
~~~~

**E42: `crates/holler-cli/src/profile/mod.rs:1-51`** — The frozen verb list and dispatch: the three verbs are declared; their doc line still says they wait for the operator.

Command: `sed -n '1,51p' crates/holler-cli/src/profile/mod.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! `holler profile ...` (epic #633): a profile is a named set of pane specs, kept in the same
2	//! registry as the panes.
3	//!
4	//! **Frozen by story #670.** This file lists the verbs and dispatches them; it never changes
5	//! again. One verb is one file in this directory with its own clap `Args` struct and its own
6	//! `run`, owned by one story. Until then each verb is a stub that refuses with
7	//! `not implemented (story #NNN)`. No profile verb takes `--profile`: the profile is the verb's
8	//! own argument, so there is no `args.rs` here; `--take-over` is `apply`'s own flag.
9	//!
10	//! The verb modules are named after the verb; their `Args` structs are `Profile<Verb>`.
11	//! `rename`, `export` and `import` are proposed (#665) and wait for the operator to confirm them.
12	
13	pub mod apply;
14	pub mod create;
15	pub mod delete;
16	pub mod export;
17	pub mod import;
18	pub mod list;
19	pub mod rename;
20	pub mod show;
21	
22	use clap::Subcommand;
23	
24	use crate::output::VerbCtx;
25	
26	/// The `holler profile` verbs.
27	#[derive(Subcommand, Debug)]
28	pub enum ProfileCmd {
29	    Create(create::ProfileCreate),
30	    Delete(delete::ProfileDelete),
31	    List(list::ProfileList),
32	    Show(show::ProfileShow),
33	    Apply(apply::ProfileApply),
34	    Rename(rename::ProfileRename),
35	    Export(export::ProfileExport),
36	    Import(import::ProfileImport),
37	}
38	
39	/// Run a `holler profile` verb and return its exit code (0 ok, 1 failure, 2 usage, 3 refusal).
40	pub fn run(cmd: &ProfileCmd, ctx: &mut VerbCtx<'_>) -> i32 {
41	    match cmd {
42	        ProfileCmd::Create(args) => create::run(args, ctx),
43	        ProfileCmd::Delete(args) => delete::run(args, ctx),
44	        ProfileCmd::List(args) => list::run(args, ctx),
45	        ProfileCmd::Show(args) => show::run(args, ctx),
46	        ProfileCmd::Apply(args) => apply::run(args, ctx),
47	        ProfileCmd::Rename(args) => rename::run(args, ctx),
48	        ProfileCmd::Export(args) => export::run(args, ctx),
49	        ProfileCmd::Import(args) => import::run(args, ctx),
50	    }
51	}
~~~~

**E43: `crates/holler-cli/src/output.rs:207-217`** — `emit`: how a verb prints its one result in both formats.

Command: `sed -n '207,217p' crates/holler-cli/src/output.rs | awk -v s=207 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
207	pub fn emit<T: Serialize>(
208	    sink: &mut Sink<'_>,
209	    format: Format,
210	    result: Result<T, ErrorBody>,
211	    text: impl FnOnce(&T) -> String,
212	) -> i32 {
213	    match format {
214	        Format::Text => emit_text(sink, result, text),
215	        Format::Json => emit_json(sink, result),
216	    }
217	}
~~~~

**E44: `crates/holler-cli/src/output.rs:266-286`** — `emit_text`: a text error is `error: <message>` on stderr, the code is the exit class; JSON carries `error.code`.

Command: `sed -n '266,286p' crates/holler-cli/src/output.rs | awk -v s=266 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
266	fn emit_text<T>(
267	    sink: &mut Sink<'_>,
268	    result: Result<T, ErrorBody>,
269	    text: impl FnOnce(&T) -> String,
270	) -> i32 {
271	    match result {
272	        Ok(data) => {
273	            let rendered = text(&data);
274	            let written = if rendered.is_empty() {
275	                Ok(())
276	            } else {
277	                write_line(sink.out, &rendered)
278	            };
279	            settle(written, 0)
280	        }
281	        Err(error) => {
282	            let written = write_line(sink.err, &format!("error: {}", error.message));
283	            settle(written, exit_code(&error))
284	        }
285	    }
286	}
~~~~

**E45: `crates/holler-cli/tests/fixtures/cli-surface.txt:166-190`** — The fixture lines by owning story; the three #665 lines are bare.

Command: `sed -n '166,190p' crates/holler-cli/tests/fixtures/cli-surface.txt | awk -v s=166 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
166	# #650
167	pane import | --from /srv/state --herdr-session main
168	pane import | --from /srv/state --herdr-session main --launcher /srv/state/launcher.toml --orchestrator demo-c1r1 --orchestrator demo-c2r1 --dry-run --format=json
169	
170	# #662
171	profile create | Demo
172	profile create | "Some Profile" --from-current
173	profile create | Copy --from Demo --format=json
174	profile delete | Demo
175	profile delete | Demo --keep-panes --json
176	profile list |
177	profile list | --format=json
178	profile show | Demo
179	profile show | "Some Profile" --json
180	
181	# #664
182	profile apply | Demo
183	profile apply | Demo --dry-run --format=json
184	profile apply | "Some Profile" --take-over --herdr-session scratch
185	
186	# #665
187	# (proposed: the operator confirms #665 first)
188	profile rename |
189	profile export |
190	profile import |
~~~~

**E46: `crates/holler-cli/tests/pane_verbs/process/docs_rows.rs:11-34`** — `STORY_GROUPS`: permanent, keyed by (namespace, verb), so `pane import` (#650) and `profile import` (#665) do not collide.

Command: `sed -n '11,34p' crates/holler-cli/tests/pane_verbs/process/docs_rows.rs | awk -v s=11 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
11	use crate::{assert_no_failures, PANE_VERBS, PROFILE_VERBS};
12	
13	/// The verbs each story owns, for the layout checks below (ADR rows and fixture lines are
14	/// grouped by owner). Permanent, unlike `stub::STUBS`: it does not shrink when a verb stops
15	/// being a stub. #648 (`roster`) has no verb here, only a fixture header.
16	const STORY_GROUPS: &[(u32, &[&str])] = &[
17	    (643, &["pane list", "pane get", "pane watch"]),
18	    (644, &["pane launch", "pane relaunch"]),
19	    (645, &["pane switch", "pane reset"]),
20	    (646, &["pane park", "pane unpark", "pane close"]),
21	    (647, &["pane doctor"]),
22	    (650, &["pane import"]),
23	    (
24	        662,
25	        &[
26	            "profile create",
27	            "profile delete",
28	            "profile list",
29	            "profile show",
30	        ],
31	    ),
32	    (664, &["profile apply"]),
33	    (665, &["profile rename", "profile export", "profile import"]),
34	];
~~~~

**E47: `crates/holler-cli/tests/pane_verbs/process/stub.rs:11-31`** — `STUBS`: a verb story deletes its entries and keeps its `// #NNN` line.

Command: `sed -n '11,31p' crates/holler-cli/tests/pane_verbs/process/stub.rs | awk -v s=11 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
11	/// shared process tests that names a stub's owning story.
12	///
13	/// Grouped by story, each group under its own `// #NNN` comment line. A verb story
14	/// deletes its own entries when its verb stops being a stub and **keeps its `// #NNN`
15	/// line**: two stories that delete whole groups, header included, delete adjacent lines,
16	/// and git reports that as a conflict. (`PANE_VERBS` and `PROFILE_VERBS` keep the verb
17	/// itself.)
18	pub const STUBS: &[(&str, &str, u32)] = &[
19	    // #643
20	    // #644
21	    // #645
22	    // #646
23	    // #647
24	    // #650
25	    // #662
26	    // #664
27	    // #665 (proposed: the operator confirms it)
28	    ("profile", "rename", 665),
29	    ("profile", "export", 665),
30	    ("profile", "import", 665),
31	];
~~~~

**E48: `crates/holler-cli/tests/docs_pane_control_test.rs:27-34`** — `PROPOSED_PROFILE_VERBS` and `PROPOSED_FINDING`: the two constants this story removes.

Command: `sed -n '27,34p' crates/holler-cli/tests/docs_pane_control_test.rs | awk -v s=27 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
27	/// The `holler profile` verbs that are PROPOSED and not built (#665): the one hand-kept verb list.
28	/// #665 removes a verb from it when it builds the verb, and then edits the guide's PROPOSED
29	/// clauses and this test.
30	const PROPOSED_PROFILE_VERBS: [&str; 3] = ["rename", "export", "import"];
31	
32	/// The PROPOSED doctor finding and error code (#665): a code of `ALL_CODES`, not yet a kind of
33	/// `FindingKind::ALL`.
34	const PROPOSED_FINDING: &str = "profile-drift";
~~~~

**E49: `crates/holler-cli/tests/docs_pane_control_test.rs:122-185`** — `heading_forms` (skips the PROPOSED verbs) and test 1 (the PROPOSED line rule and the span rule).

Command: `sed -n '122,185p' crates/holler-cli/tests/docs_pane_control_test.rs | awk -v s=122 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
122	fn heading_forms() -> Vec<(String, String, String)> {
123	    let cli = Cli::command();
124	    let mut out = Vec::new();
125	    for ns in ["pane", "profile"] {
126	        let cmd = cli.find_subcommand(ns).unwrap_or_else(|| panic!("no `{ns}` subcommand"));
127	        for verb in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
128	            let name = verb.get_name();
129	            if ns == "profile" && PROPOSED_PROFILE_VERBS.contains(&name) {
130	                continue;
131	            }
132	            let placeholder = |a: &clap::Arg| {
133	                a.get_value_names()
134	                    .and_then(|v| v.first().map(|s| s.to_string()))
135	                    .unwrap_or_else(|| a.get_id().as_str().to_uppercase())
136	            };
137	            let mut form = format!("holler {ns} {name}");
138	            let required = |a: &&clap::Arg| a.is_required_set() && !a.is_global_set();
139	            for a in verb.get_arguments().filter(|a| a.is_positional()).filter(required) {
140	                form.push_str(&format!(" {}", placeholder(a)));
141	            }
142	            for a in verb.get_arguments().filter(|a| !a.is_positional()) {
143	                let long = a.get_long().unwrap_or_default();
144	                let runtime_required = ns == "pane" && name == "import" && long == "herdr-session";
145	                if (a.is_required_set() && !a.is_global_set()) || runtime_required {
146	                    form.push_str(&format!(" --{long} {}", placeholder(a)));
147	                }
148	            }
149	            out.push((ns.to_string(), name.to_string(), form));
150	        }
151	    }
152	    out
153	}
154	
155	/// Acceptance 1: every verb is named in a span or heading, the PROPOSED line rule, and no
156	/// `holler roster --profile`.
157	#[test]
158	fn every_verb_is_named_and_proposed_lines_say_so() {
159	    let doc = guide();
160	    let mut missing = Vec::new();
161	    let cli = Cli::command();
162	    for ns in ["pane", "profile"] {
163	        let cmd = cli.find_subcommand(ns).unwrap();
164	        for verb in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
165	            let span = format!("`holler {ns} {}", verb.get_name());
166	            if ![" ", "`"].iter().any(|end| doc.contains(&format!("{span}{end}"))) {
167	                missing.push(format!("no span or heading starts {span}"));
168	            }
169	        }
170	    }
171	    let proposed_words = PROPOSED_PROFILE_VERBS.iter().map(|v| format!("profile {v}"));
172	    for word in proposed_words.chain([PROPOSED_FINDING.to_string()]) {
173	        let lines: Vec<&str> = doc.lines().filter(|l| l.contains(&word)).collect();
174	        if lines.is_empty() {
175	            missing.push(format!("no line names `{word}` (it must, on a PROPOSED line)"));
176	        }
177	        for l in lines.iter().filter(|l| !l.contains("PROPOSED")) {
178	            missing.push(format!("a line names `{word}` without PROPOSED: {l}"));
179	        }
180	    }
181	    if doc.contains("holler roster --profile") {
182	        missing.push("the guide documents `holler roster --profile`, which #648 has not built".into());
183	    }
184	    assert_none("docs/pane-control.md, the verbs", missing);
185	}
~~~~

**E50: `docs/pane-control.md:104-109`** — Guide section 3's opening paragraph.

Command: `sed -n '104,109p' docs/pane-control.md | awk -v s=104 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
104	## 3. The verbs
105	
106	`holler pane` has `list`, `get`, `watch`, `launch`, `relaunch`, `switch`, `reset`, `park`, `unpark`, `close`, `doctor`
107	and `import`. `holler profile` has `create`, `delete`, `list`, `show` and `apply`, and three verbs that are PROPOSED and
108	not built (`rename`, `export` and `import` under `profile`, #665), at the end of this section.
109	
~~~~

**E51: `docs/pane-control.md:378-384`** — Guide section 3's `The PROPOSED profile verbs` subsection.

Command: `sed -n '378,384p' docs/pane-control.md | awk -v s=378 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
378	### The PROPOSED profile verbs
379	
380	`holler profile rename`, `holler profile export` and `holler profile import` are PROPOSED (#665) and not built: each
381	answers `not-implemented`.
382	- `holler profile rename` (PROPOSED) would change a profile's name and every member's `profile` in one transaction.
383	- `holler profile export` (PROPOSED) would write a profile's specs to a file, named by a planned output flag.
384	- `holler profile import` (PROPOSED) would read a profile's specs from a file, refusing an env value and an existing name.
~~~~

**E52: `docs/pane-control.md:458-468`** — Guide section 5's finding table tail and the `--fix` paragraph ("four name none").

Command: `sed -n '458,468p' docs/pane-control.md | awk -v s=458 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
458	| `shown-driven-mismatch` | The TUI shows another session than the session of record, or its home screen. | `holler pane doctor PANE --fix` |
459	| `stray-session` | A session on a pane's server that is the session of record of no pane. | none |
460	| `unmanaged-stalled` | An unmanaged pane whose screen has not changed for the threshold or longer. Whole-fleet passes with `--unmanaged-stalled-after SECONDS` only. | none |
461	
462	**`--fix` repairs only `shown-driven-mismatch`.** It selects the session of record in the pane's TUI, so the TUI shows
463	the session the record already names, and observes the TUI again. It never changes which session is the session of
464	record, never writes the record's session, and never creates, starts, stops or types into anything. It leaves the
465	orchestrator's pane alone unless the run names that pane (the bracket then says `skipped`).
466	
467	`--fix` does not repair any other kind: each of those names the command to run instead, and four name none. A
468	`shown-driven-mismatch` is also `not-fixable` when its server is not healthy or does not have the session of record; its
~~~~

**E53: `docs/pane-control.md:494-497`** — Guide section 5's PROPOSED sentence about `profile-drift`.

Command: `sed -n '494,497p' docs/pane-control.md | awk -v s=494 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
494	is never stalled, and Herdr 0.8.2 reports no time of the last screen change, so **this kind cannot fire against real
495	Herdr yet** (ADR-0021 section 14).
496	
497	`profile-drift` (a live pane that differs from its spec) is PROPOSED (#665): it is a code of the closed list and not yet a doctor kind.
~~~~

**E54: `docs/pane-control.md:499-510`** — Guide section 6's opening and verb bullets.

Command: `sed -n '499,510p' docs/pane-control.md | awk -v s=499 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
499	## 6. Profiles
500	
501	A profile is a named set of pane specs, kept in the registry beside the panes. A pane belongs to at most one profile
502	(`pane-in-other-profile`). A profile holds names and non-secret values, never a secret: an environment variable is a
503	NAME only, and a spec that carries a value is refused with `profile-secret-refused` (I7). A profile's **live panes** are
504	the records whose `profile` names it.
505	
506	- `create` makes an empty profile, a detached copy (`--from PROFILE`), or a snapshot of every pane (`--from-current`,
507	  each pane becomes a member).
508	- `delete` is refused with `profile-has-live-panes` while panes belong to it; with `--keep-panes` each member is
509	  detached first and keeps running.
510	- `list` and `show` read only. `show` prints the specs, the live panes and, per pane, every field that differs from its
~~~~

**E55: `docs/pane-control.md:662-666`** — Guide section 9's `profile-drift` row.

Command: `sed -n '662,666p' docs/pane-control.md | awk -v s=662 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
662	| `pane-not-found` | refusal (3) | The named pane has no record. |
663	| `session-not-found` | refusal (3) | The named harness session does not exist. |
664	| `store-corrupt` | failure (1) | Stored state cannot be read back; the store fails closed. |
665	| `unavailable` | failure (1) | The hub, Herdr or a harness cannot be reached, or the live state is not there after the act. |
666	| `profile-drift` | failure (1) | PROPOSED (#665): a doctor finding listed in the closed set; never an error today. |
~~~~

**E56: `docs/pane-control.md:783-794`** — Guide section 12 "Later": the last bullet is #665's.

Command: `sed -n '783,794p' docs/pane-control.md | awk -v s=783 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
783	## 12. Later
784	
785	Deferred, so not documented here as existing; none of them is available:
786	
787	- the roster extension: the pane columns on `holler roster` (#648);
788	- applying a pane's model, effort and environment to its harness server (#829);
789	- DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839);
790	- an import that leaves the panes Holler does not drive alone (#835);
791	- the shared subprocess runner of the adapters and the probe (#696);
792	- the real-adapter scenario for `profile apply` (#667);
793	- recreating the Herdr workspace from the stored profiles (#666);
794	- the PROPOSED profile verbs and the PROPOSED doctor finding `profile-drift` (#665).
~~~~

**E57: `docs/adr/ADR-0003.md:70-75`** — ADR 0003's three proposed rows.

Command: `sed -n '70,75p' docs/adr/ADR-0003.md | awk -v s=70 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
70	holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]    #664
71	
72	holler profile rename                                             #665 (proposed: the operator confirms it first)
73	holler profile export                                             #665 (proposed: the operator confirms it first)
74	holler profile import                                             #665 (proposed: the operator confirms it first)
75	
~~~~

**E58: `docs/adr/ADR-0003.md:92-92`** — ADR 0003's sentence that the three rows are proposed.

Command: `sed -n '92,92p' docs/adr/ADR-0003.md | awk -v s=92 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
92	- **`holler pane` and `holler profile`** are pane control (epic #633): the hub keeps one registry of panes and profiles, and every change to a pane goes through these verbs. The rows above list only the flags the skeleton declares (the shared `--profile`, `--spec-only` and `--take-over`, the spec flags and the global `--format`) and no positionals; each owning story adds its verb's positionals and flags to its own row, with its own line in `cli-surface.txt`, and edits no other verb's. Rows are grouped by owning story, one story per block. The rows of `profile rename`, `export` and `import` are **proposed** (#665): the operator confirms them first. ADR-0021 (#634) ratifies the epic's contract.
~~~~

**E59: `docs/adr/ADR-0021.md:3-3`** — ADR-0021's status line.

Command: `sed -n '3,3p' docs/adr/ADR-0021.md | awk -v s=3 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
3	**Status:** accepted (the items marked **PROPOSED** below are not; they wait for the operator, see "Decisions taken")
~~~~

**E60: `docs/adr/ADR-0021.md:106-106`** — ADR-0021 section 2's `ProfileStore` row.

Command: `sed -n '106,106p' docs/adr/ADR-0021.md | awk -v s=106 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
106	| `ProfileStore` | `get`, `list`, `cas_put(profile, expected_generation, actor)`, `delete(name, expected_generation, actor)`, `watch(since)`, `log(name)`, and **PROPOSED** `rename(from, to, expected_generation, actor)` | the hub (#661); `rename` answers `not-implemented` until #665 is confirmed |
~~~~

**E61: `docs/adr/ADR-0021.md:244-247`** — ADR-0021 section 3's verb list.

Command: `sed -n '244,247p' docs/adr/ADR-0021.md | awk -v s=244 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
244	`import`, under the `pane` subcommand group of the `holler` binary. The profile verbs are `create`, `delete`, `list`, `show` and
245	`apply` (REQUIRED, #664; `apply` refuses a pane of another profile unless `--take-over`, C3, and `--dry-run` prints the plan),
246	plus **PROPOSED** `rename`, `export` and `import` (#665). There is no `diff` verb: `profile show` reports where live panes differ
247	from their spec. One verb is one file and one owning story, including its clap `Args` struct (ruling 2); the shared flag groups
~~~~

**E62: `docs/adr/ADR-0021.md:397-405`** — ADR-0021 section 6's method list.

Command: `sed -n '397,405p' docs/adr/ADR-0021.md | awk -v s=397 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
397	connection that sends one still gets `method_not_found`. The names are:
398	
399	- `PANE_METHODS`: `pane/get`, `pane/list`, `pane/cas_put`, `pane/delete`, `pane/watch`.
400	- `PROFILE_METHODS`: `profile/get`, `profile/list`, `profile/cas_put`, `profile/delete`, `profile/watch`, `profile/log`,
401	  and **PROPOSED** `profile/rename`.
402	
403	The control server forwards them through `holler-hub/src/pane_dispatch.rs` to `panes::dispatch` (#639) and
404	`profile::dispatch` (#661, with `profile/rename` routed to `profile/rename.rs` for #665). Each dispatcher gets both
405	registries' state.
~~~~

**E63: `docs/adr/ADR-0021.md:430-451`** — ADR-0021 section 6's `data` table and section 7's lock order and fail-closed rules.

Command: `sed -n '430,451p' docs/adr/ADR-0021.md | awk -v s=430 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
430	| `pane/cas_put`, `profile/cas_put` | the stored record, with its new generation |
431	| `pane/delete`, `profile/delete` | `null` |
432	| `pane/watch`, `profile/watch` | `WatchReply<PaneEvent>`, `WatchReply<ProfileEvent>` |
433	| `profile/log` | an array of `ProfileLogEntry`, oldest first |
434	| `profile/rename` (PROPOSED) | the renamed record |
435	
436	### 7. Persistence on the hub
437	
438	**Decided**, consistent with the brief of #639 (which proposes it for panes) and extended to profiles:
439	
440	- The pane registry is `<state dir>/hub/panes.json` and the profile registry is `<state dir>/hub/profiles.json`, beside
441	  `holds.json`. Each is written at mode `0600` through `holler_proto::atomic_file::write_atomic` (a temporary file in the same
442	  directory, then a rename), the hub's one state-file writer. As with the hub's other state files there is no fsync.
443	- Each file is a versioned JSON document (`"version": 1`). Its exact layout belongs to the registry's story (#639, #661); both
444	  use the same generic load and save helpers. The profile change log is persisted in `profiles.json` with the records,
445	  never inside a `Profile`.
446	- **One lock per registry.** A single mutex guards each registry's whole table. A write runs under it in this order: the
447	  compare-and-swap check, building the next document, saving it, and only then committing to memory and publishing the
448	  event. A failed save therefore changes nothing (memory, file and cursor), and the call answers `unavailable`.
449	- **Lock order.** The pane registry never calls into the profile registry while it holds its own lock. A hub-side
450	  operation that needs both (only the PROPOSED `profile/rename`) takes the profile lock before the pane lock.
451	- **Fail closed.** A missing file is an empty registry. An unreadable file, a parse failure, an unknown version or a failed
~~~~

**E64: `docs/adr/ADR-0021.md:481-486`** — ADR-0021 section 8: "No store transaction spans the two registries."

Command: `sed -n '481,486p' docs/adr/ADR-0021.md | awk -v s=481 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
481	- **Known gap:** a record deleted and then created again restarts at generation 1, so a writer still holding generation 1 of
482	  the old record can overwrite the new one. The contract fixes "0 for a new record", so this is recorded, not fixed.
483	- No store transaction spans the two registries. The membership rule is enforced on `pane/cas_put`: setting `Pane.profile`
484	  to P when the stored pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that
485	  names a pane of another profile (a detached spec) is not refused.
486	
~~~~

**E65: `docs/adr/ADR-0021.md:762-765`** — ADR-0021 section 9's failure-mode row for the three PROPOSED verbs.

Command: `sed -n '762,765p' docs/adr/ADR-0021.md | awk -v s=762 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
762	| `profile show` | `profile-not-found` |
763	| `profile apply` | In the doctor form (#664): the verb fails only before it has a plan, with `profile-not-found`, or `usage` when a spec pane with no record needs `--herdr-session`; every other code is a step's. A `refused` step leaves the exit code 0, and a `failed` step stops the run and exits 1 with that step's code. Per step: `pane-in-other-profile` (without `--take-over`), `probe-failed` (the others proceed), `herdr-version-unsupported`, `grid-out-of-range`, `pane-not-found` and `pane-not-in-profile` (the record or the spec gone during the run), `generation-conflict`, `profile-conflict`; open (#664): `orchestrator-not-relaunched` (an orchestrator that is not healthy) and `spec-not-launchable` (launch or relaunch answered `usage` for the spec); open (#644), from a create or a relaunch: `pane-exists`, `grid-occupied`, `port-in-use`; open (#802), through the launch engine, `agent-unsupported`, and (#642) the OpenCode adapter's `agent-unusable`; open (#640) for a cell that no single split reaches, `grid-unreachable` |
764	| PROPOSED `profile rename`, `export`, `import` | `profile-not-found`, `profile-exists`, `profile-secret-refused`, `command-not-argv`, `grid-ambiguous`, `grid-out-of-range` |
765	
~~~~

**E66: `docs/adr/ADR-0021.md:811-814`** — ADR-0021 section 9's class table row for `profile-drift`.

Command: `sed -n '811,814p' docs/adr/ADR-0021.md | awk -v s=811 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
811	  | `store-corrupt` | Failure (1) | Stored state cannot be read back; the store fails closed. |
812	  | `not-implemented` | Failure (1) | The verb cannot do the work yet; every stub exits 1. |
813	  | `profile-drift` | Failure (1) | A finding kind, not an error a port returns; if one surfaces, live state disagrees with its spec. |
814	  | any other well-formed code (open, raised as `Refused`) | Refusal (3) | `Refused` is the channel for the codes a verb or adapter owns; the open codes so far (#640's `grid-unreachable`, #645's `session-busy`, `session-holds-question` and `prompt-unsupported`, #646's `pane-parked`, `pane-unhealthy` and `pane-shown-driven-mismatch`, all merged, #802's `agent-unsupported`, #642's `agent-unusable`, the OpenCode adapter's, and #650's `import-blocked`) are all refusals. |
~~~~

**E67: `docs/adr/ADR-0021.md:1073-1077`** — ADR-0021 "Decisions taken" item 4: the PROPOSED items.

Command: `sed -n '1073,1077p' docs/adr/ADR-0021.md | awk -v s=1073 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1073	3. **The operation id is deferred to #644 (section 12).** Until then I5 is satisfied by the per-call bound and the loud
1074	   `timeout`.
1075	4. **`profile rename`, `export`, `import`, the `profile/rename` method, `ProfileStore::rename` and the `profile-drift`
1076	   finding (#665) stay PROPOSED** until wave 4. Nothing required depends on them.
1077	5. **Refusals exit 3 (section 9), not 1.** This amends the epic's output contract. #670 is not merged yet and its output
~~~~

**E68: `crates/holler-pane-testkit/src/conformance/profile_store/log.rs:60-77`** — The conformance case `rename-is-not-implemented`, which this story replaces.

Command: `sed -n '60,77p' crates/holler-pane-testkit/src/conformance/profile_store/log.rs | awk -v s=60 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
60	/// Case 17: `rename` is PROPOSED (#665) and answers `not-implemented`, changing
61	/// nothing: the profile, its log and the absence of the new name. #665 replaces this
62	/// case.
63	pub(super) fn rename_is_not_implemented(store: &dyn ProfileStore) -> Result<(), String> {
64	    let who = actor()?;
65	    let p = sample(ALPHA, &[C1])?;
66	    put(store, &p, 0, &who)?;
67	    let before = shown(store, &p.name)?;
68	    let q = profile_name(BETA)?;
69	    let call = "rename(p, q, 1) while rename is PROPOSED (#665)";
70	    expect_code(call, store.rename(&p.name, &q, 1, &who), NOT_IMPLEMENTED)?;
71	    unchanged(store, &p.name, &before, call)?;
72	    expect_eq(
73	        "get of q after the refused rename",
74	        succeeds("get", store.get(&q))?,
75	        None,
76	    )
77	}
~~~~

**E69: `crates/holler-pane-testkit/src/conformance/profile_store/watch.rs:119-127`** — The case that uses a PROPOSED `rename` as its fifth refused write.

Command: `sed -n '119,127p' crates/holler-pane-testkit/src/conformance/profile_store/watch.rs | awk -v s=119 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
119	    expect_code(call, store.cas_put(&shouted, 0, &who), EXISTS)?;
120	    unchanged(store, &p.name, &before, call)?;
121	    let call = "rename(p, q, 2) while rename is PROPOSED (#665)";
122	    let renamed = store.rename(&p.name, &missing, 2, &who);
123	    expect_code(call, renamed, NOT_IMPLEMENTED)?;
124	    unchanged(store, &p.name, &before, call)?;
125	    expect_eq(
126	        "next() after five refused writes",
127	        next_item(&mut watch)?,
~~~~

**E70: `crates/holler-pane-testkit/src/profile_store.rs:294-306`** — The fake's `rename` (a stub).

Command: `sed -n '294,306p' crates/holler-pane-testkit/src/profile_store.rs | awk -v s=294 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
294	
295	    /// PROPOSED (#665): after the fault switch, `not-implemented`, changing nothing.
296	    fn rename(
297	        &self,
298	        _from: &ProfileName,
299	        _to: &ProfileName,
300	        _expected_generation: u64,
301	        _actor: &Actor,
302	    ) -> Result<Profile, PaneError> {
303	        self.faults.enter(ProfileStoreOp::Rename)?;
304	        Err(PaneError::NotImplemented)
305	    }
306	}
~~~~

**E71: `crates/holler-pane-testkit/src/conformance/profile_store.rs:128-130`** — The case table entry for the stub case.

Command: `sed -n '128,130p' crates/holler-pane-testkit/src/conformance/profile_store.rs | awk -v s=128 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
128	    ),
129	    ("rename-is-not-implemented", log::rename_is_not_implemented),
130	    ("env-is-names-only", log::env_is_names_only),
~~~~

**E72: `crates/holler-cli/tests/profile_verbs/rig.rs:1-12`** — The profile verbs' rig, which "#664 and #665 extend".

Command: `sed -n '1,12p' crates/holler-cli/tests/profile_verbs/rig.rs | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	//! The rig the `holler profile` verb tests run over (story #662, Decision 12).
2	//!
3	//! Declared from `list.rs` by `#[path]` (so `profile_verbs/main.rs`, #670's, is not
4	//! edited) and reached by the other verb files as `crate::list::rig`. #664 and #665
5	//! extend this file, not `list.rs`.
6	//!
7	//! The two stores are the test kit's fakes. The adapters and the prober are fakes too,
8	//! kept only so their call logs prove a profile verb never calls them; the scope is
9	//! `Unwired`, so a call to it fails the verb. Each format runs on its own freshly seeded
10	//! rig ([`run_both`]).
11	
12	use std::sync::atomic::{AtomicUsize, Ordering};
~~~~

**E73: `git -C ../0831-claude-harness diff -- crates/holler-pane/src/findings.rs`, lines 1-60** — IN FLIGHT, uncommitted when read: #831 appends two kinds (`pane-waiting`, `claude-login-missing`) to `FindingKind::ALL`, `code` and `remedy`. #665 appends a third at the end and rebases.

Command: `git -C ../0831-claude-harness diff -- crates/holler-pane/src/findings.rs | sed -n '1,60p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	diff --git a/crates/holler-pane/src/findings.rs b/crates/holler-pane/src/findings.rs
2	index b8b0e92..990fbe6 100644
3	--- a/crates/holler-pane/src/findings.rs
4	+++ b/crates/holler-pane/src/findings.rs
5	@@ -3,10 +3,11 @@
6	 //! untrusted text a finding quotes. #665 adds only the `ProfileDrift` kind after #647 has
7	 //! merged.
8	 //!
9	-//! - [`FindingKind`]: twelve kinds, each with a stable kebab-case code that is also its JSON
10	-//!   form ([`FindingKind::code`] is the one source of both). A finding is a report, not an
11	-//!   error: its code is never a `RefusalCode`, and two codes (`herdr-version-unsupported`,
12	-//!   and #665's `profile-drift`) equal closed error codes.
13	+//! - [`FindingKind`]: fifteen kinds, each with a stable kebab-case code that is also its JSON
14	+//!   form ([`FindingKind::code`] is the one source of both). #832 added `unmanaged-stalled`,
15	+//!   and #831 `pane-waiting` and `claude-login-missing`. A finding is a report, not an error:
16	+//!   its code is never a `RefusalCode`, and two codes (`herdr-version-unsupported`, and #665's
17	+//!   `profile-drift`) equal closed error codes.
18	 //! - [`Finding`]: one finding, with what it is about (a pane, a position, a Herdr pane, a
19	 //!   session, harness servers), a one-line message, its remedy and what `--fix` did.
20	 //! - **Remedies** ([`FindingKind::remedy`]) name only the verb that owns each repair
21	@@ -73,6 +74,13 @@ pub enum FindingKind {
22	     /// A Herdr pane no record names has shown the same screen for longer than the
23	     /// operator's threshold (#832). Only a whole-fleet pass with a threshold looks.
24	     UnmanagedStalled,
25	+    /// A pane's single process (Claude Code) waits for a person: a permission prompt or a
26	+    /// question; the message names which (#831). No Holler verb answers one.
27	+    PaneWaiting,
28	+    /// The processes of the Claude Code panes do not use the subscription login (#831): once
29	+    /// per pass that holds such a pane. The message names the login, never a command that
30	+    /// reads one, and no Holler verb sets a login (I7).
31	+    ClaudeLoginMissing,
32	 }
33	 
34	 impl FindingKind {
35	@@ -91,6 +99,8 @@ impl FindingKind {
36	         FindingKind::ShownDrivenMismatch,
37	         FindingKind::StraySession,
38	         FindingKind::UnmanagedStalled,
39	+        FindingKind::PaneWaiting,
40	+        FindingKind::ClaudeLoginMissing,
41	     ];
42	 
43	     /// The kind's stable kebab-case code, which is also its JSON form. A code is stable
44	@@ -110,6 +120,8 @@ impl FindingKind {
45	             FindingKind::ShownDrivenMismatch => "shown-driven-mismatch",
46	             FindingKind::StraySession => "stray-session",
47	             FindingKind::UnmanagedStalled => "unmanaged-stalled",
48	+            FindingKind::PaneWaiting => "pane-waiting",
49	+            FindingKind::ClaudeLoginMissing => "claude-login-missing",
50	         }
51	     }
52	 
53	@@ -122,13 +134,17 @@ impl FindingKind {
54	     /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
55	     ///   one (choosing an existing session is the operator's, never a guess);
56	     /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
57	-    ///   it cannot or failed, nothing once it is fixed.
58	+    ///   it cannot or failed, nothing once it is fixed;
59	+    /// - a pane that waits for a person, and a login that is not the subscription's (#831):
60	+    ///   none, since only a person answers the one and logs in for the other.
~~~~

**E74: `git show issue-839-implementation:docs/handoffs/839-brief.md`, lines 297-299** — IN FLIGHT: #839's blast radius names `reconcile.rs`, `reconcile/observe.rs`, `findings.rs` and the `Ports` literals.

Command: `git show issue-839-implementation:docs/handoffs/839-brief.md | sed -n '297,299p' | awk -v s=297 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
297	- `crates/holler-pane/src/{lib.rs,ports.rs,gate.rs,driven.rs,hub.rs (the last three new),tx_switch.rs,reconcile.rs,reconcile/observe.rs,findings.rs}`, `crates/holler-pane/tests/{driven_test.rs (new),ports_test.rs}`.
298	- `crates/holler-pane-testkit/src/{lib.rs,fixture.rs,hub.rs (new),conformance/hub.rs (new),conformance/mod.rs}`, and its tests.
299	- `crates/holler-cli/src/{say_cmd.rs,pane/wiring.rs,pane/wiring/hub.rs (new),pane/reset.rs (doc)}`, `crates/holler-cli/Cargo.toml` (the new `[[test]]` `driven_e2e_test`), the 15 `Ports` literals listed above, `crates/holler-cli/tests/{driven_e2e_test.rs (new),pane_verbs/driven.rs (new),pane_verbs/reset.rs,pane_verbs/main.rs,support/driven.rs (new),support/mod.rs (one line),pane_integration/scenario.rs}`.
~~~~

**E75: `git show issue-648-implementation:docs/handoffs/648-brief.md`, lines 123-127** — IN FLIGHT: #648 edits the same guide sections (4 and 12) and the same docs test (`every_verb_is_named_and_proposed_lines_say_so`).

Command: `git show issue-648-implementation:docs/handoffs/648-brief.md | sed -n '123,127p' | awk -v s=123 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
123	  - `docs/pane-control.md` (794 lines) and `crates/holler-cli/tests/docs_pane_control_test.rs` (523 lines), the exact edits (R9):
124	    - Guide section 4, line 432, the sentence "The roster extension (the PANE, POS, PROFILE, HEALTH, SHOWN and DRIVEN columns on `holler roster`, #648) is later: ...": replace it by the paragraph below. Section 12, line 787, the bullet "the roster extension: the pane columns on `holler roster` (#648);": delete it. Nothing else of the guide changes: the DRIVEN row and the "SHOWN against DRIVEN" paragraph already say DRIVEN is `-` until #839, and section 10's "SYNC is read from `pane list` and `doctor`, not from the roster" stays true, since the roster has no SYNC column.
125	    - Test `every_verb_is_named_and_proposed_lines_say_so` (lines 155-184): delete the guard `if doc.contains("holler roster --profile") { ... }` (lines 181-183) and the words "and no `holler roster --profile`" in its doc comment (line 156). It pinned the absence of the flag this story builds, so it fails the moment the guide documents it.
126	    - Test `section_4_marks_the_roster_extension_later` (lines 295-303; acceptance 7 of #652): replace it by `section_4_documents_the_roster_columns`. Section 4 names each of `holler_cli::roster_cmd::PANE_COLUMNS` as a whole word and has a line that names `holler roster` and `--profile`; no line of the guide names `holler roster`, `#648` and `later` together; no line of section 12 names `#648`. #652's own text anticipates this: its section 12 is the list of what is later, and its roster bullet exists to be removed when #648 lands.
127	    - `docs_cli_test` parses every `holler ...` command the guide shows, so the examples in the paragraph (`holler roster --profile fleet`, `holler roster --format=json`) must parse; they do.
~~~~

**E76: `gh issue view 647 --json comments -q '.comments[-1].body'`, lines 1-3** — #647 part 2's operator decision (adds read-only port methods; `agent-cannot-dispatch` and `unattached`, which edit `findings.rs` and `reconcile.rs`).

Command: `gh issue view 647 --json comments -q '.comments[-1].body' | sed -n '1,3p' | awk -v s=1 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
1	Operator decision, 2026-10-11: the contract change for part 2 is **allowed**. `agent-cannot-dispatch` and `unattached` may add read-only port methods (default-armed or required as the contract amendment decides; #835 and #832 already set the pattern with `HostPort::foreground_commands` and `HerdrPort::observe_pane`, and #649's wrappers must forward every new method). The amendment comes first, per the epic's amend-first rule; part 2's brief starts from this decision and from the plan-check findings in `docs/handoffs/647/handoff-A.md` in its worktree.
2	
3	Filed by an AI agent (Claude Code) at the operator's direction.
~~~~

**E77: `crates/holler-hub/src/profile/handlers.rs:41-55`** — The handler pattern: each method is `panes::handlers::run` with a closure over the store; `profile/rename` follows it with both stores in the closure.

Command: `sed -n '41,55p' crates/holler-hub/src/profile/handlers.rs | awk -v s=41 '{printf "%d\t%s\n", s+NR-1, $0}'`

~~~~text
41	/// `profile/cas_put`: the stored record.
42	pub(crate) async fn cas_put(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
43	    run(cid, obj, move |params: ProfileCasPutParams| {
44	        store.cas_put(&params.profile, params.expected_generation, &params.actor)
45	    })
46	    .await
47	}
48	
49	/// `profile/delete`: `null`.
50	pub(crate) async fn delete(cid: &CorrelationId, obj: &Value, store: Arc<Store>) -> String {
51	    run(cid, obj, move |params: ProfileDeleteParams| {
52	        store.delete(&params.name, params.expected_generation, &params.actor)
53	    })
54	    .await
55	}
~~~~
