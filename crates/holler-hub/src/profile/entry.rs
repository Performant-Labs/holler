//! The profile registry's file entry (D2), the time a write is stamped with (D4) and the
//! one-line summary of an update (D5).
//!
//! # The file
//!
//! `<state dir>/hub/profiles.json` is the pane registry's versioned document
//! (`panes::persist`), with one [`ProfileEntry`] per slug, sorted by slug:
//!
//! ```text
//! {"version": 1, "cursor": <head>,
//!  "entries": [{"slug": "<slug>",
//!               "event": {"cursor": <c>, "name": "<display name>",
//!                         "profile": {<the Profile record>} or null},
//!               "log": [{"at": <ms>, "generation": <g>, "actor": "<who>",
//!                        "change": <the ProfileChange>}, ...]}, ...]}
//! ```
//!
//! - `cursor` is the change feed's head, so cursors keep increasing across a restart.
//! - `event` is the last change filed under the slug, a `ProfileEvent`: a put carries the
//!   record, and a delete leaves a tombstone (`"profile": null`) under the display name it
//!   was deleted by. A deletion therefore survives a restart, and a watcher that resumes
//!   across one still learns of it (`panes::feed`, rule 4).
//! - `log` is the profile's append-only change log, oldest first. It is kept here, beside
//!   the record and never inside it (ADR-0021 §7, and `Profile` has no `log` field). It is
//!   never cut: it outlives a delete and goes on across a re-create.
//!
//! The table keeps the same entry per slug in memory. The feed's ring holds the events
//! alone, without the logs.
//!
//! # Loading fails closed
//!
//! `panes::persist` checks what every registry file must hold, reading an entry through
//! `RegistryEntry`: one entry per slug, a cursor of at least 1 and at most the head, one
//! entry per cursor, and a record filed under its own slug at a generation of at least 1.
//! An entry adds its own checks when it is read:
//!
//! - its event names a profile of its slug, a tombstone's included;
//! - a live event names the profile exactly as its record does;
//! - its log is not empty;
//! - a live record's log ends with a write at the record's generation, and a tombstone's log
//!   ends with its deletion.
//!
//! A failed check is a serde data error, so the problem `persist` reports carries only its
//! line and column and never quotes the file.

use std::collections::BTreeMap;

use holler_pane::{Cursor, Profile, ProfileChange, ProfileEvent, ProfileLogEntry, ProfileSpec};
use serde::{Deserialize, Serialize};

use crate::panes::RegistryEntry;

/// One entry of `profiles.json`: the last change filed under a slug, and the profile's
/// change log. The table holds the same entry in memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RawEntry")]
pub(crate) struct ProfileEntry {
    /// The slug the entry is filed under (`ProfileName::slug`).
    pub(crate) slug: String,
    /// The last change: a put carrying the record, or a tombstone after a delete.
    pub(crate) event: ProfileEvent,
    /// The change log, oldest first. It is never empty.
    pub(crate) log: Vec<ProfileLogEntry>,
}

/// The serde form of a [`ProfileEntry`] before its checks (see the module docs).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    slug: String,
    event: ProfileEvent,
    log: Vec<ProfileLogEntry>,
}

impl TryFrom<RawEntry> for ProfileEntry {
    type Error = &'static str;

    fn try_from(raw: RawEntry) -> Result<Self, Self::Error> {
        check(&raw.slug, &raw.event, &raw.log)?;
        Ok(Self {
            slug: raw.slug,
            event: raw.event,
            log: raw.log,
        })
    }
}

/// The checks an entry adds to `persist`'s when it is read (see the module docs). The
/// messages name no content: the loader reports a failed check by its position only.
fn check(slug: &str, event: &ProfileEvent, log: &[ProfileLogEntry]) -> Result<(), &'static str> {
    if event.name.slug() != slug {
        return Err("the entry's event names a profile of another slug");
    }
    let Some(last) = log.last() else {
        return Err("the entry's change log is empty");
    };
    let deleted = matches!(last.change, ProfileChange::Deleted);
    match event.profile.as_deref() {
        Some(record) if record.name != event.name => {
            Err("the entry's event and record name the profile differently")
        }
        Some(record) if deleted || last.generation != record.generation => {
            Err("the entry's change log does not end with a write at its record's generation")
        }
        None if !deleted => Err("the entry's change log does not end with its deletion"),
        _ => Ok(()),
    }
}

