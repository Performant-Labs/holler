# [docs: the operator guide for `holler pane`, the deprecation of hand-managed session files, and the runbook entries](https://github.com/Performant-Labs/holler/issues/652)

Part of epic #633. This is the complete statement of the story (consolidated 2026-10-11 from the original scope, its amendments of 2026-10-08, and the operator's decisions of 2026-10-11). Where an older comment disagrees, this text is the decision. The story is **docs plus one docs test**: it changes no behaviour. It is briefed ahead of its dependencies and built after #649 and #664 have merged.

## Scope (final)

Write `docs/pane-control.md` (new), update `docs/orchestrating.md`, and add the guide to `docs/README.md`. The guide has these sections, in this order:

1. **The rule (I1).** Nothing but `holler pane` and `holler profile` changes a pane; the registry is the one source of truth for what a pane is; Herdr, tmux and the harness are observed (I6), never inferred from files.
2. **What a pane runs.** One Herdr pane shows one tmux session; the harness server (OpenCode) runs *inside that tmux session*, started through the host adapter from the command `HarnessPort::serve_command` builds, and stopped by `HostPort::stop_owned` (TERM, a shared grace, then KILL of every process group still alive) (#802). `--agent KEY` sets the server's default agent when it starts; a key the running server cannot use is refused with the adapter's code `agent-unusable` and the launch is rolled back; `relaunch` without `--agent` keeps the recorded agent; there is no flag to clear an agent; the agent is printed as `agent=KEY` and as an additive JSON key `agent` (#644 part 2, #642 part 3).
3. **The verbs.** `holler pane` `list`, `get`, `watch`, `launch`, `relaunch`, `switch`, `reset`, `park`, `unpark`, `close`, `doctor`, `import`; and `holler profile` `create`, `delete`, `list`, `show`, `apply`, plus `rename`, `export`, `import`, which are **PROPOSED** (#665, not built) and marked as such, with `profile-drift` likewise PROPOSED. One short subsection per verb: purpose, the flags, an example, the exit codes it can end with. `--profile` has two classes: spec-editing (`launch`, `relaunch`, `close`) and scoping (the rest); `pane import` takes no `--profile`; `--spec-only` and `--take-over`.
4. **Reading the table.** The columns of `holler pane list` (PANE, POS, PROFILE, PROJECT, HEALTH, SHOWN, DRIVEN, SYNC, HOLD) and what each means; SHOWN and HEALTH are what reconcile last recorded; DRIVEN is printed as the record holds it. **The roster extension (the PANE/POS/PROFILE/HEALTH/SHOWN/DRIVEN columns on `holler roster`, #648) is marked "later"**: this guide documents `pane list` and `doctor` as the places to read SHOWN against DRIVEN, and says `holler roster` does not show them yet.
5. **`doctor`.** What one reconcile pass reports (every finding kind, in `FindingKind::ALL`), what `--fix` repairs (only `shown-driven-mismatch`: it selects the session of record; it never changes the session of record), what it will not fix (every other kind, with the command the finding names), that a pass that completes exits 0 with findings in the report, and the periodic pass (a scheduler runs `holler pane doctor --format=json` without `--fix`; there is no hub timer).
6. **Profiles.** What a profile is (a set of pane specs; at most one profile per pane; no secrets, env var names only); `create` (with `--from-current`), `delete` (refused while panes are live, `--keep-panes`), `list`, `show`; and `apply`, whose reference is the decided contract in "The `profile apply` reference" below.
7. **Grid notation.** Positions are rowcol and 1-based: text `r2c1`, JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first. Input accepts `r<row>c<col>`, `c<col>r<row>` and a bare `<row>,<col>` (always row first); the refusals `grid-ambiguous` and `grid-out-of-range`; and that **pane names such as `hj-c1r2` predate the standard and list the column first, so `hj-c1r2` sits at `r2c1`; no pane is renamed and a name is never read as a position**.
8. **Probe, argv, layout, Herdr version.** The health probe, with the example `check ["curl","-s","http://127.0.0.1:8095/v1/models"]`, `expect ["qwen38"]` and the refusal `probe-failed`; the argv rule (`--command-arg` / `--command-json`, never a shell string, `command-not-argv`); layout reached by splits; the Herdr API version shown by `doctor` and the finding `herdr-version-unsupported`.
9. **JSON output reference.** `--format=json` (and `--json` under `pane` and `profile`) prints exactly one envelope per result with `schema_version` 1; `pane watch` prints one envelope per line (NDJSON, resumable with `--since CURSOR`); the exit codes (a table: 0 ok, 1 runtime failure, 2 usage, 3 refusal, the same in text and JSON mode); and **every error code** (`ALL_CODES`) with its class.
10. **Migration and cutover.** The procedure of #654: run `holler pane import --dry-run` on the live fleet and read every finding; fix drift with the old tools; import (a real import also creates the profile `fleet` from today's panes and their layout); run `doctor` until SHOWN equals DRIVEN for every pane (and `pane list` shows it); cut **one** pane over to `launch` / `reset` and watch it **through a full overnight**; then move the rest; seven days with no invisible session (every day `doctor` reports no `shown-driven-mismatch` and no stray session older than an hour) and the operator signs off. Record each step in `docs/handoffs/<#654>-cutover/decisions.md`. **Files the old launcher wrote that are now derived** (a list): `sessions.toml`, `last-session`, `owned-sessions`, `pane.env`; they are kept, read-only, for one release.
11. **Rollback.** **`scripts/pane-restart` stays as the fallback until the 7-day sign-off**; the guide says so, shows its usage line, and says the script is removed (in a later story, not this one) after the sign-off. Restarting a pane with the verbs is `holler pane relaunch PANE --model ...`.
12. **Deferred, so not documented as existing:** the roster extension (#648), the shared subprocess runner (#696), the real-adapter scenario for `profile apply` (#667) and the workspace recreation from profiles (#666) are after the cutover; the guide may name them once under "Later" and describes none of them as available.

`docs/orchestrating.md`: the section "Restarting a pane on another model (interim)" is rewritten so that `holler pane relaunch PANE --model PROVIDER/ID [--effort E]` is the way, and `scripts/pane-restart` is named as the fallback **until the 7-day sign-off** (replacing the sentence "This section is removed when #644 and #653 have merged"); the rest of that page is unchanged. The agent guidance for orchestrators is this page and the `docs/README.md` entry; `AGENTS.md` and `agent-skills/` are out of scope.

## The `profile apply` reference (decided in the #664 brief; quote it, do not restate differently)

- One step per spec pane in (workspace, row, col) order; the first matching row applies: **a** another profile and no `--take-over`: `refused` (`pane-in-other-profile`); **b** no record: `create` through the launch transaction; **c** healthy and matches its spec: `adopt` (sets `Pane.profile`; with a take-over from another profile, leave then join, two writes); **d** healthy and differs: `reported` with the pane's diff, nothing live touched; **e** not healthy, role orchestrator: `refused` (`orchestrator-not-relaunched`), no write even under `--take-over`; **f** not healthy, role agent: `relaunch`, and under `--take-over` from another profile the leave and join happen **first**, then the relaunch.
- Outcomes: `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`. `refused` and `reported` do not stop the run or change the exit code. A `failed` step (`unavailable`, `timeout`, a failed health call, a store conflict) stops the run; the later steps are `skipped`. Exit code: 1 if any step failed, 2 for the up-front usage case (a plan with a `create` and no `--herdr-session`), else 0.
- A run that ends with a `failed` step prints **only the failure**: in JSON the failure envelope (`ok` false, `data` null, `error.message` is `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`), in text one `error:` line on stderr and nothing on stdout; a dry run with a failed step prints no plan.
- `--dry-run` prints the same table as the plan, runs no probe, issues no split and writes nothing. `data` is `{"profile": NAME, "dry_run": bool, "steps": [{pane, action, outcome, code?, diff?}]}`. Flags: `--herdr-session NAME`, `--take-over`, `--dry-run`, `--format`.
- "Healthy" is a live observation (the harness answers its health call, Herdr's snapshot lists the recorded pane, and a readable shown session equals the session of record), never the stored health; apply never kills a healthy pane; a shown session that cannot be read does not make a pane unhealthy.

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
3. **Error codes.** Every string of `holler_pane::error::ALL_CODES` appears in the guide inside backticks, and the exit-code table of section 9 has a row for each of 0, 1, 2, 3 (the values of `ErrorClass::exit_code`).
4. **Columns.** Every name in `holler_cli::pane::list::COLUMNS` appears in the guide's section 4.
5. **Grid examples parse.** The guide's grid section carries a fenced block tagged `text` with one example per line, `ok <form>` or `refused <code> <form>`; the test parses each `ok` form with `holler_pane::GridPos::parse` and each `refused` form and asserts the named code (`grid-ambiguous` or `grid-out-of-range`). The block contains at least the forms `r2c1`, `c1r2`, `2,1`, `R2C1` and one refused form of each code.
6. **The existing docs test still passes:** every `holler ...` command in the new guide parses against the clap tree (`docs_cli_test` scans `docs/**/*.md` already; a command whose leaf verb is in `cli-surface.pending.txt` is tolerated and reported). No bare command fragment appears in backticks.
7. **Roster is marked later.** The guide's section 4 contains a line with `holler roster` and `#648` and the word `later`.
8. **Rollback.** The guide contains `scripts/pane-restart` and the words `until the 7-day sign-off`; `docs/orchestrating.md` contains `holler pane relaunch` and `scripts/pane-restart` and no longer contains `removed when #644 and #653 have merged`.
9. **Derived files.** The guide's section 10 names each of `sessions.toml`, `last-session`, `owned-sessions` and `pane.env`.
10. **Index.** `docs/README.md` links `pane-control.md` under "Guidance".
11. **Deferred items are not described as available:** the guide does not contain `holler roster --profile` as an example, and does not contain the strings `profile export` or `profile import` outside a line marked `PROPOSED`.

## Blast radius

- `docs/pane-control.md` (new), `docs/orchestrating.md`, `docs/README.md`
- `crates/holler-cli/tests/pane_control_doc_test.rs` (new) and its `[[test]]` entry in `crates/holler-cli/Cargo.toml`
- `CHANGELOG.md` (an `## [Unreleased]` entry linking this issue)

No source file outside tests changes. Markdown files under `docs/` are scanned by `docs_cli_test`, so the guide must obey its rules (quoted in E12).

## Depends on

#649 and #664 (both before this story is built). The brief can be written and reviewed ahead of them; the guide is written and its tests run after both have merged, because the verbs it describes must exist in the clap tree and the behaviours it states must be the merged ones. If #649 or #664 changes a behaviour stated above, the brief is patched and the review re-run before the build.

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
