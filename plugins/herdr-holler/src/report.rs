//! The params of the plugin's two reports, built from registry records.
//!
//! - **Pane report:** one per registry pane, addressed by its Herdr pane id, under the
//!   source `holler`: the six display tokens `pos`, `project`, `shown`, `driven`,
//!   `sync` and `hold`, an empty `state_labels`, and `ttl_ms`.
//! - **Unknown report:** the same six tokens, each re-sent as `unknown`, with
//!   `state_labels: ["unknown"]` and `ttl_ms`. Herdr's merge of reports from one
//!   source is unverified, so dropping a token could leave its old value live; it is
//!   re-sent instead. The live report's empty `state_labels` clears the label for the
//!   same reason.
//! - **Workspace report:** one per Herdr workspace whose panes name a profile, token
//!   `profile`, and **no** `ttl_ms` (TTL is pane-only, Herdr spike section 4).

use std::collections::{BTreeMap, BTreeSet};

use holler_pane::pane::Hold;
use holler_pane::reconcile::shown_differs;
use holler_pane::{Pane, PaneId};
use serde_json::{json, Map, Value};

/// The one source id every report carries, so Herdr can clear them together.
const SOURCE: &str = "holler";

/// How long a pane report reads as current. Each event hook runs a one-shot refresh,
/// and SHOWN, DRIVEN and the hold change with no Herdr event, so this bounds how long
/// a fact the plugin is no longer refreshing stays on screen.
const PANE_TTL_MS: u64 = 120_000;

/// What a token holds when the record has no value (the CLI's `-`).
const NO_VALUE: &str = "-";

/// Every token and the state label of a report the hub could not back.
const UNKNOWN: &str = "unknown";

/// The pane report's token names, in the order [`pane_report`] fills them.
const PANE_TOKENS: [&str; 6] = ["pos", "project", "shown", "driven", "sync", "hold"];

/// The pane report of `record`: its grid cell, its project directory, the sessions
/// reconcile last observed as SHOWN and DRIVEN, SYNC and the hold.
pub(crate) fn pane_report(record: &Pane) -> Value {
    let observed = &record.last_observed;
    let values = [
        record.herdr.grid.to_string(),
        word(Some(record.host.cwd.as_str())),
        word(observed.shown.as_deref()),
        word(observed.driven.as_deref()),
        sync_word(record).to_owned(),
        hold_word(&record.hold).to_owned(),
    ];
    pane_params(&record.herdr.pane_id, &[], values)
}

/// The report that replaces a pane's facts when the hub cannot be read: every token
/// `unknown`, labelled `unknown`.
pub(crate) fn unknown_report(pane_id: &PaneId) -> Value {
    pane_params(pane_id, &[UNKNOWN], PANE_TOKENS.map(|_| UNKNOWN.to_owned()))
}

/// The workspace reports of `records`: per Herdr workspace, the profile its panes
/// name (several distinct ones sorted and joined with `,`). A workspace none of whose
/// panes names a profile gets no report.
pub(crate) fn workspace_reports(records: &[Pane]) -> Vec<Value> {
    let mut profiles: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for record in records {
        if let Some(profile) = &record.profile {
            profiles
                .entry(record.herdr.workspace.as_str())
                .or_default()
                .insert(profile.as_str());
        }
    }
    profiles
        .into_iter()
        .map(|(workspace_id, names)| {
            let profile = names.into_iter().collect::<Vec<_>>().join(",");
            json!({"workspace_id": workspace_id, "source": SOURCE, "tokens": {"profile": profile}})
        })
        .collect()
}

/// A pane report's params: the six tokens named by [`PANE_TOKENS`], in order.
fn pane_params(pane_id: &PaneId, state_labels: &[&str], values: [String; 6]) -> Value {
    let tokens: Map<String, Value> = PANE_TOKENS
        .iter()
        .zip(values)
        .map(|(name, value)| ((*name).to_owned(), Value::String(value)))
        .collect();
    json!({
        "pane_id": pane_id.as_str(),
        "source": SOURCE,
        "state_labels": state_labels,
        "tokens": tokens,
        "ttl_ms": PANE_TTL_MS,
    })
}

/// A stored value as a token: `-` when there is none or it is empty.
fn word(value: Option<&str>) -> String {
    match value {
        Some(text) if !text.is_empty() => text.to_owned(),
        _ => NO_VALUE.to_owned(),
    }
}

/// The `sync` token, the plugin's form of the CLI's SYNC cell: it mirrors
/// `SessionSync::of` (`crates/holler-cli/src/pane/list.rs`) case for case, so the
/// sidebar and `holler pane list` cannot drift. `-` unless reconcile has observed the
/// pane (`last_observed.at > 0`) and it has a session of record; then `mismatch` when
/// [`shown_differs`], the one SHOWN/DRIVEN rule, answers yes for
/// `last_observed.shown` (`None` is the home screen), and `ok` when it answers no.
fn sync_word(record: &Pane) -> &'static str {
    let observed = &record.last_observed;
    let of_record = record.session_of_record.as_deref();
    if observed.at <= 0 || of_record.is_none() {
        NO_VALUE
    } else if shown_differs(of_record, observed.shown.as_deref()) {
        "mismatch"
    } else {
        "ok"
    }
}

/// The `hold` token: the CLI's hold word (`hold_word`), `none`, `parked` or `drained`.
fn hold_word(hold: &Hold) -> &'static str {
    match hold {
        Hold::None => "none",
        Hold::Parked { .. } => "parked",
        Hold::Drained => "drained",
    }
}