impl ProfileEntry {
    /// The entry that a write filed under `slug` leaves: its `event`, and the log of
    /// `previous` (the entry filed under the slug before, a tombstone's included) with
    /// `logged` appended. The log therefore goes on across a delete and a re-create.
    pub(crate) fn next(
        previous: Option<&Self>,
        slug: String,
        event: ProfileEvent,
        logged: ProfileLogEntry,
    ) -> Self {
        let mut log = previous.map_or_else(Vec::new, |entry| entry.log.clone());
        log.push(logged);
        Self { slug, event, log }
    }

    /// The live record, or `None` for a tombstone.
    pub(crate) fn profile(&self) -> Option<&Profile> {
        self.event.profile.as_deref()
    }
}

/// `persist` checks a file's entries through this impl: one entry per slug, and a record
/// filed under its own slug.
impl RegistryEntry for ProfileEntry {
    /// The slug the entry is filed under.
    fn name(&self) -> &str {
        &self.slug
    }

    fn cursor(&self) -> Cursor {
        self.event.cursor
    }

    /// The record's own slug and generation.
    fn record(&self) -> Option<(&str, u64)> {
        self.profile()
            .map(|profile| (profile.slug.as_str(), profile.generation))
    }
}

/// The feed's ring and its rules (`panes::feed`) read the events through this impl, and
/// they read only `cursor()` and whether `record()` is `Some`. An event carries no slug of
/// its own (a tombstone has no record to borrow one from), so `name()` and the record's
/// name are the display name.
impl RegistryEntry for ProfileEvent {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn record(&self) -> Option<(&str, u64)> {
        self.profile
            .as_deref()
            .map(|profile| (profile.name.as_str(), profile.generation))
    }
}

/// The time a write to `entry` (`None` for a slug never written) is stamped with (D4):
/// `now`, unless the entry already holds a later time, the last `at` of its log or its
/// record's `updated`. The one stamp is the log entry's `at`, the record's `updated`, and a
/// create's `created`, so a log's `at` never decreases and a record's `updated` never goes
/// backwards, even when the clock steps back.
pub(crate) fn stamp(entry: Option<&ProfileEntry>, now: i64) -> i64 {
    entry.map_or(now, |entry| {
        let logged = entry.log.last().map_or(now, |last| last.at);
        let updated = entry.profile().map_or(now, |record| record.updated);
        now.max(logged).max(updated)
    })
}

/// The one-line summary of an update (D5): `pane specs: <before> -> <after> (<a> added,
/// <r> removed, <c> changed)`. Specs are matched by the pane they name, in order when two
/// name the same pane. It holds counts only, so it is one line by construction, and no text
/// from the request reaches the log.
pub(crate) fn summary(before: &[ProfileSpec], after: &[ProfileSpec]) -> String {
    let mut by_pane: BTreeMap<&str, (Vec<&ProfileSpec>, Vec<&ProfileSpec>)> = BTreeMap::new();
    for spec in before {
        by_pane.entry(&spec.pane).or_default().0.push(spec);
    }
    for spec in after {
        by_pane.entry(&spec.pane).or_default().1.push(spec);
    }
    let (mut added, mut removed, mut changed) = (0, 0, 0);
    for (old, new) in by_pane.values() {
        let kept = old.len().min(new.len());
        added += new.len() - kept;
        removed += old.len() - kept;
        changed += old.iter().zip(new).filter(|(was, is)| was != is).count();
    }
    format!(
        "pane specs: {} -> {} ({added} added, {removed} removed, {changed} changed)",
        before.len(),
        after.len()
    )
}
