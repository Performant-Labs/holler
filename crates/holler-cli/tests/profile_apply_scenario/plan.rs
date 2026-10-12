//! What the scenario reads from `profile apply` and from the live scratch set (decisions 2
//! and 8 of the #667 brief):
//!
//! - [`Applied`]: a strict reader of apply's `data`, the guide's shape `{"profile",
//!   "dry_run", "steps": [{"pane", "action", "outcome", "code", "diff", "detail"}]}` with an
//!   absent value `null`. A missing key, or an action or outcome name that production's
//!   `holler_pane::tx_apply::{Action, Outcome}` does not print, is refused (A's W-7: the names
//!   are production's own `as_str`, never a second list of strings). `pins.rs` pins it.
//! - [`assert_refused_run`]: a real run that refused steps, which exits 3 and prints only the
//!   refusal (#853), so its steps are not data; a case reads them from a dry run first.
//! - [`assert_plan_agrees_with_show`]: a dry run's plan against `profile show`'s comparison,
//!   with the same assertion shape as the fakes' `a_dry_run_and_profile_show_agree`.
//! - [`Still`]: what "apply left these panes alone" measures. Built only from reads (`pane
//!   list`, `pane get`, `profile show`, tmux's tags, a TCP connect and Herdr's snapshot), never
//!   from `pane doctor`, which writes records (A's W-4).

use std::collections::{BTreeMap, BTreeSet};

use holler_pane::findings::quoted;
use holler_pane::tx_apply::{Action, Outcome};
use holler_pane::GridPos;
use serde_json::{json, Map, Value};

use crate::scratch::rig::refused;
use crate::scratch::{Scratch, Settings};
use crate::Out;

/// Decision 9: `program` is on the binary's `PATH` (the scratch set's, not this process's),
/// so a health check that names it runs. Opted in, a missing one fails the run.
pub fn assert_on_binary_path(settings: &Settings, program: &str) {
    let found = settings
        .path
        .split(':')
        .any(|dir| std::path::Path::new(dir).join(program).is_file());
    assert!(
        found,
        "no {program} on the binary's PATH {:?}: the scenario needs it",
        settings.path
    );
}

/// Every action apply prints, as production names it.
const ACTIONS: [Action; 6] = [
    Action::Create,
    Action::Adopt,
    Action::Report,
    Action::Relaunch,
    Action::Refuse,
    Action::Observe,
];

/// Every outcome apply prints, as production names it.
const OUTCOMES: [Outcome; 8] = [
    Outcome::Created,
    Outcome::Relaunched,
    Outcome::Adopted,
    Outcome::Reported,
    Outcome::Refused,
    Outcome::Failed,
    Outcome::Skipped,
    Outcome::Planned,
];

/// `profile apply`'s `data`, read strictly.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub profile: String,
    pub dry_run: bool,
    pub steps: Vec<Step>,
}

/// One step of [`Applied`]. `diff` is `null` or show's `{pane, status, differences}` row.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub pane: String,
    pub action: Action,
    pub outcome: Outcome,
    pub code: Option<String>,
    pub diff: Value,
    pub detail: Option<String>,
}

/// The value of `key` in `object`, which must be present (it may be `null`).
fn field<'a>(object: &'a Map<String, Value>, key: &str, of: &str) -> Result<&'a Value, String> {
    object
        .get(key)
        .ok_or_else(|| format!("{of} has no {key:?}: {}", Value::Object(object.clone())))
}

/// A present key whose value is a string.
fn text(object: &Map<String, Value>, key: &str, of: &str) -> Result<String, String> {
    let value = field(object, key, of)?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{of}'s {key:?} is not a string: {value}"))
}

/// A present key whose value is a string or `null`.
fn optional_text(
    object: &Map<String, Value>,
    key: &str,
    of: &str,
) -> Result<Option<String>, String> {
    match field(object, key, of)? {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.clone())),
        other => Err(format!(
            "{of}'s {key:?} is neither a string nor null: {other}"
        )),
    }
}

impl Applied {
    /// Read `data` in the guide's shape, or say why it is not.
    pub fn parse(data: &Value) -> Result<Applied, String> {
        let top = data
            .as_object()
            .ok_or_else(|| format!("apply's data is not an object: {data}"))?;
        let dry_run = field(top, "dry_run", "apply's data")?;
        let steps = field(top, "steps", "apply's data")?;
        Ok(Applied {
            profile: text(top, "profile", "apply's data")?,
            dry_run: dry_run
                .as_bool()
                .ok_or_else(|| format!("\"dry_run\" is not a boolean: {dry_run}"))?,
            steps: steps
                .as_array()
                .ok_or_else(|| format!("\"steps\" is not an array: {steps}"))?
                .iter()
                .map(Step::parse)
                .collect::<Result<_, _>>()?,
        })
    }

