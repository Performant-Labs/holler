# [docs: the operator guide for `holler pane`, the deprecation of hand-managed session files, and the runbook entries](https://github.com/Performant-Labs/holler/issues/652)

Part of epic #633. This is the complete statement of the story (consolidated 2026-10-11 from the original scope, its amendments of 2026-10-08, and the operator's decisions of 2026-10-11). Where an older comment disagrees, this text is the decision. The story is **docs plus one docs test**: it changes no behaviour. It is briefed ahead of its dependencies and built after #649 and #664 have merged.

## Scope (final)

Write `docs/pane-control.md` (new), update `docs/orchestrating.md`, and add the guide to `docs/README.md`. The guide has these sections, in this order:

1. **The rule (I1).** Nothing but `holler pane` and `holler profile` changes a pane; the registry is the one source of truth for what a pane is; Herdr, tmux and the harness are observed (I6), never inferred from files.
2. **What a pane runs.** One Herdr pane shows one tmux session; the harness server (OpenCode) runs *inside that tmux session*, started through the host adapter from the command `HarnessPort::serve_command` builds, and stopped by `HostPort::stop_owned` (TERM, a shared grace, then KILL of every process group still alive) (#802). `--agent KEY` sets the server's default agent when it starts; a key the running server cannot use is refused with the adapter's code `agent-unusable` and the launch is rolled back; `relaunch` without `--agent` keeps the recorded agent; there is no flag to clear an agent; the agent is printed as `agent=KEY` and as an additive JSON key `agent` (#644 part 2, #642 part 3). (Review round 1, NV-1.) `agent-unusable` is not one of `ALL_CODES` (E7, E8): it is an **open** code, the OpenCode adapter's own constant (#642 decision D5 and scope item 2, E26; ADR-0021's rule for open codes, E25), and every open code is a refusal, exit 3 (`class_of`, E23). The guide says so where it names the code.
3. **The verbs.** `holler pane` `list`, `get`, `watch`, `launch`, `relaunch`, `switch`, `reset`, `park`, `unpark`, `close`, `doctor`, `import`; and `holler profile` `create`, `delete`, `list`, `show`, `apply`, plus `rename`, `export`, `import`, which are **PROPOSED** (#665, not built) and marked as such, with `profile-drift` likewise PROPOSED. One short subsection per verb: purpose, the flags, an example, the exit codes it can end with. `--profile` has two classes: spec-editing (`launch`, `relaunch`, `close`) and scoping (the rest); `pane import` takes no `--profile`; `--spec-only` and `--take-over`. (Review round 1, NIT-2.) `--spec-only` exists only on the spec-editing verbs (`launch`, `relaunch`, `close`) and requires `--profile` (E19, lines 9 and 31); the guide shows it on no other verb. (Review round 1, NV-2.) `profile-drift` is a PROPOSED `doctor` finding code, not a verb (#665: "A doctor finding `profile-drift` (a live pane differs from its spec)", E31): it is a string of `ALL_CODES` (E7, E8; class failure, exit 1, E23) and not yet a kind in `FindingKind::ALL` (E5), so acceptance 3 puts it in the guide and acceptance 11 keeps every line that names it marked `PROPOSED`.
4. **Reading the table.** The columns of `holler pane list` (PANE, POS, PROFILE, PROJECT, HEALTH, SHOWN, DRIVEN, SYNC, HOLD) and what each means; SHOWN and HEALTH are what reconcile last recorded; DRIVEN is printed as the record holds it. **The roster extension (the PANE/POS/PROFILE/HEALTH/SHOWN/DRIVEN columns on `holler roster`, #648) is marked "later"**: this guide documents `pane list` and `doctor` as the places to read SHOWN against DRIVEN, and says `holler roster` does not show them yet.
5. **`doctor`.** What one reconcile pass reports (every finding kind, in `FindingKind::ALL`), what `--fix` repairs (only `shown-driven-mismatch`: it selects the session of record; it never changes the session of record), what it will not fix (every other kind, with the command the finding names), that a pass that completes exits 0 with findings in the report, and the periodic pass (a scheduler runs `holler pane doctor --format=json` without `--fix`; there is no hub timer). (Review round 1, NIT-1.) The guide words `--fix` so that it cannot be read as editing the record: `--fix` makes the pane's TUI show the session of record ("it selects the session of record", E5, lines 68-69; "the TUI was seen showing the session of record afterwards", E6, line 164), and it never changes which session is the session of record (E17, line 33). (Review round 1, B-2.) The guide's list of finding kinds says that `herdr-version-unsupported` is also a closed error code with the same wire string (E5, E7), and points to section 8.
6. **Profiles.** What a profile is (a set of pane specs; at most one profile per pane; no secrets, env var names only); `create` (with `--from-current`), `delete` (refused while panes are live, `--keep-panes`), `list`, `show`; and `apply`, whose reference is the decided contract in "The `profile apply` reference" below.
7. **Grid notation.** Positions are rowcol and 1-based: text `r2c1`, JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first. Input accepts `r<row>c<col>`, `c<col>r<row>` and a bare `<row>,<col>` (always row first); the refusals `grid-ambiguous` and `grid-out-of-range`; and that **pane names such as `hj-c1r2` predate the standard and list the column first, so `hj-c1r2` sits at `r2c1`; no pane is renamed and a name is never read as a position**. (Review round 1, W-2.) The section's example block (acceptance 5) also carries the pane name itself as a refused form, `refused grid-ambiguous hj-c1r2`: `GridPos::parse` refuses it as unreadable, because it holds no comma and a labelled cell must start with `r` or `c` (E11, lines 10-11; E33, line 108).
8. **Probe, argv, layout, Herdr version.** The health probe, with the example `check ["curl","-s","http://127.0.0.1:8095/v1/models"]`, `expect ["qwen38"]` and the refusal `probe-failed`; the argv rule (`--command-arg` / `--command-json`, never a shell string, `command-not-argv`); layout reached by splits; the Herdr API version shown by `doctor` and the finding `herdr-version-unsupported`. (Review round 1, B-2.) The guide says plainly that `herdr-version-unsupported` is one wire string with two roles: a **finding** that `doctor` reports, in a pass that still exits 0 (E5, E17; "the reported `herdr-version-unsupported`", E25, line 526), and a closed **error code** in `ALL_CODES` (E7), class refusal, exit 3 (E23, line 288), with which a verb that needs a supported Herdr ends: `pane launch`, `pane relaunch` and `profile apply` (E25, lines 522 and 533) and `pane import` (#650, E27, line 21).
9. **JSON output reference.** `--format=json` (and `--json` under `pane` and `profile`) prints exactly one envelope per result with `schema_version` 1; `pane watch` prints one envelope per line (NDJSON, resumable with `--since CURSOR`); the exit codes (a table: 0 ok, 1 runtime failure, 2 usage, 3 refusal, the same in text and JSON mode); and **every error code** (`ALL_CODES`) with its class. (Review round 1, B-1.) The guide states plainly that `pane watch` is the one exception to "one envelope per result": its JSON output is a stream (NDJSON, one envelope per line, resumable with `--since CURSOR`) and never a single result envelope ("`pane watch` is the one stream: one envelope per line", E10, lines 7-8; E18). (Review round 1, NV-1.) The code list is `ALL_CODES`, the closed set; the guide also says that a verb or an adapter owns further **open** codes, which are not in `ALL_CODES` and are always class refusal, exit 3 (E23, E25), and names `agent-unusable` of section 2 as an example.
10. **Migration and cutover.** The procedure of #654: run `holler pane import --dry-run` on the live fleet and read every finding; fix drift with the old tools; import (a real import also creates the profile `fleet` from today's panes and their layout); run `doctor` until SHOWN equals DRIVEN for every pane (and `pane list` shows it); cut **one** pane over to `launch` / `reset` and watch it **through a full overnight**; then move the rest; seven days with no invisible session (every day `doctor` reports no `shown-driven-mismatch` and no stray session older than an hour) and the operator signs off. Record each step in `docs/handoffs/<#654>-cutover/decisions.md`. **Files the old launcher wrote that are now derived** (a list): `sessions.toml`, `last-session`, `owned-sessions`, `pane.env`; they are kept, read-only, for one release. (Review round 1, NV-3.) What the import does is #650's (E27): one real run writes the records and the profile `fleet`, or nothing at all, and "their layout" is each pane's position as Herdr's snapshot shows it (the `grid` of the snapshot entry whose label is the pane's name), never a position read from the name (E27, lines 7, 25, 26 and 42). (Review round 1, NV-4.) The import reads the old files as the source of the records it creates, never as observed state, so section 1's rule (I6, E30) holds during the migration, and the guide says so: the import observes Herdr, tmux and the harness only through the ports, a session id read from a file becomes the session of record only when a healthy server's session list holds it, and nothing in `last_observed` is read from a file (E27, lines 21, 30, 34 and 42).
11. **Rollback.** **`scripts/pane-restart` stays as the fallback until the 7-day sign-off**; the guide says so, shows its usage line, and says the script is removed (in a later story, not this one) after the sign-off. Restarting a pane with the verbs is `holler pane relaunch PANE --model ...`.
12. **Deferred, so not documented as existing:** the roster extension (#648), the shared subprocess runner (#696), the real-adapter scenario for `profile apply` (#667) and the workspace recreation from profiles (#666) are after the cutover; the guide may name them once under "Later" and describes none of them as available.

`docs/orchestrating.md`: the section "Restarting a pane on another model (interim)" is rewritten so that `holler pane relaunch PANE --model PROVIDER/ID [--effort E]` is the way, and `scripts/pane-restart` is named as the fallback **until the 7-day sign-off** (replacing the sentence "This section is removed when #644 and #653 have merged"); the rest of that page is unchanged. The agent guidance for orchestrators is this page and the `docs/README.md` entry; `AGENTS.md` and `agent-skills/` are out of scope.

## The `profile apply` reference (decided in the #664 brief; quote it, do not restate differently)

- One step per spec pane in (workspace, row, col) order; the first matching row applies: **a** another profile and no `--take-over`: `refused` (`pane-in-other-profile`); **b** no record: `create` through the launch transaction; **c** healthy and matches its spec: `adopt` (sets `Pane.profile`; with a take-over from another profile, leave then join, two writes); **d** healthy and differs: `reported` with the pane's diff, nothing live touched; **e** not healthy, role orchestrator: `refused` (`orchestrator-not-relaunched`), no write even under `--take-over`; **f** not healthy, role agent: `relaunch`, and under `--take-over` from another profile the leave and join happen **first**, then the relaunch.
- Outcomes: `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`. `refused` and `reported` do not stop the run or change the exit code. A `failed` step (`unavailable`, `timeout`, a failed health call, a store conflict) stops the run; the later steps are `skipped`. Exit code: 1 if any step failed, 2 for the up-front usage case (a plan with a `create` and no `--herdr-session`), else 0.
- A run that ends with a `failed` step prints **only the failure**: in JSON the failure envelope (`ok` false, `data` null, `error.message` is `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`), in text one `error:` line on stderr and nothing on stdout; a dry run with a failed step prints no plan.
- `--dry-run` prints the same table as the plan, runs no probe, issues no split and writes nothing. `data` is `{"profile": NAME, "dry_run": bool, "steps": [{pane, action, outcome, code?, diff?}]}`. Flags: `--herdr-session NAME`, `--take-over`, `--dry-run`, `--format`.
- "Healthy" is a live observation (the harness answers its health call, Herdr's snapshot lists the recorded pane, and a readable shown session equals the session of record), never the stored health; apply never kills a healthy pane; a shown session that cannot be read does not make a pane unhealthy.

(Review round 1, B-3.) **The failure envelope, stated once.** O6 (E20) words the failed run's message twice, and both wordings describe one message: O6's first bullet says what `error.message` names (the pane, the step's action and how many later steps were skipped), and its third bullet pins the exact text, `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`, which names all three and also carries the failed step's own error message. So a run that ends with a `failed` step prints, in JSON, exactly one envelope: `schema_version` 1, `ok` false, `data` null, `error.code` the failed step's error code, and `error.message` the pinned text (O6, E20). `data` null is not particular to apply: every failed `pane` or `profile` envelope is written with `data` null, and every successful one with `error` null (`output.rs`, E24, lines 152-153 and 174-181). The guide's `profile apply` reference states the failure envelope once, in this form.

## Operator decisions (2026-10-11)

1. **The guide documents `pane list` and `doctor` for SHOWN/DRIVEN and marks the roster extension "later".** Reason: #648 is deferred until after the cutover, and the guide must not describe a column that does not exist.
2. **`scripts/pane-restart` stays as the rollback until the 7-day sign-off.** Reason: it is the only restart path that does not depend on the new registry, and removing it before sign-off removes the way back.
3. **The grep tests derive their lists from the code's own tables**: the verbs from the clap tree (`Cli::command()`), the finding kinds from `FindingKind::ALL`, the error codes from `ALL_CODES`, the columns from `COLUMNS`. Reason: a hand-kept list in the test drifts from the code (the repo's single-source rule).
4. **The `profile apply` reference is the decided shape of the #664 brief** (above), with the O1 to O6 decisions. Reason: one contract, stated once.
5. **`orchestrating.md`'s interim section is rewritten, not deleted**, and names the script as a fallback until the sign-off. Reason: the section's own deletion condition (#653) is in another repository and has not happened.
6. **The cutover procedure keeps the overnight watch on the first pane.** Reason: a day-scale defect (idle drift, timeouts, restarts) shows only over time, and a staged cutover exists to find it on one pane.
7. **Deferred after the cutover:** #648, #696, #667, #666. Reason: none gates #654.

## Acceptance (each is a test)

All new tests live in `crates/holler-cli/tests/pane_control_doc_test.rs`, declared as a `[[test]] name = "pane_control_doc_test"` in `crates/holler-cli/Cargo.toml` (the crate sets `autotests = false`). They read `docs/pane-control.md` and `docs/orchestrating.md` from the workspace root, the way `crates/holler-cli/tests/docs_cli_test.rs` does.

1. **Verbs.** Every subcommand of `pane` and of `profile` read from the clap tree (`holler_cli::Cli::command()`, `find_subcommand("pane")` and `("profile")`, `get_subcommands()`) appears in the guide as `holler pane <verb>` / `holler profile <verb>` inside a heading or a code span. The three PROPOSED profile verbs (`rename`, `export`, `import`) must appear on a line or in a section whose heading contains the word `PROPOSED`.
2. **Finding kinds.** Every `FindingKind::ALL[i].code()` appears in the guide inside backticks.
3. **Error codes.** Every string of `holler_pane::error::ALL_CODES` appears in the guide inside backticks, and the exit-code table of section 9 has a row for each of 0, 1, 2, 3 (the values of `ErrorClass::exit_code`). (Review round 1, B-2.) A code that is both a finding kind and an error code appears inside backticks both in section 5 and in section 9. The test computes these codes as the intersection of `FindingKind::ALL[i].code()` and `ALL_CODES` from the two tables, never from a hand-kept list; at `263a2d7` the intersection is `herdr-version-unsupported` alone (E5, E7).
4. **Columns.** Every name in `holler_cli::pane::list::COLUMNS` appears in the guide's section 4.
5. **Grid examples parse.** The guide's grid section carries a fenced block tagged `text` with one example per line, `ok <form>` or `refused <code> <form>`; the test parses each `ok` form with `holler_pane::GridPos::parse` and each `refused` form and asserts the named code (`grid-ambiguous` or `grid-out-of-range`). The block contains at least the forms `r2c1`, `c1r2`, `2,1`, `R2C1` and one refused form of each code. (Review round 1, W-2.) It also contains the line `refused grid-ambiguous hj-c1r2` (section 7: a pane name is not a position; E33).
6. **The existing docs test still passes:** every `holler ...` command in the new guide parses against the clap tree (`docs_cli_test` scans `docs/**/*.md` already; a command whose leaf verb is in `cli-surface.pending.txt` is tolerated and reported). No bare command fragment appears in backticks. (Review round 1, NIT-3.) "Bare command fragment" is this repository's term for a code span that starts `holler ` and does not parse ("a bare command fragment in backticks fails it", the tester overlay, E32). So the last sentence names the same check as the first: `docs_cli_test` makes it, with its own extraction and placeholder normalisation (E12, E34), and the new test file does not repeat it. A bare namespace such as `holler pane` alone parses as a prefix of the tree and is allowed (E34, lines 189-199).
7. **Roster is marked later.** The guide's section 4 contains a line with `holler roster` and `#648` and the word `later`.
8. **Rollback.** The guide contains `scripts/pane-restart` and the words `until the 7-day sign-off`; `docs/orchestrating.md` contains `holler pane relaunch` and `scripts/pane-restart` and no longer contains `removed when #644 and #653 have merged`.
9. **Derived files.** The guide's section 10 names each of `sessions.toml`, `last-session`, `owned-sessions` and `pane.env`.
10. **Index.** `docs/README.md` links `pane-control.md` under "Guidance".
11. **Deferred items are not described as available:** the guide does not contain `holler roster --profile` as an example, and does not contain the strings `profile export` or `profile import` outside a line marked `PROPOSED`. (Review round 1, NV-2.) The same holds for `profile-drift` (scope section 3): every line of the guide that contains `profile-drift` is a line marked `PROPOSED`.

## Blast radius

- `docs/pane-control.md` (new), `docs/orchestrating.md`, `docs/README.md`
- `crates/holler-cli/tests/pane_control_doc_test.rs` (new) and its `[[test]]` entry in `crates/holler-cli/Cargo.toml`
- `CHANGELOG.md` (an `## [Unreleased]` entry linking this issue)

No source file outside tests changes. Markdown files under `docs/` are scanned by `docs_cli_test`, so the guide must obey its rules (quoted in E12).

## Depends on

#649 and #664 (both before this story is built). The brief can be written and reviewed ahead of them; the guide is written and its tests run after both have merged, because the verbs it describes must exist in the clap tree and the behaviours it states must be the merged ones. If #649 or #664 changes a behaviour stated above, the brief is patched and the review re-run before the build.

(Review round 1, NV-3.) **#650 too, by the rule above.** Sections 3 (`pane import`) and 10 (the cutover) describe the import as #650 builds it, and neither #649 nor #664 depends on #650 (E28); #650 depends on neither of them, nor on this story (E27, line 106). The guide's tests need #650 as well: on `main` at `263a2d7`, `holler pane import` is #670's stub with no arguments, and `cli-surface.pending.txt` lists no command (E29), so a `holler pane import --dry-run` code span fails `docs_cli_test` (E34) until #650 adds the flag. If #650 changes a behaviour stated above, the brief is patched and the review re-run before the build, as for #649 and #664. Section 2's agent behaviour (#802, #642 part 3, #644 part 2) needs no entry of its own: #649 depends on #642 and #644, and #644 is built after #802 and #642 part 3 (E28).

## Pipeline

`rigor: second-opinion`. Fresh worktree, branch `issue-652-implementation`.

## Rules for this story

See the epic (#633): the contract section there is fixed, so this story builds against it without waiting for another story's code. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`). Edit only the files in the Blast radius; the shared hot spots in the epic are owned by one story each. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes from the test kit and temporary directories. Never print or commit a credential.

## AI assistance

This issue was drafted by Claude (Sonnet 5.5) from the operator's decisions in session; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.


## Evidence (verbatim)

The outside reviewer sees only this body. Every excerpt below was cut by command from `origin/main` at `263a2d7` (`git show origin/main:PATH` and line numbers shown), never retyped; the excerpts of the #664 brief (E20), of issue #654 (E21) and of the decisions of #802 (E22) are quoted from the places named in their headings.

**E1: `crates/holler-cli/src/cli.rs:101-127`** — The top-level command enum and how the global flags are read from the clap tree (the pattern the verb test reuses with `Cli::command()`); `Pane` and `Profile` are subcommands.

~~~~text
101	#[derive(Subcommand, Debug)]
102	pub enum Command {
103	    /// Operate a hub (serve, tokens, status, caps, support, query).
104	    Hub(Hub),
105	    /// Operate a body (join, run, detach, status, caps, support, query, attach).
106	    Body(Body),
107	    /// List the roster. (hub-only, top-level daily verb)
108	    Roster(Roster),
109	    /// Send a one-shot prompt to a session and print the reply. (hub-only)
110	    Say(Say),
111	    /// Interrupt the session's in-flight turn. (hub-only)
112	    Interrupt(Interrupt),
113	    /// Resolve a held permission/elicitation. (hub-only)
114	    Answer(Answer),
115	    /// Block until a session settles/blocks/fails/vanishes. (hub-only)
116	    Wait(Wait),
117	    /// Refuse new work to a session until it is released. (hub-only)
118	    Hold(Hold),
119	    /// Lift a hold set with `hold`. (hub-only)
120	    Release(Release),
121	    /// Control panes: launch, switch, park, close, doctor, import. (epic #633)
122	    #[command(subcommand)]
123	    Pane(PaneCmd),
124	    /// Manage profiles, the named sets of pane specs. (epic #633)
125	    #[command(subcommand)]
126	    Profile(ProfileCmd),
127	}
~~~~

**E2: `crates/holler-cli/src/pane/mod.rs:34-49`** — The `holler pane` verbs (acceptance 1 reads them from the clap tree).

~~~~text
34	/// The `holler pane` verbs.
35	#[derive(Subcommand, Debug)]
36	pub enum PaneCmd {
37	    List(list::PaneList),
38	    Get(get::PaneGet),
39	    Watch(watch::PaneWatch),
40	    Launch(launch::PaneLaunch),
41	    Relaunch(relaunch::PaneRelaunch),
42	    Switch(switch::PaneSwitch),
43	    Reset(reset::PaneReset),
44	    Park(park::PanePark),
45	    Unpark(unpark::PaneUnpark),
46	    Close(close::PaneClose),
47	    Doctor(doctor::PaneDoctor),
48	    Import(import::PaneImport),
49	}
~~~~

**E3: `crates/holler-cli/src/profile/mod.rs:1-37`** — The `holler profile` verbs; `rename`, `export` and `import` are proposed (#665).

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
~~~~

**E4: `crates/holler-cli/src/pane/list.rs:28-38`** — The columns of `pane list` (acceptance 4) and the empty-cell marker.

~~~~text
28	/// The table's columns, in order. A `pane watch` line prints the same cells after the
29	/// pane's name, each as `key=value` with the column name in lower case.
30	pub const COLUMNS: [&str; 9] = [
31	    "PANE", "POS", "PROFILE", "PROJECT", "HEALTH", "SHOWN", "DRIVEN", "SYNC", "HOLD",
32	];
33	
34	/// What a text cell or field holds when the record has no value.
35	pub const NO_VALUE: &str = "-";
36	
37	/// What separates two columns of the table.
38	const GAP: &str = "  ";
~~~~

**E5: `crates/holler-pane/src/findings.rs:40-109`** — `FindingKind`, its `ALL` table and the stable `code()` strings (acceptance 2).

~~~~text
40	/// What a reconcile pass found about a pane, a harness server, a session or Herdr.
41	#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
42	pub enum FindingKind {
43	    /// Herdr reports an API version the adapter does not know; the message names the
44	    /// supported ones. No Holler verb upgrades Herdr.
45	    HerdrVersionUnsupported,
46	    /// A port call of the pass failed, so nothing was concluded from what it would have
47	    /// observed. The message names the call as `<port>.<method>` and the error's code. It is
48	    /// a read (`herdr.snapshot`, `host.ps`, `harness.health`, ...) or the record write
49	    /// (`pane_store.cas_put`), a write that lost a race with another writer included.
50	    ObserveFailed,
51	    /// A Herdr pane that no record names: a process with no record. Whole-fleet runs only.
52	    UnregisteredHerdrPane,
53	    /// Herdr has no pane where the record says the pane is.
54	    HerdrPaneMissing,
55	    /// The pane's tmux session does not exist.
56	    TmuxSessionMissing,
57	    /// The pane's harness server accepts a connection and never answers.
58	    ServerWedged,
59	    /// The pane's harness server fails its health check, and is not wedged.
60	    ServerDown,
61	    /// The record names no session of record. Doctor never picks one.
62	    NoSessionOfRecord,
63	    /// The pane's server answered, and it does not have the session of record.
64	    SessionOfRecordMissing,
65	    /// The TUI shows a session its pane's own server does not have: a harness the record
66	    /// does not know is in the pane.
67	    TuiForeignSession,
68	    /// The TUI shows another session than the session of record, or its home screen. The
69	    /// one kind `--fix` repairs: it selects the session of record.
70	    ShownDrivenMismatch,
71	    /// A session on a pane's server that is the session of record of no pane.
72	    StraySession,
73	}
74	
75	impl FindingKind {
76	    /// Every kind, in declaration order.
77	    pub const ALL: &'static [FindingKind] = &[
78	        FindingKind::HerdrVersionUnsupported,
79	        FindingKind::ObserveFailed,
80	        FindingKind::UnregisteredHerdrPane,
81	        FindingKind::HerdrPaneMissing,
82	        FindingKind::TmuxSessionMissing,
83	        FindingKind::ServerWedged,
84	        FindingKind::ServerDown,
85	        FindingKind::NoSessionOfRecord,
86	        FindingKind::SessionOfRecordMissing,
87	        FindingKind::TuiForeignSession,
88	        FindingKind::ShownDrivenMismatch,
89	        FindingKind::StraySession,
90	    ];
91	
92	    /// The kind's stable kebab-case code, which is also its JSON form. A code is stable
93	    /// once merged (ADR-0021 section 9).
94	    pub const fn code(self) -> &'static str {
95	        match self {
96	            FindingKind::HerdrVersionUnsupported => "herdr-version-unsupported",
97	            FindingKind::ObserveFailed => "observe-failed",
98	            FindingKind::UnregisteredHerdrPane => "unregistered-herdr-pane",
99	            FindingKind::HerdrPaneMissing => "herdr-pane-missing",
100	            FindingKind::TmuxSessionMissing => "tmux-session-missing",
101	            FindingKind::ServerWedged => "server-wedged",
102	            FindingKind::ServerDown => "server-down",
103	            FindingKind::NoSessionOfRecord => "no-session-of-record",
104	            FindingKind::SessionOfRecordMissing => "session-of-record-missing",
105	            FindingKind::TuiForeignSession => "tui-foreign-session",
106	            FindingKind::ShownDrivenMismatch => "shown-driven-mismatch",
107	            FindingKind::StraySession => "stray-session",
108	        }
109	    }
~~~~

**E6: `crates/holler-pane/src/findings.rs:153-182`** — `FixState`: what `--fix` did or could do.

~~~~text
153	/// What `--fix` did, or could do, about a finding.
154	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
155	pub enum FixState {
156	    /// `--fix` does not repair it: every kind but the mismatch, and a mismatch whose server
157	    /// is not healthy or does not have the session of record.
158	    NotFixable,
159	    /// `--fix` would repair it, and this run did not ask to.
160	    Fixable,
161	    /// `--fix` was asked for and left it alone: the orchestrator's pane is switched only by a
162	    /// run that names it.
163	    Skipped,
164	    /// `--fix` repaired it: the TUI was seen showing the session of record afterwards.
165	    Fixed,
166	    /// `--fix` tried and the TUI was not seen showing the session of record; the finding's
167	    /// `fix_error` says why.
168	    Failed,
169	}
170	
171	impl FixState {
172	    /// The state's kebab-case name, which is also its JSON form.
173	    pub const fn as_str(self) -> &'static str {
174	        match self {
175	            FixState::NotFixable => "not-fixable",
176	            FixState::Fixable => "fixable",
177	            FixState::Skipped => "skipped",
178	            FixState::Fixed => "fixed",
179	            FixState::Failed => "failed",
180	        }
181	    }
182	}
~~~~

**E7: `crates/holler-pane/src/error.rs:97-123`** — The closed error codes and their wire strings.

~~~~text
97	    /// The kebab-case wire string of this code.
98	    pub(crate) const fn as_str(self) -> &'static str {
99	        match self {
100	            PaneCode::NotImplemented => "not-implemented",
101	            PaneCode::Usage => "usage",
102	            PaneCode::GridAmbiguous => "grid-ambiguous",
103	            PaneCode::GridOutOfRange => "grid-out-of-range",
104	            PaneCode::CommandNotArgv => "command-not-argv",
105	            PaneCode::EnvNameInvalid => "env-name-invalid",
106	            PaneCode::GenerationConflict => "generation-conflict",
107	            PaneCode::ProbeFailed => "probe-failed",
108	            PaneCode::ProfileConflict => "profile-conflict",
109	            PaneCode::ProfileNotFound => "profile-not-found",
110	            PaneCode::ProfileExists => "profile-exists",
111	            PaneCode::ProfileHasLivePanes => "profile-has-live-panes",
112	            PaneCode::PaneNotInProfile => "pane-not-in-profile",
113	            PaneCode::PaneInOtherProfile => "pane-in-other-profile",
114	            PaneCode::ProfileSecretRefused => "profile-secret-refused",
115	            PaneCode::HerdrVersionUnsupported => "herdr-version-unsupported",
116	            PaneCode::Timeout => "timeout",
117	            PaneCode::PaneNotFound => "pane-not-found",
118	            PaneCode::SessionNotFound => "session-not-found",
119	            PaneCode::StoreCorrupt => "store-corrupt",
120	            PaneCode::Unavailable => "unavailable",
121	            PaneCode::ProfileDrift => "profile-drift",
122	        }
123	    }
~~~~

**E8: `crates/holler-pane/src/error.rs:140-156`** — `ALL_CODES`, the public list acceptance 3 reads.

~~~~text
140	/// Every closed code, as its kebab-case wire string, in the epic's order.
141	///
142	/// The set is closed: the invariants of this crate (one variant per code, unique,
143	/// valid under [`is_valid_code`]) are tested against it, and no story edits it.
144	/// `profile-drift` is listed for convenience: it is a reconcile finding kind
145	/// (#647/#665 put it in `findings.rs`), not an error a port returns.
146	pub const ALL_CODES: &[&str] = &CLOSED_CODES;
147	
148	const CLOSED_CODES: [&str; CODE_COUNT] = {
149	    let mut codes = [""; CODE_COUNT];
150	    let mut i = 0;
151	    while i < CODE_COUNT {
152	        codes[i] = PaneCode::ALL[i].as_str();
153	        i += 1;
154	    }
155	    codes
156	};
~~~~

**E9: `crates/holler-pane/src/error.rs:217-246`** — `ErrorClass` and its exit codes (usage 2, refusal 3, failure 1; success 0).

~~~~text
217	/// What kind of error a code stands for, which decides the exit code of a `pane` or
218	/// `profile` verb that ends with it (ADR-0021 section 9). [`class_of`] gives the
219	/// class of a code.
220	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
221	pub enum ErrorClass {
222	    /// The request is malformed: a missing or conflicting argument, a bad name, a
223	    /// params object that does not decode (`usage`). Exit 2.
224	    Usage,
225	    /// The request was understood and declined, working as designed, so nothing was
226	    /// wrong with the system (`pane-in-other-profile`, `grid-ambiguous`, every open
227	    /// code). Exit 3.
228	    Refusal,
229	    /// Something went wrong while doing the work: a timeout, an unreachable hub or
230	    /// adapter, a conflict between writers (`timeout`, `unavailable`,
231	    /// `generation-conflict`). Exit 1.
232	    Failure,
233	}
234	
235	impl ErrorClass {
236	    /// The exit code of a verb that ends with an error of this class, the same in
237	    /// text and JSON mode: 2 usage, 3 refusal, 1 runtime failure (ADR 0003's exit
238	    /// codes; success is 0 and has no class).
239	    pub const fn exit_code(self) -> i32 {
240	        match self {
241	            ErrorClass::Usage => 2,
242	            ErrorClass::Refusal => 3,
243	            ErrorClass::Failure => 1,
244	        }
245	    }
246	}
~~~~

**E10: `crates/holler-cli/src/output.rs:1-58`** — The envelope, its `schema_version`, the exit codes and the `--format` / `--json` rule (section 9 of the guide).

~~~~text
1	//! What the `holler pane` and `holler profile` verbs print (story #670, epic #633): one module
2	//! decides it, so every verb answers in one shape and each verb supplies only its data.
3	//!
4	//! - **Text mode** (the default): the data of a successful verb goes to `out`; an error goes to
5	//!   `err` as `error: <message>`, with nothing on `out`.
6	//! - **JSON mode** (`--format=json`, or `--json`): exactly one envelope goes to `out` per result,
7	//!   an error included, and nothing goes to `err`. `pane watch` is the one stream: one envelope
8	//!   per line ([`emit_stream`]).
9	//!
10	//! The exit code is the same in both modes: `0` ok, `1` runtime failure, `2` usage, `3` refusal
11	//! (ADR-0021 section 9). The class of the error's code decides it, through
12	//! `holler_pane::error::class_of`, and this module keeps no table of its own. An error coded
13	//! `usage` exits 2 however it got here, so a run-time guard of `holler-pane` (a bad pane name,
14	//! `--command-json` that is not JSON) is a usage error and not a runtime failure; a request
15	//! understood and declined (`pane-in-other-profile`, `grid-ambiguous`) exits 3; something that
16	//! went wrong while doing the work (`timeout`, `unavailable`) exits 1.
17	//!
18	//! Writing goes through an injected [`Sink`], so a test captures both streams. Nothing here exits
19	//! the process: every function returns the code and `main.rs`, the one file allowed to exit,
20	//! applies it (`scripts/lint.sh` check 2).
21	//!
22	//! A write that fails (a closed pipe) turns an exit 0 into 1 and ends a stream, so a verb never
23	//! reports success for output nobody received.
24	
25	use std::io::{self, Write};
26	
27	use clap::error::ErrorKind;
28	use clap::ValueEnum;
29	use holler_pane::error::{class_of, is_valid_code};
30	use holler_pane::{PaneError, Ports};
31	use serde::Serialize;
32	
33	/// The envelope's `schema_version` (epic #633, "Output"). A change to the envelope's shape bumps it.
34	pub const SCHEMA_VERSION: u32 = 1;
35	
36	/// The two subcommands whose usage errors are envelopes in JSON mode (decision 1(d) of #670).
37	/// Every other verb keeps its own usage-error output, as ADR 0003 requires.
38	const ENVELOPE_NAMESPACES: [&str; 2] = ["pane", "profile"];
39	
40	/// The output format: the global `--format`, and what `--json` means.
41	#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
42	pub enum Format {
43	    /// Human-readable text (the default).
44	    Text,
45	    /// One JSON envelope per result.
46	    Json,
47	}
48	
49	/// How the global `--format` and `--json` resolved.
50	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
51	pub struct FormatChoice {
52	    /// The format every verb prints in.
53	    pub format: Format,
54	    /// `--format=json` was given itself (`--json` alone does not set it). `roster` prints the
55	    /// envelope only for this, so `roster --json` keeps its legacy shape (ADR 0003: the `--json`
56	    /// shape is part of the surface); the `pane` and `profile` verbs treat either flag the same.
57	    pub json_explicit: bool,
58	}
~~~~

**E11: `crates/holler-pane/src/grid.rs:1-55`** — `GridPos::parse`: the accepted forms and the two refusal codes (acceptance 5).

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
15	use std::fmt;
16	
17	use serde::de::{self, Deserializer};
18	use serde::ser::{SerializeStruct, Serializer};
19	use serde::{Deserialize, Serialize};
20	
21	use crate::error::{excerpt, PaneError};
22	
23	/// What every refusal of a grid position tells the user to write instead.
24	const FORMS: &str = "write r<row>c<col>, c<col>r<row> or <row>,<col>";
25	
26	/// What every out-of-range refusal says about the bounds.
27	const BOUNDS: &str = "rows and columns run from 1 to 65535";
28	
29	/// A cell of the layout grid, 1-based: `row` counts down, `col` counts across.
30	///
31	/// The fields are public so a cell can be written `GridPos { row: 2, col: 1 }`; a
32	/// zero there is not a cell, and [`GridPos::parse`] and the serde reader both
33	/// refuse it.
34	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
35	pub struct GridPos {
36	    /// The row, from 1.
37	    pub row: u16,
38	    /// The column, from 1.
39	    pub col: u16,
40	}
41	
42	impl GridPos {
43	    /// Parse `r2c1`, `c1r2` or `2,1` (see the module docs for the exact rules).
44	    pub fn parse(text: &str) -> Result<GridPos, PaneError> {
45	        let lower = text.trim_ascii().to_ascii_lowercase();
46	        let (row, col) = split_cells(&lower).ok_or_else(|| PaneError::GridAmbiguous {
47	            what: format!("{} ({FORMS})", excerpt(text)),
48	        })?;
49	        match (coordinate(row), coordinate(col)) {
50	            (Some(row), Some(col)) => Ok(GridPos { row, col }),
51	            _ => Err(PaneError::GridOutOfRange {
52	                what: format!("{} ({BOUNDS})", excerpt(text)),
53	            }),
54	        }
55	    }
~~~~

**E12: `crates/holler-cli/tests/docs_cli_test.rs:1-31`** — The existing docs test whose rules the new guide must obey, and the style the new test copies (`holler_cli::Cli` is public).

~~~~text
1	#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
2	//! Docs doc-test (issue #155 §6): every `holler …` command the documentation
3	//! shows must parse against the real clap tree.
4	//!
5	//! Two sources, across `README.md` and `docs/**/*.md`:
6	//! 1. lines inside fenced code blocks (`bash`/`sh`/`shell`/`console`/`text`/
7	//!    untagged) that start with `holler ` (an optional `$ ` prompt is
8	//!    stripped),
9	//! 2. inline code spans `` `holler …` `` anywhere in the prose (a few
10	//!    `v2.md` §10.1 wire-mapping rows, e.g. `hub query <target> caps`).
11	//!    The bulk of the surface is the `v2.md` §10 code block (the ADR 0003
12	//!    table reproduced verbatim, #148), which arrives via source (1).
13	//!
14	//! Placeholders are normalised (`<x>` → `x`, `[optional]` dropped, `a|b` →
15	//! `a`, `…` dropped, a trailing annotation after two spaces / ` (` / ` — `
16	//! cut). A command whose leaf verb is in `cli-surface.pending.txt` is
17	//! tolerated (specified, not yet implemented) and reported, never silently
18	//! skipped.
19	
20	mod fixture;
21	
22	use std::path::{Path, PathBuf};
23	
24	use clap::error::ErrorKind;
25	use clap::Parser;
26	use fixture::{pending_leaves, split_args};
27	use holler_cli::Cli;
28	
29	fn workspace_root() -> PathBuf {
30	    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
31	}
~~~~

**E13: `crates/holler-cli/Cargo.toml:74-98`** — How tests are declared (the crate sets `autotests = false`); the new test needs its own `[[test]]` entry.

~~~~text
74	
75	# Selftests for the shared test harness (story #138). Exercises the pieces of
76	# `tests/support/mod.rs` that are exercisable before the hub/body stories land
77	# (StateDir drop, wait_for timeout, kill_tree).
78	[[test]]
79	name = "support_selftest"
80	path = "tests/support_selftest.rs"
81	
82	# CLI-surface fixture and docs doc-test (issue #155 §5/§6). Both read
83	# tests/fixtures/cli-surface*.txt via the shared tests/fixture/mod.rs.
84	[[test]]
85	name = "cli_surface_test"
86	path = "tests/cli_surface_test.rs"
87	
88	[[test]]
89	name = "docs_cli_test"
90	path = "tests/docs_cli_test.rs"
91	
92	# `holler hub serve` + `holler hub status` e2e (story #143): the hub's
93	# WebSocket listener, instance lock, Unix-domain control socket, and status.
94	[[test]]
95	name = "hub_serve_test"
96	path = "tests/hub_serve_test.rs"
97	
98	# `holler hub token …` e2e (story #163): the CLI's thin layer over the hub's
~~~~

**E14: `docs/README.md:24-60`** — The index the guide is added to ("Guidance" section).

~~~~text
24	## Deployment
25	
26	- [**deploy.md**](deploy.md) — why the hub only ever binds loopback, the supported Tailscale
27	  tailnet path (and other TLS-terminating proxies), and how a remote admin client (`--server URL`
28	  on `roster`/`say`/`hub status`/etc., ADR 0020) reaches the hub the same way a body already does.
29	- [**releasing.md**](releasing.md) — the release process narrative: version bumps, the CHANGELOG
30	  entry structure, the manual acceptance gates, and building for a platform you don't have locally.
31	  [**release-checklist-template.md**](release-checklist-template.md) is the copy-per-release
32	  checklist that follows it.
33	
34	## Guidance
35	
36	- [**orchestrating.md**](orchestrating.md) — the orchestrator pattern: dispatch with `say`, don't poll `roster`, block on a deterministic `wait`, redirect with `interrupt`, resolve with `answer` (issue #142).
37	- [**guidance-effective-agent-monitoring.md**](guidance-effective-agent-monitoring.md) — how to monitor a body/agent effectively (operating guidance, not a protocol or an ADR).
38	- [**setup-wizard.md**](setup-wizard.md) — watching multiple attach-mode sessions in one local [Herdr](https://herdr.dev) workspace: plain `opencode attach` panes (not `herdr-mirror`), and the config-driven setup wizard (installs Herdr itself, then optionally configures and launches a workspace) that automates it.
~~~~

**E15: `docs/orchestrating.md:108-138`** — The interim section acceptance 8 rewrites (start of the section and its deletion sentence).

~~~~text
108	## Restarting a pane on another model (interim)
109	
110	Until holler pane relaunch with --model (#644) and the pfleet adoption (#653) land, `scripts/pane-restart`
111	restarts one fleet pane's OpenCode TUI on a new model. Run it on the host that owns the pane's tmux session
112	(the fleet host), from any shell except the pane being restarted:
113	
114	```bash
115	scripts/pane-restart c3r1 --model anthropic/claude-sonnet-5-5 --effort medium \
116	  --key-file '$HOME/.holler-oc/keys/anthropic.key'      # add --dry-run first to see the plan
117	```
118	
119	It prints one line: `c3r1: OK model=… session=…`, `c3r1: REFUSED: …` (exit 1, nothing changed) or
120	`c3r1: FAIL step N (name): …` (exit 2, with the `pane.env` backup path). It refuses the MO pane (`c4r2`) and
121	any pane that is not idle. Pause the watchdog yourself first if you do not want it acting on the pane meanwhile.
122	Do not type `tmux send-keys` by hand for this: text sent without `Enter` sits unsubmitted in the TUI prompt
123	(the 2026-10-10 failure that motivated the script). This section is removed when #644 and #653 have merged.
124	
125	A pane running **Claude Code** is detected, not named: the script looks for a live Claude Code process whose
126	record in `~/.claude/sessions/<pid>.json` names the pane's tmux session (`hj-c3r1`). Such a pane takes a Claude
127	model alias or name and an effort level, and none of the OpenCode options:
128	
129	```bash
130	scripts/pane-restart c3r1 --model opus --effort high --dry-run   # alias (opus, sonnet, fable) or full name, optional [1m]
131	scripts/pane-restart c3r1 --effort xhigh                         # an omitted --model or --effort keeps its old value
132	scripts/pane-restart c3r1 --model sonnet --fresh                 # a new session instead of resuming the old one
133	```
134	
135	It refuses unless the record says `idle` and the screen shows neither `esc to interrupt`, a question
136	(`Enter to select`) nor a running agent row (`◯ name`). It stops the process (`/exit`, then one Ctrl-C),
137	starts `claude` again in the same directory with the flags it had, the new model and effort, and
138	`--resume <session id>`, and waits for the new process to report `idle`. A session that has no saved
~~~~

**E16: `scripts/pane-restart:1-24`** — The header of the fallback script the guide names as the rollback.

~~~~text
1	#!/usr/bin/env bash
2	# pane-restart: restart one fleet pane's OpenCode or Claude Code TUI on a new model, in its existing tmux session.
3	# A stop-gap (issue #746) until `holler pane relaunch --model` (#644) and the pfleet adoption (#653) land.
4	#
5	#   pane-restart PANE --model PROVIDER/ID [--effort E] [--key-file PATH] [--project DIR] [--agent KEY] [--dry-run]
6	#   pane-restart PANE [--model ALIAS|NAME] [--effort E] [--fresh] [--dry-run]            (a Claude Code pane, see below)
7	#
8	# PANE is cCrR (the tmux session is hj-PANE, derived, never typed). Steps, in order; the first failure stops the
9	# script and names the step:
10	#   1 refuse unless the pane is idle and reachable   2 back up pane.env   3 set the OC_ lines in pane.env
11	#   4 stop the TUI (/exit, then one Ctrl-C)          5 start it fresh      6 wait for the server
12	#   7 wait for the new session to register (and repair pfleet's record with --set-registration)        8 the pane answers on the new model   9 pane-ready says READY
13	# Every text typed into a pane is followed by Enter: in the same `tmux send-keys` call for the shell line, as a
14	# separate call after a short pause for `/exit` (the TUI's autocomplete popup swallows an Enter sent with it). Only the key file's PATH is written to
15	# pane.env; the file is never read or printed. Exit: 0 ok, 1 refused (nothing changed), 2 failed after a change.
16	# --mo restarts c4r2, the MO pane: it edits opencode-mo/opencode.json and agents/mo.md instead of pane.env, launches
17	# with OC_SESSION_NAME=mo and no --agent, needs --key-file, accepts a busy pane, and skips pane-ready.
18	# A pane that is already fully down is started without the stop step; that needs --project.
19	# A Claude Code pane (issue #784) is found, not named: a live Claude Code process whose per-process record in
20	# ~/.claude/sessions/<pid>.json names the pane's tmux session. It is restarted through its own command line, with no
21	# pane.env and no oc-holler: refuse unless the record says idle and the screen shows neither 'esc to interrupt', a
22	# question ('Enter to select') nor a running agent row ('◯ name'); stop it (C-u, /exit, then two Ctrl-C); start `claude` again in the same
23	# directory with the flags it had, the new --model and/or --effort (an omitted one keeps its old value), and
24	# `--resume <its session id>` (--fresh starts a new session instead; a session with no saved conversation can only be
~~~~

**E17: `crates/holler-cli/src/pane/doctor.rs:1-38`** — `pane doctor`: exit codes, the periodic pass, text mode and the `--fix` flag.

~~~~text
1	//! `holler pane doctor [PANE] [--fix] [--profile NAME]` (story #647): run one reconcile pass
2	//! (`holler_pane::reconcile`) over the panes in scope and print its report.
3	//!
4	//! - **Exit codes.** A pass that completes exits 0, with or without findings: findings are
5	//!   kinds in the report, not errors (ADR-0021 section 9), and an envelope cannot carry data
6	//!   with a failure, so a script reads `data.findings`. The verb fails only when it cannot
7	//!   run the pass: a bad `PANE` or `--profile` value is `usage` (2); a pane or profile that
8	//!   does not exist, or a pane outside the profile, is a refusal (3); a pane store or profile
9	//!   scope that cannot be read is a failure (1).
10	//! - **The periodic pass.** There is no hub timer (ADR-0021 section 12): a scheduler runs
11	//!   `holler pane doctor --format=json`, without `--fix`, as often as wanted, and each run
12	//!   records what changed.
13	//! - **Text mode** prints one line each: Herdr's version, each host's recorded Herdr API
14	//!   version, each finding (`<pane> <pos> <kind>: <message>`, then what `--fix` did and the
15	//!   command to run), and a summary. Untrusted values are quoted (`findings::quoted`), so no
16	//!   control sequence reaches the terminal; JSON carries the raw values.
17	
18	use clap::Args;
19	use holler_pane::findings::{quoted, Finding, FixState};
20	use holler_pane::reconcile::{reconcile, ReconcileRequest, Report};
21	use holler_pane::{PaneError, PaneName, Ports, ProfileName};
22	use holler_proto::clock::now_millis;
23	
24	use super::args::ProfileOpt;
25	use crate::output::{emit, ErrorBody, VerbCtx};
26	
27	/// Check panes against Herdr, tmux and the harness and report what differs.
28	#[derive(Args, Debug)]
29	pub struct PaneDoctor {
30	    /// Check only this pane (default: every pane in scope).
31	    #[arg(value_name = "PANE")]
32	    pub pane: Option<String>,
33	    /// Repair what the record decides (selects the session of record; never changes it).
34	    #[arg(long)]
35	    pub fix: bool,
36	    #[command(flatten)]
37	    pub profile: ProfileOpt,
38	}
~~~~

**E18: `crates/holler-cli/src/pane/watch.rs:1-9`** — `pane watch`: NDJSON stream, `--since`, `--until-idle`.

~~~~text
1	//! `holler pane watch` (story #643, epic #633): the pane store's change feed as a stream,
2	//! one line per change, NDJSON in JSON mode.
3	//!
4	//! "Each change once" is the port's contract, not a dedupe here: the verb prints every
5	//! change the stream yields, in order. From cursor 0 that is the current state (one put
6	//! per pane) and then every later change; from `--since N`, every change after N
7	//! (`holler_pane::ports::Watch`). The verb reads only `ProfileScope::resolve` (with
8	//! `--profile`, before the stream opens) and `PaneStore::watch`, and it writes nothing.
9	
~~~~

**E19: `crates/holler-cli/src/pane/args.rs:1-34`** — The shared flag groups (`--profile`, `--spec-only`, the spec flags including `--agent`).

~~~~text
1	//! The flag groups the `holler pane` verbs share (story #670, epic #633), defined once:
2	//!
3	//! - [`SpecFlags`]: one flag per field of a pane spec, used by `launch` and `relaunch` (and by
4	//!   the profile spec). Every value stays a string at clap time; [`SpecFlags::validate`] types
5	//!   them, with the guards of `holler-pane` and their stable error codes, so a bad `--grid` is a
6	//!   refusal with a code a script can match (exit 3) and not a clap usage error.
7	//! - [`ProfileOpt`]: `--profile NAME`, on every `pane` verb but `import`, and on `roster`,
8	//!   `say`, `interrupt` and `answer`.
9	//! - [`SpecOnly`]: `--spec-only`, on `launch`, `relaunch` and `close`.
10	//!
11	//! This is all that #670 declares for the verbs. The positionals and flags that belong to one
12	//! verb (`pane get PANE`, `pane doctor --fix`) are declared in that verb's own file by its owning
13	//! story.
14	
15	use clap::Args;
16	use holler_pane::pane::PaneRole;
17	use holler_pane::{AgentKey, Argv, EnvVarName, GridPos, PaneError};
18	
19	/// `--profile NAME`: scope the verb to a profile.
20	#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
21	pub struct ProfileOpt {
22	    /// Act on the panes of this profile; a named pane must belong to it.
23	    #[arg(long, value_name = "NAME")]
24	    pub profile: Option<String>,
25	}
26	
27	/// `--spec-only`: edit a profile's spec and change nothing live.
28	#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
29	pub struct SpecOnly {
30	    /// Edit the profile's spec for the pane and change nothing live.
31	    #[arg(long, requires = "profile")]
32	    pub spec_only: bool,
33	}
34	
~~~~

**E20: `docs/handoffs/664-brief.md` sections 3 to 7 (the step table, answers outside it, outcomes, `--dry-run`, `data`) and the "Decisions on O1 to O5" and "Decision on O6" sections** — as committed on branch `issue-664-implementation` (worktree `0664b-profile-apply`, commit `39ad11a`), taken by the operator on 2026-10-11. Quoted for the `profile apply` reference. A sentence in the quote that says "Not decided" or "NOT DECIDED" is about earlier wording of that brief: every point in it is decided.

~~~~text
**3. The step table.** One row per spec pane, in `(workspace, row, col)` order; the first row that matches applies:

| # | Situation | Step | Live effect | Registry effect |
|---|---|---|---|---|
| a | the pane has a record that belongs to **another profile**, and no `--take-over` | `refused` (`pane-in-other-profile`) | none | none |
| b | **no record** | `create` through the launch transaction (it refuses an occupied cell or an existing session through its own checks: that is a `refused` or `failed` step per 5) | launch's | launch's |
| c | record, **healthy**, matches its spec (`diff_spec` empty) | `adopt` | none (never touched) | set `Pane.profile` to P if not already (with `--take-over` from another profile: leave then join, two writes) |
| d | record, **healthy**, **differs** from its spec | `reported` with the `PaneDiff` | none (apply never kills or rewrites a healthy pane; the operator uses `launch`/`relaunch --profile P`) | with `--take-over` from another profile: leave then join, otherwise none |
| e | record, **not healthy**, role **orchestrator** | `refused` (`orchestrator-not-relaunched`) | none | none, **even under `--take-over`** (no profile writes happen for a refused pane) |
| f | record, **not healthy**, role agent | `relaunch` through `tx_launch::relaunch` (`grid_given: false`, `profile: Some(P)`) | relaunch's, **after** the profile move below | with `--take-over` from another profile: **leave then join first (two writes, exactly as rows c and d), then the relaunch**, which then sees a record that already names P. `tx_launch::relaunch` is called unchanged and never refuses `pane-in-other-profile` for this step, because the record no longer names another profile when it runs. If the relaunch then fails, the pane stays in P (the intended end state) and the step is `failed`; apply does not compensate the profile move. Without a take-over (the record already names P, or none) there is no profile move, and the relaunch's own write sets `Pane.profile` to P |

Row order matters: row a is checked first and fires only for another profile **without** `--take-over`; with `--take-over` the pane falls to rows b to f, and rows c, d and f add the profile move described in the table, **always before any live effect of the same step**; row e never writes.

**4. Per-pane answers outside the table.** A step that the launch transaction answers with a **refusal** (exit class 3, for example `probe-failed`, `pane-exists`) is a `refused` step carrying that code. A step answered with `usage` (exit class 2) for a per-pane reason (a relaunch's refusal to move a pane when the spec's cell or directory differs from the record's, a spec whose port policy is not `fixed:<port>`) is also a `refused` step carrying a named code, not a run failure. **`create` without `--herdr-session`** is checked **before any step runs**: if the plan contains a `create` and the flag is absent, the whole verb exits 2 (`usage`) and no step runs, in a dry run too.

**5. The exhaustive outcome set and what each contributes.** Every step ends in exactly one of: `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`.
- `refused` and `reported` do **not** stop the run and do **not** change the exit code.
- `failed` is any failure-class answer (exit class 1: `unavailable`, `timeout`, a failed `health` call, a store conflict) and **stops the run at that step**; steps after it are not attempted and are listed as `skipped`.
- Run exit code: **1** if any step is `failed`; **2** for the up-front `usage` case in 4; otherwise **0**. This is the doctor form (ADR-0021 section 9's apply row is reworded to say so).

**6. `--dry-run`** prints the same table as the plan and changes nothing: it runs no probe, issues no split, writes nothing, and performs no health call that has a side effect; it does the read-only health and snapshot observations needed to choose each row.

**7. `data`** is the object `{"profile": NAME, "dry_run": bool, "steps": [{pane, action, outcome, code?, diff?}]}`, embedding `PaneDiff` unchanged.

**8. Flags:** `--herdr-session NAME` (as `pane launch` has it), `--take-over`, `--dry-run`, `--format`.
## Decisions on O1 to O5 (final, 2026-10-11, taken under the operator's standing rules; they replace "Not decided" above)

- **O1 (row f under `--take-over`): leave then join first, then relaunch.** The profile move (leave Q, join P: two writes, as rows c and d) happens before the relaunch, so `tx_launch::relaunch` (unchanged) sees a record that names P and does not refuse `pane-in-other-profile`. If the relaunch then fails, the pane stays in P (the intended end state), the step is `failed`, and apply writes nothing to compensate. Row f's table text says the same.
- **O2 (a shown session that cannot be read): `Ok(None)` and `Err(_)` both mean "cannot tell" and do not make a pane unhealthy.** Only `Ok(Some(s))` with `shown_differs` true counts as a mismatch. Reason: apply never kills a pane because its TUI shows the home screen; an unreadable reading is reported in the step's detail, not acted on.
- **O3 (#802 vs this story):** this story is written against the test kit as it is on `main` when the run starts, seeding servers through one rig helper (`profile_verbs/rig.rs`), so that if #802 merges first, adapting is a one-line rename of that helper and nothing else. Both stories edit `docs/adr/ADR-0021.md` in different sections (this one: the definition of "adopt" and section 9's apply row); the merge of main resolves it.
- **O4 (a take-over whose join fails after its leave landed): leave the pane in no profile and say so.** The step is `failed`, its error text says the pane left Q and is in no profile, and the run stops (section 5). Apply does not write a compensating re-join of Q (a further write that can fail too); the operator re-runs apply, which then sees a pane with no profile and takes it through rows b to f.
- **O5 (a budget over the whole run): none.** The bound is per step plus one up-front observation, recorded in the module doc of `crates/holler-pane/src/tx_apply.rs` (section 9) and asserted by a test over that file.

## Decision on O6 (final, 2026-10-11; overrides every "O6 ... open / NOT DECIDED" sentence above)

- **O6: option (a).** A run that ends with a `failed` step exits 1 and the verb prints **only the failure**: in `--format=json` the failure envelope (`ok` false, `data` null, `error.code` the failed step's error code, `error.message` naming the pane, the step's action and how many later steps were skipped); in text mode one `error: <same message>` line on stderr and nothing on stdout. No plan table and no `skipped` list is printed at exit 1, for a dry run too (a dry run with a `failed` step prints no plan). This narrows §6's "prints the same table as the plan" to runs that do not fail, and C2 reads accordingly. ADR-0021's decision 6, `output.rs` and the test kit's envelope checker are **not** changed.
- **Where the `failed` / `skipped` steps are asserted:** the tests behind C14's entries 17, 21, 22, 23, 25 and 29 read the steps from `tx_apply`'s own returned result (an in-process test over the fakes), not from the verb's output. The verb-level test for the same situations asserts exit 1, the failure envelope's code and the message text, and that stdout is empty in text mode.
- The `error.message` format is pinned: `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`.

~~~~

**E21: issue #654 body (`gh issue view 654`, 2026-10-11)** — the cutover procedure and its acceptance.

~~~~text
Part of epic #633. **Operator-run; spends no model tokens beyond what the panes use.**

## Scope
Staged and reviewed before anything changes: run `holler pane import --dry-run` on the live fleet and read every finding; fix drift with the old tools; import (a real import also creates the profile `fleet` from today's panes and their layout, #650 (amended 2026-10-08, review)); run `doctor` and the roster until SHOWN equals DRIVEN for every pane; cut one pane over to `launch`/`reset` and watch a day; then the rest. The old per-pane files are kept (read-only) for one release. Record each step in `docs/handoffs/<this issue>-cutover/decisions.md`.

## Acceptance
- Seven days with no invisible session: every day, `doctor` reports no SHOWN/DRIVEN mismatch and no stray session older than an hour; the operator signs off.

Depends on: #649, #650, #652, #655 (the Herdr server moved to the hub's machine).

~~~~

**E22: issue #802 body, "Operator decisions" 4 to 7 (`gh issue view 802`, 2026-10-11)** — the server-in-tmux contract section 2 of the guide describes.

~~~~text
4. **The harness server runs inside the pane's tmux session, started through the host adapter.** Operator rules: change signatures freely while in development; one source of truth; a wedged process can always be killed with extreme prejudice. `HostPort` (`run` / `stop_owned`: TERM, a shared grace, then KILL) is the one place that starts and stops a pane's processes. Consequences: `HarnessPort::serve` becomes `serve_command` (it builds the command and starts nothing); the OpenCode adapter no longer spawns `opencode serve` itself; `harness.pid` stops being a second source of truth. This absorbs #695.
5. **(review round 1, decided by the maintainer's agent under the operator's rules) The agent check after start** (#642 part 3 decision 3) stays possible: add a **required** method `HarnessPort::verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError>` (no default: a harness that starts servers must say whether the running one honours the spec). The engine calls it **after the health wait and before `create_session`**; a refusal goes through the engine's existing rollback, which now stops the server because the server is a process of the session. #642 part 3 implements it for OpenCode; here the OpenCode implementation checks nothing beyond `Ok(())` for a spec with no agent (a spec with an agent never reaches it, because `serve_command` refuses first). #642 part 3's decisions 3, 5 and 7 are amended accordingly (done on that issue).
6. **(review round 1) The server's working directory has one source: the pane's spec** (`plan.spec.host.cwd`). `HostPort::run` gains the directory: `run(&self, name: &PaneName, cwd: &str, argv: &Argv)`; the tmux adapter passes it as `new-window -c <cwd>` (escaped like every value the adapter did not write; an empty or relative `cwd` is `usage`). The engine passes `plan.spec.host.cwd` for **both** the spec's command and the server, so a session reused after a directory change no longer starts a program in the old directory. `OpenCodeConfig.workdir` is deleted (its only reader was `serve`), and ADR-0021 section 2 fact 6 is rewritten to say the directory comes from the pane spec through `HostPort::run`. A session keeps the directory it was made with for the shell window only; nothing relies on it.
7. **(review round 1) Every refusal stays in the plan.** `serve_command` is pure, so the engine calls it **in the plan** (in `Plan::observe`, before the probe), keeps the argv there, and the act passes exactly that argv to `host.run` after `ensure_session`. An `agent-unsupported` refusal is therefore exit 3 with nothing written, no rollback and no reconcile step, like `pane-exists` and `port-in-use`.

~~~~

**Added in brief review round 1 (E23 to E34).** Cut the same way: a file from `263a2d7` (`git show 263a2d7:PATH`; `263a2d7` is `origin/main`), an issue body from `gh issue view N --json body --jq .body` on 2026-10-11, with the line numbers of that output; never retyped.

**E23: `crates/holler-pane/src/error.rs:248-303`** — `class_of`, the one place that gives a code its class: `herdr-version-unsupported` is a refusal (exit 3), `profile-drift` a failure (exit 1), and every well-formed code outside `ALL_CODES` is an open code and always a refusal (B-2, NV-1, NV-2).

~~~~text
248	/// The class of the error coded `code`: the one place that decides which code is a
249	/// refusal and which a runtime failure. ADR-0021 section 9 has the table, with the
250	/// reason for every row. The CLI's output module calls this, and so will the test
251	/// kit (#638); neither keeps a table of its own.
252	///
253	/// It takes the code, not a [`PaneError`], because a code is what an envelope
254	/// carries. A caller that holds a `PaneError` passes [`PaneError::code`]:
255	/// `class_of(error.code())`. (It is not `PaneError::classify`, a crate-private
256	/// helper that only tells a closed code from an open one.)
257	///
258	/// - A **closed** code has the class of its arm below. The `match` names every
259	///   closed code and has no catch-all arm, so a closed code added without a class
260	///   does not compile.
261	/// - Any other **well-formed** code is an open code, carried by
262	///   [`PaneError::Refused`]: a refusal, always. A verb that reports a runtime
263	///   failure uses a closed failure code.
264	/// - A **malformed** code is a failure: it cannot be trusted, the same rule that
265	///   turns a garbled wire reply into `unavailable`.
266	pub fn class_of(code: &str) -> ErrorClass {
267	    let Some(closed) = PaneCode::parse(code) else {
268	        return if is_valid_code(code) {
269	            ErrorClass::Refusal
270	        } else {
271	            ErrorClass::Failure
272	        };
273	    };
274	    match closed {
275	        PaneCode::Usage => ErrorClass::Usage,
276	        // Understood and declined: a guard, a policy or a gate said no, the name is
277	        // taken, or the request named something that does not exist.
278	        PaneCode::GridAmbiguous
279	        | PaneCode::GridOutOfRange
280	        | PaneCode::CommandNotArgv
281	        | PaneCode::EnvNameInvalid
282	        | PaneCode::ProfileSecretRefused
283	        | PaneCode::ProfileExists
284	        | PaneCode::ProfileHasLivePanes
285	        | PaneCode::PaneNotInProfile
286	        | PaneCode::PaneInOtherProfile
287	        | PaneCode::ProbeFailed
288	        | PaneCode::HerdrVersionUnsupported
289	        | PaneCode::ProfileNotFound
290	        | PaneCode::PaneNotFound
291	        | PaneCode::SessionNotFound => ErrorClass::Refusal,
292	        // Went wrong while doing the work: a race between writers, a bound that ran
293	        // out, something unreachable or unreadable, live state that disagrees with
294	        // its spec, or work the verb cannot do yet.
295	        PaneCode::GenerationConflict
296	        | PaneCode::ProfileConflict
297	        | PaneCode::Timeout
298	        | PaneCode::Unavailable
299	        | PaneCode::StoreCorrupt
300	        | PaneCode::NotImplemented
301	        | PaneCode::ProfileDrift => ErrorClass::Failure,
302	    }
303	}
~~~~

**E24: `crates/holler-cli/src/output.rs:125-132` and `150-182`** — The `error` member (`code`, `message`) and the envelope of every `pane` and `profile` result: `data` is `null` when `ok` is false (B-3).

~~~~text
125	/// The `error` member of a failed envelope.
126	#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
127	pub struct ErrorBody {
128	    /// The stable code a script matches on.
129	    pub code: ErrorCode,
130	    /// One line for a person. In JSON mode [`emit`] puts it on one line.
131	    pub message: String,
132	}
~~~~

~~~~text
150	/// The one envelope of every `holler pane` and `holler profile` result in JSON mode.
151	///
152	/// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
153	/// `ok` is false and `error` is `null` when it is true.
154	#[derive(Debug, Serialize)]
155	pub struct Envelope<T> {
156	    pub schema_version: u32,
157	    pub ok: bool,
158	    pub data: Option<T>,
159	    pub error: Option<ErrorBody>,
160	}
161	
162	impl<T> Envelope<T> {
163	    /// A successful result.
164	    pub fn success(data: T) -> Self {
165	        Self {
166	            schema_version: SCHEMA_VERSION,
167	            ok: true,
168	            data: Some(data),
169	            error: None,
170	        }
171	    }
172	
173	    /// A failed result.
174	    pub fn failure(error: ErrorBody) -> Self {
175	        Self {
176	            schema_version: SCHEMA_VERSION,
177	            ok: false,
178	            data: None,
179	            error: Some(error),
180	        }
181	    }
182	}
~~~~

**E25: `docs/adr/ADR-0021.md:505-534`** — The closed and the open codes, and the codes each verb can end with: `herdr-version-unsupported` is an error of `pane launch`, `pane relaunch` and `profile apply` and a reported finding of `pane doctor`; a verb or an adapter declares its own open codes and raises them as `PaneError::Refused` (B-2, NV-1).

~~~~text
505	**Decided: closed and open pane codes.** The closed set (`ALL_CODES`, one `PaneError` variant each) is: `not-implemented`,
506	`usage`, `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `generation-conflict`, `probe-failed`,
507	`profile-conflict`, `profile-not-found`, `profile-exists`, `profile-has-live-panes`, `pane-not-in-profile`,
508	`pane-in-other-profile`, `profile-secret-refused`, `herdr-version-unsupported`, `timeout`, `pane-not-found`,
509	`session-not-found`, `store-corrupt`, `unavailable`, and `profile-drift` (a reconcile finding kind, listed for convenience; it
510	is not an error a port returns). No story edits that list (ruling 3). A verb or adapter declares any other code it owns as a
511	constant in its own file through `RefusalCode::from_static`, which refuses a malformed or closed code at build time, and
512	raises it as `PaneError::Refused`. **Stable** means a code is never renamed or reused for another meaning once it has
513	merged; a code whose condition disappears is retired, not recycled. Every message is one line and never echoes a secret.
514	
515	**Failure modes by verb.** Every verb can also answer `usage`, `unavailable`, `timeout`, `store-corrupt` and, until its
516	story lands, `not-implemented`. "Open (#N)" means codes that story declares as its own constants.
517	
518	| Verb | Codes beyond the common ones |
519	|---|---|
520	| `pane list`, `pane watch` | `profile-not-found`, `pane-not-in-profile` |
521	| `pane get` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile` |
522	| `pane launch`, `pane relaunch` | `grid-ambiguous`, `grid-out-of-range`, `command-not-argv`, `env-name-invalid`, `profile-secret-refused`, `probe-failed`, `herdr-version-unsupported`, `session-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-in-other-profile`; `relaunch` also `pane-not-found`; open (#644) for a name that already has a record, an occupied target cell and a port a server already answers on (`pane-exists`, `grid-occupied`, `port-in-use`); open (#640) for a cell that no single split reaches, `grid-unreachable` |
523	| `pane switch`, `pane reset` | `pane-not-found`, `session-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile`; open (#645): `orchestrator-pane`, `server-unhealthy`, for `switch` `session-of-other-pane`, and for `reset` a session of record that is running a turn or holds a question (`session-busy`, `session-holds-question`) and, with `--first`, a harness that does not queue a prompt (`prompt-unsupported`, the port's default) |
524	| `pane park`, `pane unpark` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `pane-not-in-profile` |
525	| `pane close` | `pane-not-found`, `generation-conflict`, `profile-not-found`, `profile-conflict`, `pane-not-in-profile` (with `--profile`, P holds no spec for the pane) |
526	| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` |
527	| `pane import` | `profile-exists` (a profile `fleet` already exists); `command-not-argv` and disagreements are findings, not errors (#650) |
528	| `say`, `interrupt`, `answer` with `--pane` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`, `session-not-found` (no session of record); open (#646), as the record says: `pane-parked`, `pane-unhealthy`, `pane-shown-driven-mismatch`; all exit 3, the code in the message. The hub's prompt gate refuses the same three as `session_held` with `hold_kind: "pane"` |
529	| `profile create` | `profile-exists`, `profile-not-found` (for `--from`), `pane-in-other-profile`, `generation-conflict`, `profile-conflict` |
530	| `profile delete` | `profile-not-found`, `profile-has-live-panes`, `generation-conflict`, `profile-conflict` |
531	| `profile list` | none |
532	| `profile show` | `profile-not-found` |
533	| `profile apply` | `profile-not-found`, `pane-in-other-profile` (without `--take-over`), `probe-failed` (per pane; the others proceed), `herdr-version-unsupported`, `grid-out-of-range`, `generation-conflict`, `profile-conflict`; open (#640) for a cell that no single split reaches, `grid-unreachable` |
534	| PROPOSED `profile rename`, `export`, `import` | `profile-not-found`, `profile-exists`, `profile-secret-refused`, `command-not-argv`, `grid-ambiguous`, `grid-out-of-range` |
~~~~

**E26: issue #642 body, lines 7-8 ("Scope (final)" items 1 and 2) and line 23 (decision D5 of "Operator decisions (2026-10-11)")** — `agent-unusable` is the OpenCode adapter's own open refusal code, not one of `ALL_CODES` (NV-1).

~~~~text
7	1. **`serve_command` applies the agent.** With `spec.agent == Some(key)` the argv it returns starts the server with `default_agent` set to `key`; with `None` it returns exactly the argv it returns today. It no longer refuses an agent with `agent-unsupported`.
8	2. **`verify_serving` checks the running server.** It returns `Ok(())` only when the server really runs the agent the spec asks for, and refuses with the adapter's own open code `agent-unusable` otherwise. The launch engine (#802) calls it after the health wait and before `create_session`, and its rollback then stops the server (a process of the pane's tmux session).
23	- **D5. `agent-unusable` is an open refusal code (exit 3), a constant in the adapter's own file** (`pub const AGENT_UNUSABLE: RefusalCode = RefusalCode::from_static("agent-unusable")` in `crates/holler-adapter-opencode/src/server.rs`, built through `PaneError::Refused`, the way the Herdr adapter owns `grid-unreachable`). It is distinct from `AGENT_UNSUPPORTED`, which stays in `holler-pane` for a harness that cannot apply an agent at all. The ADR-0021 code tables list it wherever they list `agent-unsupported`. Reason: "this harness cannot" and "this agent cannot run here" are different refusals for the operator.
~~~~

**E27: issue #650 body, lines 7, 21, 25, 26, 30, 34, 42 and 106** — What `holler pane import` does: the records and the profile `fleet` in one real run, the position taken from Herdr's snapshot, what it reads from the old files and what it observes, its `herdr-version-unsupported` error, and its dependencies (NV-3, NV-4, B-2).

~~~~text
7	Read the current fleet's per-pane files and produce registry records, a profile `fleet`, and one report of every disagreement between those files and what Herdr, tmux and the harness show. The dry run writes nothing. A real run writes the records and the profile or writes nothing at all. The verb adopts what is running: it never starts, stops, types into or moves anything, and it deletes nothing from the source files.
21	Only through `Ports` (E3, E12, E17): `herdr.snapshot()` once, `herdr.version()` once, then per pane `host.ps(name)`, `harness.health(port)` and, only when the health call answered healthy, `harness.list_sessions(port)`. **Nothing else is called**: not `ensure_pane`, `ensure_session`, `run`, `stop_owned`, `serve`, `create_session`, `attach_tui`, `select_session`, `abort`, `send_text`, `send_keys`, `close`, `shown_session`. A `herdr.snapshot()` or `herdr.version()` error ends the verb with that error (its class, its exit code), because nothing can be joined without them; `herdr-version-unsupported` is such an error.
25	1. **The Herdr pane is the snapshot entry whose `label` equals the pane's name** (ADR-0021 section 10, E9, merged by #808, E1). The name's cell is never the join key and never chooses a position. Zero matches: problem `pane-unlabelled`. Two or more: problem `pane-label-ambiguous` (naming the pane ids). The record's `herdr` is that snapshot entry **with `label` set to `None`** (a label is an observation, never recorded state; launch does the same, E12).
26	2. **The position** is the entry's `grid`. Output prints rowcol (`r2c1`). If the directory name is `hj-` followed by text that `GridPos::parse` accepts (for example `c1r2`) and that cell differs from the observed position, the report carries the information finding `name-cell-differs` (naming both cells); a name that spells no cell produces no finding.
30	   - the files agree and a **healthy** server's `list_sessions` holds `L`: `session_of_record = L` (reconcile's trust rule, E17: only a healthy server's list counts);
34	   A session id read from a file is never copied into `last_observed` (I6).
42	`name`; `generation` 0; `herdr` (item 1); `host { name: tx_launch::HOST_NAME, tmux: name, cwd: launcher cwd, herdr_api_version: Some(herdr.version()) }`; `harness { kind: opencode, port: launcher port, pid: None, health: healthy only when this run's `health` answered healthy, otherwise unknown }`; `session_of_record` (item 4); `role` (item 5); `hold: none`; `last_observed { shown: None, driven: None, at: 0 }` (0 means never observed; the first doctor pass fills it; nothing in it is read from a file); `profile: fleet`; `model`; `opencode_agent`; `env` (names); `context`; `command`; `probe` empty (E2, E12). The profile is `profile_from_panes(fleet, &records)` (E4), so `holler profile show fleet` reports every pane `matches` by construction.
106	#808 (merged, `263a2d7`: the label), #637, #639, #662 (merged). Does not depend on #802, #642 or #644.
~~~~

**E28: the dependency lines of issues #649 (line 18), #664 (line 14) and #644 (line 10)** — Neither #649 nor #664 depends on #650; #649 depends on #642 and #644, and #644 is built after #802 and #642 part 3 (NV-3).

~~~~text
18	Depends on: #639, #640, #641, #642, #644, #645, #646, #647; and #660, #661, #662, #663 (amended 2026-10-08, profiles) (all wave 3, so this story stays in wave 4). (amended 2026-10-08, review) Also #643; this story stays in wave 4 (it does not depend on #664). (amended 2026-10-09, split) Also #637, #669 and #670.
~~~~

~~~~text
14	Depends on: #637, #638, #644, #662. (amended 2026-10-08, review) (Was #640, #641, #642, #644, #647, #661, #662: the adapters and the registry are faked here.)
~~~~

~~~~text
10	Built after **#802** and **#642 part 3** have merged. Every `#802` line quoted below is the pre-merge reading, taken from #802's branch at `3fe111d` (`origin/main` is `263a2d7`); the contract names used here (`serve_command`, `verify_serving`, `ServeSpec`, `AGENT_UNSUPPORTED`) are fixed by #802's issue body. #642 part 3 supplies the OpenCode adapter's real `verify_serving` and its refusal code `agent-unusable`; the fakes in this story stand in for it.
~~~~

**E29: `crates/holler-cli/src/pane/import.rs:1-19` and `crates/holler-cli/tests/fixtures/cli-surface.pending.txt:1-10`** — On `main`, `holler pane import` is #670's stub and takes no arguments, and the pending list names no command (NV-3).

~~~~text
1	//! `holler pane import`: a stub (story #670). Story #650 owns the real verb: it replaces this
2	//! file and adds the verb's own positionals and flags to the arguments below, edits its own
3	//! ADR 0003 row and `cli-surface.txt` line, and edits no frozen file.
4	
5	use clap::Args;
6	
7	use crate::output::{emit_error, not_implemented, VerbCtx};
8	
9	/// The story that owns this verb.
10	const STORY: u32 = 650;
11	
12	/// Import the existing fleet into the pane registry.
13	#[derive(Args, Debug)]
14	pub struct PaneImport {}
15	
16	/// Run `holler pane import`: refuse, naming the story that owns it.
17	pub fn run(_args: &PaneImport, ctx: &mut VerbCtx<'_>) -> i32 {
18	    emit_error(&mut ctx.sink, ctx.format, not_implemented(STORY))
19	}
~~~~

~~~~text
1	# Verbs and flags ADR 0003 specifies that NO story has implemented yet
2	# (issue #155 §5). Same format as cli-surface.txt.
3	#
4	# tests/cli_surface_test.rs asserts every line here does NOT parse — so when
5	# the owning story lands its verb, the test fails until the line is moved to
6	# cli-surface.txt. The docs doc-test (tests/docs_cli_test.rs) tolerates a
7	# documented command whose leaf path is listed here, and nowhere else.
8	#
9	# Each line names its story.
10	
~~~~

**E30: epic #633 body, lines 124-129** — The invariants the guide's section 1 cites, I1 and I6 among them (NV-4).

~~~~text
124	Invariants      I1 only `holler pane` verbs change a pane or its registration
125	                I2 session_of_record is the only session a pane's TUI shows and the hub drives
126	                I3 every verb is plan -> act -> observe -> record; a mismatch fails loudly and records nothing
127	                I4 no verb changes a session by typing into a TUI
128	                I5 a verb returns in bounded time (default 10 s); long work returns an operation id
129	                I6 state is observed from Herdr, tmux and the harness, never inferred from files
~~~~

**E31: issue #665, its title (`gh issue view 665 --json title --jq .title`) and body lines 1, 7 and 9** — `profile-drift` is a PROPOSED `doctor` finding, not a verb (NV-2).

~~~~text
title	feat(cli): holler profile rename, export and import; doctor code profile-drift [PROPOSED]
1	Part of epic #633. (added 2026-10-08, profiles) **PROPOSED: the operator confirms before any pane picks this up. Not on the critical path; nothing waits for it.**
7	- A doctor finding `profile-drift` (a live pane differs from its spec), raised by reconcile through the comparison in `crates/holler-pane/src/profile_diff.rs` (#662).
9	(amended 2026-10-09, split) Depends on #637 and #670 (CLI side), and #661 and #669 (hub side). The positionals and flags of `profile rename OLD NEW`, `export NAME [--out FILE]` and `import FILE [--name NAME]` are yours: they live in `rename.rs`, `export.rs` and `import.rs` with your ADR 0003 rows, `cli-surface.txt` lines and `crates/holler-cli/tests/profile_verbs/<verb>.rs` files. `ProfileStore::rename(old, new, expected_generation)` and `profile/rename` are declared (PROPOSED) by #637 with a `NotImplemented` stub; the hub-side transaction is yours. #669 pre-creates the empty `holler-hub/src/profile/rename.rs`, already declared in `profile/mod.rs`, so you edit none of #661's files. `ProfileDrift` is added to #647's `findings.rs` after #647 merges.
~~~~

**E32: `docs/agent-overlays/tester.md:14`** — What "a bare command fragment in backticks" means in this repository (NIT-3).

~~~~text
14	- Every documented `holler ...` command must parse (`docs_cli_test`); a bare command fragment in backticks fails it.
~~~~

**E33: `crates/holler-pane/src/grid.rs:75-116`** — Why `GridPos::parse("hj-c1r2")` is `grid-ambiguous`: there is no comma, and a labelled cell must start with `r` or `c` (line 108) (W-2).

~~~~text
75	/// The digit strings of the row and the column of `text` (already trimmed and
76	/// lower-cased), or `None` when its structure is wrong.
77	fn split_cells(text: &str) -> Option<(&str, &str)> {
78	    if text.contains(',') {
79	        split_pair(text)
80	    } else {
81	        split_labelled(text)
82	    }
83	}
84	
85	/// `<row>,<col>`: exactly two digit runs around one comma, row first.
86	fn split_pair(text: &str) -> Option<(&str, &str)> {
87	    let (row, col) = text.split_once(',')?;
88	    (is_number(row) && is_number(col)).then_some((row, col))
89	}
90	
91	/// `r<row>c<col>` or `c<col>r<row>`: two labelled cells with different labels.
92	fn split_labelled(text: &str) -> Option<(&str, &str)> {
93	    let (first_label, first, rest) = take_cell(text)?;
94	    let (second_label, second, tail) = take_cell(rest)?;
95	    if !tail.is_empty() || first_label == second_label {
96	        return None;
97	    }
98	    if first_label == 'r' {
99	        Some((first, second))
100	    } else {
101	        Some((second, first))
102	    }
103	}
104	
105	/// One labelled cell at the start of `text`: its label (`r` or `c`), its digits and
106	/// what follows.
107	fn take_cell(text: &str) -> Option<(char, &str, &str)> {
108	    let label = text.chars().next().filter(|c| matches!(c, 'r' | 'c'))?;
109	    let body = text.strip_prefix(label)?;
110	    let end = body
111	        .bytes()
112	        .position(|b| !b.is_ascii_digit())
113	        .unwrap_or(body.len());
114	    let (digits, rest) = body.split_at(end);
115	    (!digits.is_empty()).then_some((label, digits, rest))
116	}
~~~~

**E34: `crates/holler-cli/tests/docs_cli_test.rs:115-125` and `186-201`** — What the docs test takes from prose (a code span that starts `holler `) and which parse errors it allows (a bare namespace such as `holler pane`); any other error fails it (NIT-3, NV-3).

~~~~text
115	        // Inline code spans in prose / tables.
116	        let mut rest = line;
117	        while let Some(start) = rest.find('`') {
118	            let after = &rest[start + 1..];
119	            let Some(end) = after.find('`') else { break };
120	            let span = &after[..end];
121	            if span.starts_with("holler ") {
122	                out.push(Doc { file: file.clone(), line_no, raw: span.to_string() });
123	            }
124	            rest = &after[end + 1..];
125	        }
~~~~

~~~~text
186	            checked += 1;
187	            match Cli::try_parse_from(&argv) {
188	                Ok(_) => {}
189	                // `holler --help` / `--version`, and a bare namespace such as
190	                // `holler hub` or `holler body …` used in prose to name the
191	                // role: a prefix of the real tree is fine, an unknown or
192	                // malformed verb is not.
193	                Err(e) if matches!(
194	                    e.kind(),
195	                    ErrorKind::DisplayHelp
196	                        | ErrorKind::DisplayVersion
197	                        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
198	                        | ErrorKind::MissingSubcommand
199	                ) => {}
200	                Err(e) => failures.push(format!("{}:{}: `{}` → {:?}: {}", doc.file, doc.line_no, doc.raw, argv, e.kind())),
201	            }
~~~~
