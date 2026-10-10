//! The comparison of a profile's specs with its live panes (#662), field by field and
//! with typed values, so that `profile show`, `profile apply` (#664) and the
//! `profile-drift` finding (#665) report a difference the same way.
//!
//! - [`SpecField`] names each field of a spec by its dotted JSON path, from one table
//!   ([`SpecField::ALL`] and [`SpecField::as_str`]) that its `Serialize` reads too.
//! - [`FieldValue`] holds one field's value. It serializes exactly as the field does
//!   inside a `ProfileSpec` (a grid row first, through `GridPos`), and its `Display`
//!   prints it for a person with every control character escaped, so no stored string
//!   reaches a terminal raw.
//! - [`diff_spec`] compares one spec with one live pane through the snapshot
//!   ([`spec_from_pane`]), so a pane compared with its own snapshot never differs.
//!   [`diff_profile`] classifies every spec and every live pane of a profile
//!   ([`PaneStatus`]).
//! - [`is_member`] is what "a live pane of a profile" means: a pane record whose
//!   `profile` has the profile's slug (ADR-0021 section 3).
//!
//! Pure: no port is called.

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use serde::{Serialize, Serializer};

use crate::argv::Argv;
use crate::grid::GridPos;
use crate::pane::Pane;
use crate::profile::{Profile, ProfileName, ProfileSpec};
use crate::profile_snapshot::spec_from_pane;

/// A field of a `ProfileSpec`, named by its dotted JSON path. Serializes as that path
/// ([`SpecField::as_str`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecField {
    Workspace,
    Grid,
    Cwd,
    HarnessKind,
    PortPolicy,
    Provider,
    ModelId,
    Effort,
    Role,
    Env,
    ContextSoft,
    ContextHard,
    Command,
    Check,
    Expect,
}

impl SpecField {
    /// Every field, in the order `profile show` lists a spec and [`diff_spec`] compares
    /// it. `harness.port_policy` is compared like every other field: the live side is
    /// the policy of the port the pane's harness uses (ADR-0021 section 3).
    pub const ALL: [SpecField; 15] = [
        SpecField::Workspace,
        SpecField::Grid,
        SpecField::Cwd,
        SpecField::HarnessKind,
        SpecField::PortPolicy,
        SpecField::Provider,
        SpecField::ModelId,
        SpecField::Effort,
        SpecField::Role,
        SpecField::Env,
        SpecField::ContextSoft,
        SpecField::ContextHard,
        SpecField::Command,
        SpecField::Check,
        SpecField::Expect,
    ];

    /// The dotted path of the field in a `ProfileSpec`'s JSON (`"herdr.grid"`), which is
    /// also its serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            SpecField::Workspace => "herdr.workspace",
            SpecField::Grid => "herdr.grid",
            SpecField::Cwd => "host.cwd",
            SpecField::HarnessKind => "harness.kind",
            SpecField::PortPolicy => "harness.port_policy",
            SpecField::Provider => "model.provider",
            SpecField::ModelId => "model.model_id",
            SpecField::Effort => "model.effort",
            SpecField::Role => "role",
            SpecField::Env => "env",
            SpecField::ContextSoft => "context.soft",
            SpecField::ContextHard => "context.hard",
            SpecField::Command => "command",
            SpecField::Check => "check",
            SpecField::Expect => "expect",
        }
    }

    /// The value of this field in `spec`. `harness.kind` and `role` take their text from
    /// their serde form, so no variant name is written a second time.
    pub fn value(self, spec: &ProfileSpec) -> FieldValue {
        match self {
            SpecField::Workspace => FieldValue::Text(spec.herdr.workspace.clone()),
            SpecField::Grid => FieldValue::Grid(spec.herdr.grid),
            SpecField::Cwd => FieldValue::Text(spec.host.cwd.clone()),
            SpecField::HarnessKind => FieldValue::Text(serde_text(&spec.harness.kind)),
            SpecField::PortPolicy => FieldValue::Text(spec.harness.port_policy.clone()),
            SpecField::Provider => FieldValue::Text(spec.model.provider.clone()),
            SpecField::ModelId => FieldValue::Text(spec.model.model_id.clone()),
            SpecField::Effort => FieldValue::Text(spec.model.effort.clone()),
            SpecField::Role => FieldValue::Text(serde_text(&spec.role)),
            SpecField::Env => {
                FieldValue::List(spec.env.iter().map(|n| n.as_str().to_owned()).collect())
            }
            SpecField::ContextSoft => FieldValue::Number(spec.context.soft),
            SpecField::ContextHard => FieldValue::Number(spec.context.hard),
            SpecField::Command => FieldValue::Argv(spec.command.clone()),
            SpecField::Check => FieldValue::Argv(spec.check.clone()),
            SpecField::Expect => FieldValue::List(spec.expect.clone()),
        }
    }

    /// Whether `spec` and `live`, two values of this field, differ: `env` and `expect` as
    /// sets (an order or a repeat is not a difference), every other field by equality
    /// (an argv in order).
    fn differs(self, spec: &FieldValue, live: &FieldValue) -> bool {
        match (self, spec, live) {
            (SpecField::Env | SpecField::Expect, FieldValue::List(a), FieldValue::List(b)) => {
                as_set(a) != as_set(b)
            }
            _ => spec != live,
        }
    }
}