    /// The plan or result `out` printed (a successful envelope), read strictly.
    pub fn of(out: &Out) -> Applied {
        let data = out.data();
        Applied::parse(&data).unwrap_or_else(|why| panic!("apply's data: {why}: {data}"))
    }

    /// The step of `pane`; exactly one.
    pub fn step(&self, pane: &str) -> &Step {
        let mut found = self.steps.iter().filter(|step| step.pane == pane);
        let step = found
            .next()
            .unwrap_or_else(|| panic!("a step for {pane}: {self:#?}"));
        assert!(found.next().is_none(), "one step for {pane}: {self:#?}");
        step
    }

    /// The steps' panes, in the order printed.
    pub fn panes(&self) -> Vec<&str> {
        self.steps.iter().map(|step| step.pane.as_str()).collect()
    }

    /// Whether any step has `outcome`.
    pub fn any(&self, outcome: Outcome) -> bool {
        self.steps.iter().any(|step| step.outcome == outcome)
    }
}

impl Step {
    fn parse(value: &Value) -> Result<Step, String> {
        let step = value
            .as_object()
            .ok_or_else(|| format!("a step is not an object: {value}"))?;
        let action = text(step, "action", "a step")?;
        let outcome = text(step, "outcome", "a step")?;
        let diff = field(step, "diff", "a step")?;
        if !(diff.is_null() || diff.is_object()) {
            return Err(format!(
                "a step's \"diff\" is neither an object nor null: {diff}"
            ));
        }
        Ok(Step {
            pane: text(step, "pane", "a step")?,
            action: ACTIONS
                .into_iter()
                .find(|known| known.as_str() == action)
                .ok_or_else(|| format!("no action is named {action:?}: {value}"))?,
            outcome: OUTCOMES
                .into_iter()
                .find(|known| known.as_str() == outcome)
                .ok_or_else(|| format!("no outcome is named {outcome:?}: {value}"))?,
            code: optional_text(step, "code", "a step")?,
            diff: diff.clone(),
            detail: optional_text(step, "detail", "a step")?,
        })
    }

    /// `(action, outcome)`, for one assertion per step.
    pub fn row(&self) -> (Action, Outcome) {
        (self.action, self.outcome)
    }
}

/// #853 (ADR-0021 section 8, decision 33): `out` is a real apply that refused `refused` (each
/// pane and its code, in step order) and ran `ran` other steps: exit 3, no `data`, `error.code`
/// the first refused step's, and the message `apply refused <N> step(s): <pane> (<code>), ...;
/// <M> other step(s) ran`. Returns the rest of the message (each refused step that had begun
/// its act adds `; <pane>: <detail>`).
pub fn assert_refused_run(out: &Out, refused: &[(&str, &str)], ran: usize) -> String {
    assert_eq!(out.code, 3, "a refused step exits 3: {out:?}");
    let envelope = out.envelope();
    assert_eq!(envelope.data, Value::Null, "no steps: {out:?}");
    let error = envelope.error.expect("a failure");
    let (_, first) = refused.first().expect("a case names a refused step");
    assert_eq!(error.code, *first, "the first refused step's code: {out:?}");
    let named: Vec<String> = refused
        .iter()
        .map(|(pane, code)| format!("{} ({code})", quoted(pane)))
        .collect();
    let head = format!(
        "apply refused {} step(s): {}; {ran} other step(s) ran",
        refused.len(),
        named.join(", ")
    );
    error
        .message
        .strip_prefix(&head)
        .unwrap_or_else(|| panic!("starts with {head:?}: {}", error.message))
        .to_owned()
}

