//! Profiles (epic #633, decision 5): a named, stored set of pane specs, owned by the
//! hub's pane registry, and the two traits that work on them. Frozen by #637.
//!
//! - [`Profile`] and [`ProfileSpec`]: names and non-secret values only (I7), so the
//!   environment is [`EnvVarName`]s and a value cannot be represented. A profile has
//!   **no `log` field**: the append-only change log is read only through
//!   [`ProfileStore::log`], so `cas_put` cannot rewrite history.
//! - [`ProfileName`] (a display name) with the one [`ProfileName::slug`], and
//!   [`Actor`] (who made a write).
//! - [`ProfileStore`] (the registry port) and [`ProfileScope`] (the helper every
//!   `--profile` verb uses; implemented in `holler-cli`, #663).
//!
//! Like the pane record, these records refuse unknown fields and use milliseconds
//! since the Unix epoch as `i64` for every timestamp.

use std::fmt;

use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

use crate::argv::{AgentKey, Argv, EnvVarName};
use crate::error::{deserialize_parsed, excerpt, PaneError};
use crate::grid::GridPos;
use crate::pane::{ContextCeilings, HarnessKind, ModelSpec, Pane, PaneName, PaneRole};
use crate::ports::{Cursor, Watch};

/// The longest a profile name or an actor may be, in characters.
const MAX_NAME_CHARS: usize = 64;

/// The display name of a profile: spaces allowed, e.g. `Some Profile`.
///
/// Parsing trims surrounding whitespace and refuses a name that is empty, longer
/// than 64 characters, has a control character, or has no ASCII letter or digit
/// (the slug would be empty, and the slug is persisted and unique-checked by #661).
/// All of these are `usage`. Serde goes through [`ProfileName::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProfileName(String);

impl ProfileName {
    /// Parse a display name (see the type docs for the rules).
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        let name = text.trim();
        let refusal = if name.is_empty() {
            Some("a profile name must not be empty")
        } else if name.chars().count() > MAX_NAME_CHARS {
            Some("a profile name is at most 64 characters")
        } else if name.chars().any(char::is_control) {
            Some("a profile name must not contain control characters")
        } else if slugify(name).is_empty() {
            Some("a profile name needs at least one ASCII letter or digit")
        } else {
            None
        };
        match refusal {
            Some(rule) => Err(PaneError::Usage {
                message: format!("invalid profile name {}: {rule}", excerpt(text)),
            }),
            None => Ok(Self(name.to_owned())),
        }
    }

    /// The display name, verbatim (trimmed).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The unique id derived from the name: ASCII letters and digits in lower case,
    /// each run of anything else becoming one `-`, with no leading or trailing `-`.
    /// Two display names with the same slug are the same profile as far as
    /// uniqueness goes.
    pub fn slug(&self) -> String {
        slugify(&self.0)
    }
}

/// The slug of `name`; empty when it has no ASCII letter or digit.
fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut separator_pending = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if separator_pending && !slug.is_empty() {
                slug.push('-');
            }
            separator_pending = false;
            slug.push(c.to_ascii_lowercase());
        } else {
            separator_pending = true;
        }
    }
    slug
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProfileName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_parsed(deserializer, ProfileName::parse)
    }
}

/// Who made a profile write: a non-empty name of at most 64 characters (a person, a
/// verb such as `holler profile apply`, a watchdog). It becomes the "who" of a log
/// entry, so control characters are refused (`usage`), and surrounding whitespace is
/// trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Actor(String);

impl Actor {
    /// Parse an actor name (see the type docs).
    pub fn parse(text: &str) -> Result<Self, PaneError> {
        let name = text.trim();
        let refusal = if name.is_empty() {
            Some("an actor must not be empty")
        } else if name.chars().count() > MAX_NAME_CHARS {
            Some("an actor is at most 64 characters")
        } else if name.chars().any(char::is_control) {
            Some("an actor must not contain control characters")
        } else {
            None
        };
        match refusal {
            Some(rule) => Err(PaneError::Usage {
                message: format!("invalid actor {}: {rule}", excerpt(text)),
            }),
            None => Ok(Self(name.to_owned())),
        }
    }

    /// The actor name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Actor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_parsed(deserializer, Actor::parse)
    }
}

/// Where a spec places its pane in Herdr.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecHerdr {
    pub workspace: String,
    pub grid: GridPos,
}

/// The machine side of a spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecHost {
    /// The project directory or worktree.
    pub cwd: String,
}

/// The harness side of a spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecHarness {
    pub kind: HarnessKind,
    /// How the harness server's port is chosen (the `--port-policy` flag).
    pub port_policy: String,
}

/// One pane of a profile: names and non-secret values only (I7).
///
/// [`ProfileSpec::pane`] is plain text, not a [`PaneName`]: a spec may name a pane
/// that does not exist or belongs to another profile (a detached spec, e.g.
/// `profile create --from`). Membership is `Pane.profile` only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSpec {
    pub pane: String,
    pub herdr: SpecHerdr,
    pub host: SpecHost,
    pub harness: SpecHarness,
    pub model: ModelSpec,
    /// The OpenCode agent the pane's hub-delivered messages run as; `None` means the
    /// server's default agent (#700). A name, never a secret (I7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_agent: Option<AgentKey>,
    pub role: PaneRole,
    /// Environment variable NAMES only, never values.
    #[serde(default, deserialize_with = "crate::argv::deserialize_env_names")]
    pub env: Vec<EnvVarName>,
    pub context: ContextCeilings,
    /// The launch command: an argv array, never a shell string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Argv>,
    /// The health probe argv, e.g. `["curl","-s","http://127.0.0.1:8095/v1/models"]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<Argv>,
    /// Strings the probe output must contain.
    #[serde(default)]
    pub expect: Vec<String>,
}