impl Serialize for SpecField {
    /// The dotted path ([`SpecField::as_str`]).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// The text of `value`'s serde form when that form is a JSON string (a unit variant's
/// name, such as a harness kind or a role), as `holler-cli`'s `--role` parser reads one.
/// Any other form gives an empty text, never a panic.
fn serde_text<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => String::new(),
    }
}

/// The distinct strings of `items`.
fn as_set(items: &[String]) -> BTreeSet<&str> {
    items.iter().map(String::as_str).collect()
}

/// One field's value, typed so that it serializes exactly as the field does inside a
/// `ProfileSpec` (a grid through `GridPos`'s own `Serialize`, row first) and prints for a
/// person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// `herdr.workspace`, `host.cwd`, `harness.kind`, `harness.port_policy`, the three
    /// `model` fields and `role`.
    Text(String),
    /// `herdr.grid`.
    Grid(GridPos),
    /// `context.soft`, `context.hard`.
    Number(u32),
    /// `env` (the names) and `expect`, in stored order.
    List(Vec<String>),
    /// `command`, `check`; `None` serializes as `null`.
    Argv(Option<Argv>),
}

impl fmt::Display for FieldValue {
    /// The text form:
    ///
    /// - `Text` with every control character escaped by `char::escape_default`, so ESC
    ///   prints as `\u{1b}` and a newline as `\n` (the rule Holler's text log lines
    ///   escape a field value with, in `holler_proto::log`); every other character as is.
    /// - `Grid` as `r2c1`, and `Number` in decimal.
    /// - `List` and `Argv(Some)` as a compact JSON array of strings, `["opencode","serve"]`,
    ///   never a line joined for a shell; `Argv(None)` as `none`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::Text(text) => write_escaped(f, text),
            FieldValue::Grid(grid) => write!(f, "{grid}"),
            FieldValue::Number(number) => write!(f, "{number}"),
            FieldValue::List(items) => write_json_strings(f, items),
            FieldValue::Argv(Some(argv)) => write_json_strings(f, argv.as_slice()),
            FieldValue::Argv(None) => f.write_str("none"),
        }
    }
}

/// `text` with each control character written as its `char::escape_default`.
fn write_escaped(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    for c in text.chars() {
        if c.is_control() {
            write!(f, "{}", c.escape_default())?;
        } else {
            f.write_char(c)?;
        }
    }
    Ok(())
}