/// A dry run's `plan` says what `profile show`'s `shown` says, for every spec pane but
/// `not_members` (the panes a spec names whose record is in no profile or another: `show`
/// lists them `missing`, while apply looks at every record a spec names and adopts it; the
/// caller asserts them on their own). `matches` is `adopt`/`planned`, `differs` is
/// `report`/`reported` with `diff` equal to show's row, `missing` is `create`/`planned`, and
/// the plan has no step beyond show's rows.
pub fn assert_plan_agrees_with_show(plan: &Applied, shown: &Value, not_members: &[&str]) {
    let rows: Vec<&Value> = shown["comparison"]
        .as_array()
        .unwrap_or_else(|| panic!("show's comparison: {shown}"))
        .iter()
        .filter(|row| row["status"] != "extra")
        .collect();
    assert_eq!(
        plan.steps.len(),
        rows.len(),
        "nothing beyond show's rows: {plan:#?} {shown}"
    );
    for row in rows {
        let pane = row["pane"].as_str().unwrap();
        if not_members.contains(&pane) {
            continue;
        }
        let step = plan.step(pane);
        match row["status"].as_str().unwrap() {
            "matches" => {
                assert_eq!(step.row(), (Action::Adopt, Outcome::Planned), "{pane}");
                assert_eq!(step.diff, Value::Null, "{pane}");
            }
            "differs" => {
                assert_eq!(step.row(), (Action::Report, Outcome::Reported), "{pane}");
                let want = json!({
                    "pane": row["pane"],
                    "status": row["status"],
                    "differences": row["differences"],
                });
                assert_eq!(step.diff, want, "{pane}");
            }
            "missing" => {
                assert_eq!(step.row(), (Action::Create, Outcome::Planned), "{pane}");
                assert_eq!(step.diff, Value::Null, "{pane}");
            }
            other => panic!("{pane}: {other}"),
        }
    }
}

/// One pane's part of a [`Still`]: its record's generation, profile and session of record
/// (`None` with no record), the pids tmux tags in its session, and whether its port refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneStill {
    pub record: Option<(u64, Value, Value)>,
    pub owned: Vec<u32>,
    pub refusing: bool,
}

/// What "apply left these panes alone" compares (decision 8): `pane list`'s registry rows,
/// each pane's [`PaneStill`], the profile's generation as `profile show` prints it, and
/// Herdr's snapshot (pane id, workspace and cell of every pane). Two equal readings around a
/// verb are the claim; every claim has a twin in the scenario where the same reading differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Still {
    pub registry: Value,
    pub panes: BTreeMap<String, PaneStill>,
    pub profile_generation: Value,
    pub herdr: BTreeSet<(String, String, String)>,
}

impl Still {
    /// Read `panes` (each with the port its spec names) and `profile` on `scratch`.
    pub fn of(scratch: &Scratch, panes: &[(&str, u16)], profile: &str) -> Still {
        let registry = scratch.json(&["pane", "list"], 0).data()["panes"].clone();
        let recorded: BTreeSet<&str> = registry
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| row["name"].as_str())
            .collect();
        let panes = panes
            .iter()
            .map(|&(pane, port)| {
                let record = recorded.contains(pane).then(|| {
                    let record = scratch.record(pane);
                    let generation = record["generation"].as_u64().expect("a generation");
                    (
                        generation,
                        record["profile"].clone(),
                        record["session_of_record"].clone(),
                    )
                });
                let mut owned = scratch.owned_pids(pane);
                owned.sort_unstable();
                let still = PaneStill {
                    record,
                    owned,
                    refusing: refused(port),
                };
                (pane.to_owned(), still)
            })
            .collect();
        let shown = scratch.json(&["profile", "show", profile], 0).data();
        let herdr = scratch
            .herdr_snapshot()
            .into_iter()
            .map(|p| {
                (
                    p.pane_id.as_str().to_owned(),
                    p.workspace,
                    json!(p.grid).to_string(),
                )
            })
            .collect();
        Still {
            registry,
            panes,
            profile_generation: shown["profile"]["generation"].clone(),
            herdr,
        }
    }

    /// `pane`'s part.
    pub fn pane(&self, pane: &str) -> &PaneStill {
        self.panes
            .get(pane)
            .unwrap_or_else(|| panic!("{pane} was read: {self:#?}"))
    }

    /// Decision 10's check against Herdr's own snapshot: `self` (read after a create) holds
    /// exactly the panes of `before`, at the same ids and cells, plus one: `pane_id` in
    /// `workspace` at `cell`.
    pub fn assert_one_herdr_pane_added(
        &self,
        before: &Still,
        pane_id: &str,
        workspace: &str,
        cell: GridPos,
    ) {
        assert!(
            before.herdr.is_subset(&self.herdr),
            "the earlier Herdr panes are where they were: {:#?} {:#?}",
            before.herdr,
            self.herdr
        );
        let added: Vec<_> = self.herdr.difference(&before.herdr).collect();
        let want = (
            pane_id.to_owned(),
            workspace.to_owned(),
            json!(cell).to_string(),
        );
        assert_eq!(added, [&want], "one new Herdr pane, at its cell");
    }

    /// The generation of `pane`'s record.
    pub fn generation(&self, pane: &str) -> u64 {
        self.pane(pane)
            .record
            .as_ref()
            .unwrap_or_else(|| panic!("{pane} has a record: {self:#?}"))
            .0
    }
}