/// A named, stored set of panes. Every write is a compare-and-swap on
/// [`Profile::generation`] (see [`crate::generation`]).
///
/// On read, the stored `slug` must equal the slug derived from the name: a record
/// where they disagree is corrupt or forged and does not load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawProfile")]
pub struct Profile {
    pub name: ProfileName,
    /// The unique id derived from the name ([`ProfileName::slug`]).
    pub slug: String,
    /// Bumped on every change.
    pub generation: u64,
    pub panes: Vec<ProfileSpec>,
    /// When the profile was created (milliseconds since the Unix epoch).
    pub created: i64,
    /// When it last changed (milliseconds since the Unix epoch).
    pub updated: i64,
}

/// The serde form of a [`Profile`] before the slug is checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfile {
    name: ProfileName,
    slug: String,
    generation: u64,
    #[serde(default)]
    panes: Vec<ProfileSpec>,
    created: i64,
    updated: i64,
}

impl TryFrom<RawProfile> for Profile {
    type Error = String;

    fn try_from(raw: RawProfile) -> Result<Self, String> {
        let expected = raw.name.slug();
        if raw.slug != expected {
            return Err(format!(
                "profile slug {} does not match its name {} (expected {})",
                excerpt(&raw.slug),
                excerpt(raw.name.as_str()),
                excerpt(&expected)
            ));
        }
        Ok(Profile {
            name: raw.name,
            slug: raw.slug,
            generation: raw.generation,
            panes: raw.panes,
            created: raw.created,
            updated: raw.updated,
        })
    }
}

/// What a spec edit does to a profile's entry for one pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecEdit {
    /// Set the pane's entry to this spec (boxed: a spec is large).
    Set(Box<ProfileSpec>),
    /// Remove the pane's entry.
    Remove,
}

/// What [`ProfileScope::resolve`] returns: the profile and the panes in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScope {
    pub profile: Profile,
    pub panes: Vec<Pane>,
}

/// What one write did to a profile, as the log records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileChange {
    Created,
    /// The spec set changed; `summary` says how, in one line.
    Updated {
        summary: String,
    },
    /// The profile was renamed from this name.
    Renamed {
        from: ProfileName,
    },
    Deleted,
}

/// One entry of a profile's append-only change log: who, when, the new generation and
/// what changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileLogEntry {
    /// When the write happened (milliseconds since the Unix epoch).
    pub at: i64,
    /// The generation the profile had after the write.
    pub generation: u64,
    pub actor: Actor,
    pub change: ProfileChange,
}

/// One change to the profile store, as `ProfileStore::watch` yields it. A rename
/// is a deletion of the old name followed by a put of the new one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileEvent {
    /// The store-wide sequence number of this change; hand it back as the `since` of
    /// the next watch to resume without a gap or a repeat.
    pub cursor: Cursor,
    /// The profile the change concerns.
    pub name: ProfileName,
    /// The record after the change; `None` when the profile was deleted.
    #[serde(default)]
    pub profile: Option<Box<Profile>>,
}

/// The registry of profiles, kept by the hub (#661) and faked by the test kit (#638).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
///
/// **Writes** are compare-and-swap on the profile's `generation` (see
/// [`crate::generation`]) and carry the [`Actor`] that made them; each appends one
/// [`ProfileLogEntry`]. A stale generation is `generation-conflict`.
pub trait ProfileStore: Send + Sync {
    /// The profile named `name`, or `None`.
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError>;

    /// Every profile.
    fn list(&self) -> Result<Vec<Profile>, PaneError>;

    /// Store `profile` if the stored one is still at `expected_generation` (0 for a
    /// new profile); returns the stored record with its bumped generation.
    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError>;

    /// Delete the profile `name` if it is still at `expected_generation`. A stale
    /// generation is `generation-conflict`; a profile that does not exist is
    /// `profile-not-found`, whatever `expected_generation` is (a missing profile is
    /// checked first, so no store has to guess which of the two to answer).
    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError>;

    /// The changes after `since`, in order (see [`Watch`] for the cursor rules).
    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError>;

    /// The change log of the profile `name`, oldest first.
    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError>;

    /// PROPOSED (#665): rename `from` to `to` if `from` is still at
    /// `expected_generation`. Until #665 is confirmed an implementation answers
    /// `not-implemented`.
    fn rename(
        &self,
        from: &ProfileName,
        to: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError>;
}

/// The helper every `--profile` verb uses to scope itself to a profile and to edit a
/// spec in one transaction with the live change. Implemented in
/// `holler-cli/src/pane/profile_scope.rs` (#663); frozen by #637.
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait ProfileScope: Send + Sync {
    /// The profile and the panes of it a verb acts on. With no `pane`, every pane
    /// of the profile; with a named pane, just that one, which must belong to the
    /// profile (`pane-not-in-profile` otherwise). A missing profile is
    /// `profile-not-found`.
    fn resolve(
        &self,
        profile: &ProfileName,
        pane: Option<&PaneName>,
    ) -> Result<ResolvedScope, PaneError>;

    /// Edit the spec of `pane` in `profile` and make the live change (`act`) as one
    /// transaction (I8): the profile is written with a compare-and-swap on its
    /// generation first, then `act` runs, then the result is recorded; if `act`
    /// fails nothing is recorded, and a conflict after `act` is `profile-conflict`.
    /// Returns the edited profile. With `profile: None` it runs only `act` and
    /// touches no profile (and returns `None`).
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError>;
}
