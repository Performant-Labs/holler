//! Difference between a `ProfileSpec` and the live pane it describes, the other half
//! of the comparison `profile show`, `profile apply` and the `profile-drift` finding
//! share. Empty stub declared by #637 so that no two stories edit `lib.rs`; story
//! #662 fills it.
//!
//! SIGNATURE STUB (#662 T, RED): the public API of the brief with no logic. Every body
//! returns a wrong-but-typed value so the tests compile and fail on their assertions. F
//! replaces the bodies.

use std::fmt;

use serde::{Serialize, Serializer};

use crate::argv::Argv;
use crate::grid::GridPos;
use crate::pane::Pane;
use crate::profile::{Profile, ProfileName, ProfileSpec};

/// A field of a `ProfileSpec`, named by its dotted JSON path. Serializes as that path.
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
    /// Every field, in the order `profile show` lists a spec and `diff_spec` compares.
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

    /// The dotted path, equal to the serde name (`"herdr.grid"`).
    pub const fn as_str(self) -> &'static str {
        ""
    }

    /// The value of this field in `spec`.
    pub fn value(self, spec: &ProfileSpec) -> FieldValue {
        let _ = spec;
        FieldValue::Text(String::new())
    }
}

impl Serialize for SpecField {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One field's value, typed so that it serializes exactly as the field does inside a `ProfileSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    Text(String),
    Grid(GridPos),
    Number(u32),
    List(Vec<String>),
    Argv(Option<Argv>),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = f;
        Ok(())
    }
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
    Matches,
    Differs,
    Missing,
    Extra,
}

/// One row of a comparison. `differences` is non-empty exactly when `status` is `Differs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneDiff {
    pub pane: String,
    pub status: PaneStatus,
    pub differences: Vec<FieldDiff>,
}

/// Whether `pane` is a member of `profile` (slugs compared on both sides).
pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
    let _ = (pane, profile);
    false
}

/// Every field where `live` differs from `spec`, in `SpecField::ALL` order.
pub fn diff_spec(spec: &ProfileSpec, live: &Pane) -> Vec<FieldDiff> {
    let _ = (spec, live);
    Vec::new()
}

/// The comparison of `profile` with the panes in `live`.
pub fn diff_profile(profile: &Profile, live: &[Pane]) -> Vec<PaneDiff> {
    let _ = (profile, live);
    Vec::new()
}
