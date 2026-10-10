//! `holler pane switch` (#645, brief ACs 1-13 and 23) and the runner its cases share with
//! `reset.rs`.
//!
//! Every case runs the verb in **both** formats, each on a fresh doctor rig
//! (`crate::doctor::rig`, brief Decision 16), and [`both`] asserts what holds for every
//! run: the exit code is the same in both formats; JSON is one valid envelope
//! (`check_envelope`) with nothing on `err`; text writes one line, to `out` on success and
//! as `error: <message>` to `err` on failure; and no run calls Herdr or the host, so no
//! keystroke reaches a TUI (I4, AC 2).

use std::collections::BTreeMap;

use holler_cli::output::Format;
use holler_pane::pane::{Health, Hold};
use holler_pane::{HarnessPort, Pane, PaneError, PaneId, PaneName};
use holler_pane_testkit::envelope::{check_envelope, Envelope};
use holler_pane_testkit::harness::{FakeHarness, HarnessOp, HarnessOp as H, Quirk, TuiView};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};
use holler_proto::clock::now_millis;
use serde_json::Value;

use crate::doctor::rig::{Calls, Rig, Seed};
use crate::verb_harness::parse::try_parse;
use crate::verb_harness::{run_verb_with, Outcome};

pub(crate) const P: &str = "demo-c1r1";
pub(crate) const Q: &str = "demo-c2r1";
/// How every failure after the act ends for `P`: the pane doctor command line with `--fix`.
pub(crate) const RECONCILE_P: &str = "; to reconcile, run holler pane doctor demo-c1r1 --fix";

/// One run of a verb on a rig of its own.
pub(crate) struct Case<T> {
    pub format: Format,
    pub rig: Rig,
    /// What the build step returned (a seeded session, a record, ...).
    pub setup: T,
    /// Every seeded pane's record and TUI just before the run.
    pub before: BTreeMap<&'static str, (Pane, Option<TuiView>)>,
    /// The calls the run made, through every port.
    pub calls: Calls,
    pub run: Outcome,
    /// `now_millis()` just before and just after the run.
    pub window: (i64, i64),
}

impl<T> Case<T> {
    pub fn record(&self, name: &str) -> Pane {
        self.rig.record(name)
    }

    pub fn before(&self, name: &str) -> &Pane {
        &self.before[name].0
    }

    /// The session the TUI of `name` shows now.
    pub fn shown(&self, name: &str) -> Option<String> {
        self.rig
            .harness
            .tui(&self.rig.pane_id(name))
            .and_then(|tui| tui.shown)
    }

    /// Every record (its generation included) and every TUI is as it was before the run.
    pub fn assert_unchanged(&self) {
        for (name, (pane, tui)) in &self.before {
            let label = format!("{:?}, {name}", self.format);
            assert_eq!(&self.record(name), pane, "{label}: the record is unchanged");
            let now = self.rig.harness.tui(&self.rig.pane_id(name));
            assert_eq!(&now, tui, "{label}: the TUI is unchanged");
        }
    }

    /// The run recorded `target` for `P` and its TUI shows it: the record before the run with
    /// exactly `session_of_record`, `last_observed.shown`, `last_observed.at` (now) and
    /// `harness.health` (healthy) set, and its generation bumped by one.
    pub fn assert_recorded(&self, target: &str) {
        let label = format!("{:?}", self.format);
        let before = self.before(P);
        let after = self.record(P);
        let mut want = before.clone();
        want.generation = before.generation + 1;
        want.session_of_record = Some(target.to_owned());
        want.last_observed.shown = Some(target.to_owned());
        want.last_observed.at = after.last_observed.at;
        want.harness.health = Health::Healthy;
        assert_eq!(after, want, "{label}: exactly the four fields change");
        let (start, end) = self.window;
        assert!(
            (start..=end).contains(&after.last_observed.at),
            "{label}: last_observed.at {} is the run's clock, in {start}..={end}",
            after.last_observed.at
        );
        assert_eq!(
            self.shown(P).as_deref(),
            Some(target),
            "{label}: the TUI shows it"
        );
    }

    /// The envelope of a JSON run (`both` has checked it).
    pub fn envelope(&self) -> Envelope {
        check_envelope(&self.run.out, self.run.code).expect("a valid envelope")
    }
}

/// `words` as an argv.
pub(crate) fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