/// `items` as a compact JSON array of strings. `serde_json` escapes the C0 controls
/// itself and writes DEL and the C1 controls raw; those are written here as JSON `\u`
/// escapes, so the text is still the JSON of the same strings and holds no control
/// character.
fn write_json_strings(f: &mut fmt::Formatter<'_>, items: &[String]) -> fmt::Result {
    // A list of strings always encodes, so the empty fallback is never used.
    let json = serde_json::to_string(items).unwrap_or_default();
    for c in json.chars() {
        if c.is_control() {
            write!(f, "\\u{:04x}", u32::from(c))?;
        } else {
            f.write_char(c)?;
        }
    }
    Ok(())
}

/// One field where a live pane differs from its spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldDiff {
    pub field: SpecField,
    pub spec: FieldValue,
    pub live: FieldValue,
}

/// How one pane of a comparison stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneStatus {
    /// A spec and a live pane of its name, equal on every compared field.
    Matches,
    /// A spec and a live pane of its name that differ on at least one compared field.
    Differs,
    /// A spec with no live pane of its name.
    Missing,
    /// A live pane that no spec names.
    Extra,
}

/// One row of a comparison. `differences` is non-empty exactly when `status` is `Differs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneDiff {
    pub pane: String,
    pub status: PaneStatus,
    pub differences: Vec<FieldDiff>,
}

/// Whether `pane` is a member of `profile`: its `profile` has `profile.slug()`. Slugs are
/// compared on both sides, since the slug is a profile's identity (a pane whose profile
/// is `SOME-PROFILE` is a member of `Some Profile`), like the test kit's fakes do.
///
/// This is what "a live pane of a profile" means (ADR-0021 section 3): `profile list`'s
/// live count, `profile show`'s comparison and `profile delete`'s refusal
/// `profile-has-live-panes` all use it.
pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
    pane.profile
        .as_ref()
        .is_some_and(|own| own.slug() == profile.slug())
}

/// Every field where `live` differs from `spec`, in `SpecField::ALL` order; empty when
/// they match. The names are not compared (the caller pairs them). `env` and `expect`
/// are compared as sets (sorted, duplicates dropped); every other field by equality.
/// Pure: it compares `spec` with `spec_from_pane(live)`, so
/// `diff_spec(&spec_from_pane(p), p)` is always empty.
pub fn diff_spec(spec: &ProfileSpec, live: &Pane) -> Vec<FieldDiff> {
    let observed = spec_from_pane(live);
    SpecField::ALL
        .into_iter()
        .filter_map(|field| {
            let (want, got) = (field.value(spec), field.value(&observed));
            field.differs(&want, &got).then_some(FieldDiff {
                field,
                spec: want,
                live: got,
            })
        })
        .collect()
}

/// The comparison of `profile` with the panes in `live` (the caller decides which panes
/// count as live; `profile show` passes the profile's members). One row per spec, in the
/// profile's order (`Matches` or `Differs` against the pane of `live` with that name,
/// else `Missing`), then one `Extra` row per pane of `live` that no spec names, in the
/// order of `live`.
pub fn diff_profile(profile: &Profile, live: &[Pane]) -> Vec<PaneDiff> {
    let rows = profile.panes.iter().map(|spec| {
        match live.iter().find(|pane| pane.name.as_str() == spec.pane) {
            Some(pane) => compared(spec, pane),
            None => row(&spec.pane, PaneStatus::Missing),
        }
    });
    let extras = live
        .iter()
        .filter(|pane| {
            !profile
                .panes
                .iter()
                .any(|spec| spec.pane == pane.name.as_str())
        })
        .map(|pane| row(pane.name.as_str(), PaneStatus::Extra));
    rows.chain(extras).collect()
}

/// The row of `spec` compared with `pane`, the live pane of its name.
fn compared(spec: &ProfileSpec, pane: &Pane) -> PaneDiff {
    let differences = diff_spec(spec, pane);
    let status = if differences.is_empty() {
        PaneStatus::Matches
    } else {
        PaneStatus::Differs
    };
    PaneDiff {
        pane: spec.pane.clone(),
        status,
        differences,
    }
}

/// A row with no difference.
fn row(pane: &str, status: PaneStatus) -> PaneDiff {
    PaneDiff {
        pane: pane.to_owned(),
        status,
        differences: Vec::new(),
    }
}