/// [`both_with`] through the rig's own ports.
pub(crate) fn both<T>(
    build: impl Fn() -> (Rig, T),
    words: impl Fn(&Rig, &T) -> Vec<String>,
) -> [Case<T>; 2] {
    both_with(build, words, |rig, _, argv, format| rig.verb(argv, format))
}

/// Build a rig, take its argv, then run the verb with `exec`: once in text and once in
/// JSON, each on its own rig. Asserts what holds for every run (see the module docs).
pub(crate) fn both_with<T>(
    build: impl Fn() -> (Rig, T),
    words: impl Fn(&Rig, &T) -> Vec<String>,
    exec: impl Fn(&Rig, &T, &[&str], Format) -> Outcome,
) -> [Case<T>; 2] {
    let cases = [Format::Text, Format::Json].map(|format| one_case(format, &build, &words, &exec));
    let [text, json] = &cases;
    assert_eq!(
        text.run.code, json.run.code,
        "the exit code is the same in both formats: text {:?}, json {:?}",
        text.run, json.run
    );
    check_text(&text.run);
    let checked = check_envelope(&json.run.out, json.run.code);
    assert!(checked.is_ok(), "json: {checked:?}: {:?}", json.run);
    assert!(
        json.run.err.is_empty(),
        "json: nothing on err: {:?}",
        json.run
    );
    for case in &cases {
        assert!(
            case.calls.herdr.is_empty() && case.calls.host.is_empty(),
            "{:?}: no Herdr or host call, so no keystroke (I4): {:?}",
            case.format,
            case.calls
        );
    }
    cases
}

fn one_case<T>(
    format: Format,
    build: &impl Fn() -> (Rig, T),
    words: &impl Fn(&Rig, &T) -> Vec<String>,
    exec: &impl Fn(&Rig, &T, &[&str], Format) -> Outcome,
) -> Case<T> {
    let (rig, setup) = build();
    let before = rig
        .live
        .keys()
        .map(|&name| {
            (
                name,
                (rig.record(name), rig.harness.tui(&rig.pane_id(name))),
            )
        })
        .collect();
    let owned = words(&rig, &setup);
    let argv: Vec<&str> = owned.iter().map(String::as_str).collect();
    let mark = rig.mark();
    let start = now_millis();
    let run = exec(&rig, &setup, &argv, format);
    let window = (start, now_millis());
    let calls = rig.calls_since(&mark);
    Case {
        format,
        rig,
        setup,
        before,
        calls,
        run,
        window,
    }
}

/// Text mode writes one line: the result to `out`, or `error: <message>` to `err`.
fn check_text(run: &Outcome) {
    let (line, other) = if run.code == 0 {
        (&run.out, &run.err)
    } else {
        (&run.err, &run.out)
    };
    assert!(other.is_empty(), "text: the other stream is empty: {run:?}");
    assert!(
        line.ends_with('\n') && line.lines().count() == 1,
        "text: one line: {run:?}"
    );
    if run.code != 0 {
        assert!(line.starts_with("error: "), "text: an error line: {run:?}");
    }
}

/// Both runs exited `exit` with `code`; returns each run's message (text's after `error: `).
pub(crate) fn failed<T>(cases: &[Case<T>; 2], exit: i32, code: &str) -> [String; 2] {
    let [text, json] = cases;
    assert_eq!(json.run.code, exit, "exit {exit} ({code}): {:?}", json.run);
    let error = json
        .envelope()
        .error
        .expect("a failed envelope has an error");
    assert_eq!(error.code, code, "{:?}", json.run);
    let line = text.run.err.trim_end();
    let message = line.strip_prefix("error: ").unwrap_or(line).to_owned();
    [message, error.message]
}

/// [`failed`] for a run that failed before `select_session` was called: it moved neither the
/// TUI nor the record, so neither message carries the reconcile step (ADR-0021 section 8,
/// "Switch and reset as built").
pub(crate) fn failed_before_the_act<T>(cases: &[Case<T>; 2], exit: i32, code: &str) -> [String; 2] {
    let messages = failed(cases, exit, code);
    for message in &messages {
        assert!(
            !message.contains("to reconcile"),
            "no reconcile step before the act: {message:?}"
        );
    }
    messages
}

/// The `data` of the JSON run of `cases`, which must have exited 0 (and so has text).
pub(crate) fn data<T>(cases: &[Case<T>; 2]) -> Value {
    let [_, json] = cases;
    assert_eq!(json.run.code, 0, "the run succeeds: {:?}", json.run);
    json.envelope().data
}

/// A rig of `seeds` with a second session seeded on `P`'s server; returns it with that session.
pub(crate) fn with_s2(seeds: &[Seed]) -> (Rig, String) {
    let rig = Rig::new(seeds);
    let s2 = rig.harness.seed_session(rig.live(P).port);
    (rig, s2)
}

fn one_pane() -> (Rig, String) {
    with_s2(&[Seed::new(P, 1, 1)])
}

/// `pane switch P <the case's S2>`, with `extra` after it.
fn switch_p(extra: &'static [&'static str]) -> impl Fn(&Rig, &String) -> Vec<String> {
    move |_, s2| {
        let mut words = argv(&["pane", "switch", P, s2]);
        words.extend(argv(extra));
        words
    }
}

/// ACs 1 and 2: the TUI and the record move together, through the harness API only.
#[test]
fn switch_moves_the_tui_and_the_record_together() {
    let build = || {
        let (rig, s2) = one_pane();
        rig.rewrite(P, |p| {
            p.last_observed.driven = Some("ses_driven".to_owned())
        });
        (rig, s2)
    };
    let cases = both(build, switch_p(&[]));
    let data = data(&cases);
    for case in &cases {
        case.assert_recorded(&case.setup);
        assert_eq!(
            case.record(P).last_observed.driven.as_deref(),
            Some("ses_driven")
        );
        let (harness, panes) = (&case.calls.harness, &case.calls.panes);
        let ops = [
            H::Health,
            H::ListSessions,
            H::SelectSession,
            H::ShownSession,
        ];
        assert_eq!(harness, &ops, "{:?}", case.format);
        let writes = [PaneStoreOp::Get, PaneStoreOp::List, PaneStoreOp::CasPut];
        assert_eq!(panes, &writes, "{:?}", case.format);
    }
    let [text, json] = &cases;
    let s1 = &text.rig.live(P).session;
    let want = format!(
        "switched {P} to session \"{}\" (was \"{s1}\")\n",
        text.setup
    );
    assert_eq!(text.run.out, want);
    assert_eq!(data["verb"], "switch");
    assert_eq!(data["previous"], json.rig.live(P).session.as_str());
    assert_eq!(data["pane"]["session_of_record"], json.setup.as_str());
    assert_eq!(
        data["pane"],
        serde_json::to_value(json.record(P)).expect("json")
    );
}

/// AC 3: a deleted session is refused before anything moves.
#[test]
fn switch_to_a_deleted_session_changes_nothing() {
    let build = || {
        let (rig, s2) = one_pane();
        rig.harness.delete_session(&s2).expect("delete S2");
        (rig, s2)
    };
    let cases = both(build, switch_p(&[]));
    failed_before_the_act(&cases, 3, "session-not-found");
    for case in &cases {
        assert!(!case.calls.harness.contains(&HarnessOp::SelectSession));
        case.assert_unchanged();
        assert_eq!(case.shown(P), Some(case.rig.live(P).session.clone()));
    }
}

/// AC 4, the "order ran in the wrong session" incident: P's server lists Q's session
/// (one shared data directory), and switch still refuses it.
#[test]
fn switch_to_another_panes_session_is_refused() {
    let build = || (Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]), ());
    let words = |rig: &Rig, (): &()| argv(&["pane", "switch", P, &rig.live(Q).session]);
    let cases = both(build, words);
    for message in failed_before_the_act(&cases, 3, "session-of-other-pane") {
        assert!(message.contains(Q), "names the other pane: {message:?}");
    }
    for case in &cases {
        assert!(!case.calls.harness.contains(&HarnessOp::SelectSession));
        case.assert_unchanged();
    }
}

/// A scenario that stops the server on a port.
type Stop = fn(&FakeHarness, u16) -> Result<(), PaneError>;

/// AC 5: a killed or frozen server is refused before the target is looked up; a health
/// check that times out is a failure.
#[test]
fn switch_refuses_an_unhealthy_server() {
    let stops: [Stop; 2] = [FakeHarness::kill, FakeHarness::freeze];
    for stop in stops {
        let build = || {
            let (rig, s2) = one_pane();
            stop(&rig.harness, rig.live(P).port).expect("stop the server");
            (rig, s2)
        };
        let cases = both(build, switch_p(&[]));
        for message in failed_before_the_act(&cases, 3, "server-unhealthy") {
            assert!(
                message.contains("run holler pane relaunch demo-c1r1"),
                "{message:?}"
            );
        }
        for case in &cases {
            assert_eq!(case.calls.harness, [HarnessOp::Health], "{:?}", case.format);
            case.assert_unchanged();
        }
    }
    let build = || {
        let (rig, s2) = one_pane();
        let op = "harness.health".to_owned();
        rig.harness
            .faults()
            .fail_next(HarnessOp::Health, PaneError::Timeout { op });
        (rig, s2)
    };
    let cases = both(build, switch_p(&[]));
    failed_before_the_act(&cases, 1, "timeout");
    cases.iter().for_each(Case::assert_unchanged);
}

/// AC 6: the orchestrator's pane needs `--as-operator`.
#[test]
fn switch_refuses_the_orchestrators_pane_unless_as_operator() {
    let build = || with_s2(&[Seed::new(P, 1, 1).orchestrator()]);
    let cases = both(build, switch_p(&[]));
    for message in failed_before_the_act(&cases, 3, "orchestrator-pane") {
        assert!(message.contains("--as-operator"), "{message:?}");
    }
    for case in &cases {
        assert!(case.calls.harness.is_empty(), "{:?}", case.calls);
        case.assert_unchanged();
    }
    let cases = both(build, switch_p(&["--as-operator"]));
    data(&cases);
    cases
        .iter()
        .for_each(|case| case.assert_recorded(&case.setup));
}

/// AC 7 (I3): the TUI does not show the target after the act, so nothing is recorded.
#[test]
fn switch_mismatch_after_select_records_nothing() {
    let build = || {
        let (rig, s2) = one_pane();
        rig.harness
            .close_tui(&rig.pane_id(P))
            .expect("close the TUI");
        rig.harness.set_quirk(Quirk::SelectAckedWithoutTui, true);
        (rig, s2)
    };
    let cases = both(build, switch_p(&[]));
    for message in failed(&cases, 1, "unavailable") {
        assert!(message.contains("its home screen"), "{message:?}");
        assert!(message.ends_with(RECONCILE_P), "{message:?}");
    }
    cases.iter().for_each(Case::assert_unchanged);
}

/// AC 8: a failed select may have moved the TUI, so the message names the reconcile step.
#[test]
fn switch_select_failure_names_the_reconcile_step() {
    let build = || {
        let (rig, s2) = one_pane();
        let op = "harness.select_session".to_owned();
        let timeout = PaneError::Timeout { op };
        rig.harness
            .faults()
            .fail_next(HarnessOp::SelectSession, timeout);
        (rig, s2)
    };
    let cases = both(build, switch_p(&[]));
    for message in failed(&cases, 1, "timeout") {
        assert!(message.ends_with(RECONCILE_P), "{message:?}");
    }
    cases.iter().for_each(Case::assert_unchanged);
}

/// A harness whose `select_session` first lets another writer store `other` (P's record,
/// changed), then delegates to the rig's fake: the record write after the act meets a
/// newer generation.
struct WriterInSelect<'a> {
    inner: &'a FakeHarness,
    panes: &'a FakePaneStore,
    other: &'a Pane,
}

impl HarnessPort for WriterInSelect<'_> {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        self.inner.serve(name, port)
    }
    fn health(&self, port: u16) -> Result<bool, PaneError> {
        self.inner.health(port)
    }
    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        self.inner.create_session(port)
    }
    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(port)
    }
    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.abort(port, session)
    }
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.attach_tui(pane, port, session)
    }
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.panes.concurrent_put(self.other)?;
        self.inner.select_session(pane, session)
    }
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane)
    }
}

/// AC 9: another writer between the plan and the record: the verb fails after the live
/// change, writes nothing more, and the other writer's record stands.
#[test]
fn switch_record_conflict_after_the_act() {
    let build = || {
        let (rig, s2) = one_pane();
        let mut other = rig.record(P);
        other.hold = Hold::Drained;
        (rig, (s2, other))
    };
    let words = |_: &Rig, (s2, _): &(String, Pane)| argv(&["pane", "switch", P, s2]);
    let exec = |rig: &Rig, (_, other): &(String, Pane), argv: &[&str], format| {
        let harness = WriterInSelect {
            inner: &rig.harness,
            panes: &rig.panes,
            other,
        };
        run_verb_with(argv, format, rig.ports_with(&harness))
    };
    let cases = both_with(build, words, exec);
    for message in failed(&cases, 1, "generation-conflict") {
        assert!(message.ends_with(RECONCILE_P), "{message:?}");
    }
    for case in &cases {
        let stored = case.record(P);
        let s1 = &case.rig.live(P).session;
        assert_eq!(
            stored.session_of_record.as_ref(),
            Some(s1),
            "the other writer's"
        );
        assert_eq!(stored.hold, Hold::Drained, "the other writer's");
        assert_eq!(stored.generation, case.before(P).generation + 1);
        assert_eq!(case.shown(P).as_ref(), Some(&case.setup.0), "the TUI moved");
    }
}

/// AC 10: `--profile` scopes the pane; a pane outside it or a missing profile is refused
/// before the harness is reached.
#[test]
fn switch_in_a_profile() {
    let seeds = [Seed::new(P, 1, 1).in_profile("demo"), Seed::new(Q, 1, 2)];
    let build = || with_s2(&seeds);
    let cases = both(build, switch_p(&["--profile", "demo"]));
    data(&cases);
    cases
        .iter()
        .for_each(|case| case.assert_recorded(&case.setup));

    let outside = |_: &Rig, s2: &String| argv(&["pane", "switch", Q, s2, "--profile", "demo"]);
    let cases = both(build, outside);
    failed_before_the_act(&cases, 3, "pane-not-in-profile");
    for case in &cases {
        assert!(case.calls.harness.is_empty(), "{:?}", case.calls);
        case.assert_unchanged();
    }

    let cases = both(build, switch_p(&["--profile", "nope"]));
    failed_before_the_act(&cases, 3, "profile-not-found");
    cases.iter().for_each(Case::assert_unchanged);
}

/// AC 11: a pane with no record.
#[test]
fn switch_unknown_pane() {
    let words = |_: &Rig, s2: &String| argv(&["pane", "switch", "demo-c9r9", s2]);
    let cases = both(one_pane, words);
    failed_before_the_act(&cases, 3, "pane-not-found");
    for case in &cases {
        assert!(case.calls.harness.is_empty(), "{:?}", case.calls);
    }
}

/// AC 12: a bad pane name or session id is `usage`, before any port is called, and no
/// control character reaches the message.
#[test]
fn switch_usage() {
    let long = "a".repeat(65);
    let bad: [[&str; 2]; 5] = [
        ["BAD_NAME", "ses_0001"],
        [P, "ses x"],
        [P, ""],
        [P, "ses_\u{1b}[31m"],
        [P, long.as_str()],
    ];
    for [pane, session] in bad {
        let words = |_: &Rig, (): &()| argv(&["pane", "switch", pane, session]);
        let cases = both(|| (Rig::new(&[Seed::new(P, 1, 1)]), ()), words);
        for message in failed_before_the_act(&cases, 2, "usage") {
            assert!(
                !message.contains('\u{1b}'),
                "{pane} {session:?}: {message:?}"
            );
        }
        for case in &cases {
            let c = &case.calls;
            let none = c.panes.is_empty() && c.profiles.is_empty() && c.harness.is_empty();
            assert!(
                none && c.probes == 0,
                "{pane} {session:?}: no port call: {c:?}"
            );
        }
    }
}

/// AC 13: switching to the session already of record is a normal run.
#[test]
fn switch_to_the_current_session_is_idempotent() {
    let build = || (Rig::new(&[Seed::new(P, 1, 1)]), ());
    let words = |rig: &Rig, (): &()| argv(&["pane", "switch", P, &rig.live(P).session]);
    let cases = both(build, words);
    let data = data(&cases);
    for case in &cases {
        case.assert_recorded(&case.rig.live(P).session);
    }
    let [text, json] = &cases;
    let s1 = &text.rig.live(P).session;
    assert_eq!(
        text.run.out,
        format!("switched {P} to session \"{s1}\" (was \"{s1}\")\n")
    );
    assert_eq!(data["previous"], json.rig.live(P).session.as_str());
}

/// AC 23: the help names the verbs' arguments; `reset` has no `--first` in 645a.
#[test]
fn help_names_the_arguments() {
    let help = |verb: &str| {
        let error = try_parse(&["pane", verb, "--help"]).expect_err("--help stops the parse");
        error.to_string()
    };
    let switch = help("switch");
    for word in ["PANE", "SESSION", "--as-operator"] {
        assert!(
            switch.contains(word),
            "switch --help names {word}: {switch}"
        );
    }
    let reset = help("reset");
    for word in ["PANE", "--as-operator"] {
        assert!(reset.contains(word), "reset --help names {word}: {reset}");
    }
    assert!(!reset.contains("--first"), "no --first in 645a: {reset}");
}
