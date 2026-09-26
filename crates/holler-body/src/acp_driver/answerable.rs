//! Resolution logic for "answerable blocking" (issue #188): turning a
//! caller-supplied `choice` string into the exact option(s) a pending
//! `session/request_permission` or `elicitation/create` request offered.
//!
//! Split out of `acp_driver.rs` on purpose (rather than folded into the
//! driver's connection/actor plumbing) so this pure, easily-unit-tested piece
//! stays small and keeps the parent file under the workspace's 900-line guard
//! (`scripts/lint.sh`). Mirrors the shape of the attach driver's
//! `resolve_question_choice`/`resolve_question_choices`
//! (`http_attach_driver/wire.rs`): one comma-separated segment per field,
//! failing closed (no partial resolution) on an unresolved segment or a
//! segment-count mismatch.
//!
//! One segment resolves against its field's options in this order, the first
//! match winning (issue #476):
//!
//! 1. a 0-based index;
//! 2. an exact, case-insensitive match against an option's key or label;
//! 3. on a permission field only, a shorthand word, matched
//!    case-insensitively: `once` selects the option of kind `allow_once`,
//!    `always` the option of kind `allow_always`, and `reject` the option of
//!    kind `reject_once`, else `reject_always` (see [`SHORTHANDS`]). A
//!    shorthand never selects an option of any other kind. When the
//!    permission offers none of the shorthand's kinds, the segment fails with
//!    an error that names the shorthand and lists the option labels.
//!
//! Any single-field pending item (a permission, or an elicitation with one
//! property) first tries the whole `choice` as its one segment, so a label
//! that contains a comma is selectable by that label (issue #477). Only when
//! the whole string misses does the comma split run, unchanged, so a bad
//! single choice fails with the same error as before. A pending item with two
//! or more fields always splits on commas.

/// The ACP kind of one permission option (issue #476). Crate-local so this
/// resolver stays free of SDK types: `pending.rs` maps the v1 and v2 SDK kinds
/// into it, and an SDK kind outside these four maps to no kind at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    /// Allow this operation only this time.
    AllowOnce,
    /// Allow this operation and remember the choice.
    AllowAlways,
    /// Reject this operation only this time.
    RejectOnce,
    /// Reject this operation and remember the choice.
    RejectAlways,
}

impl PermissionKind {
    /// The kind's ACP wire name, for error messages.
    fn wire_name(self) -> &'static str {
        match self {
            Self::AllowOnce => "allow_once",
            Self::AllowAlways => "allow_always",
            Self::RejectOnce => "reject_once",
            Self::RejectAlways => "reject_always",
        }
    }
}

/// The permission shorthands (issue #476) and the option kinds each one
/// selects, most preferred first. `reject` falls back to `reject_always` when
/// no `reject_once` option is offered; `once` and `always` have no fallback.
/// No shorthand lists a kind of the opposite decision, so `reject` can never
/// select an allow option, nor `once`/`always` a reject option.
const SHORTHANDS: [(&str, &[PermissionKind]); 3] = [
    ("once", &[PermissionKind::AllowOnce]),
    ("always", &[PermissionKind::AllowAlways]),
    ("reject", &[PermissionKind::RejectOnce, PermissionKind::RejectAlways]),
];

/// One option of a field. `key` is what gets echoed back to the agent on the
/// wire (a `PermissionOptionId`, or an elicitation enum's declared value);
/// `label` is the human-readable name a caller may match against instead of
/// the key; `kind` is the permission option's ACP kind (always `None` on an
/// elicitation field).
#[derive(Debug, Clone, PartialEq, Eq)]
struct OptionEntry {
    key: String,
    label: String,
    kind: Option<PermissionKind>,
}

/// One resolvable field's option set, in the order the request declared them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSet {
    options: Vec<OptionEntry>,
    /// `true` only for a permission field (built by [`OptionSet::permission`]),
    /// the one kind of field on which the shorthands resolve.
    permission: bool,
}

impl OptionSet {
    /// Build an elicitation field's option set from `(key, label)` pairs, in
    /// declaration order. Its options carry no kind, so no shorthand resolves.
    pub fn new(options: Vec<(String, String)>) -> Self {
        let options = options
            .into_iter()
            .map(|(key, label)| OptionEntry { key, label, kind: None })
            .collect();
        Self { options, permission: false }
    }

    /// Build a permission field's option set from `(key, label, kind)`
    /// triples, in declaration order (issue #476). `kind` is `None` for an
    /// option whose ACP kind is not one of the four [`PermissionKind`]s, so no
    /// shorthand can select it.
    pub fn permission(options: Vec<(String, String, Option<PermissionKind>)>) -> Self {
        let options = options
            .into_iter()
            .map(|(key, label, kind)| OptionEntry { key, label, kind })
            .collect();
        Self { options, permission: true }
    }

    /// The human-readable label of every option, in declaration order (issue
    /// #151: the roster's `PENDING` column and `session/presence`'s
    /// `PendingItem.options` render from these).
    pub fn labels(&self) -> Vec<String> {
        self.options.iter().map(|option| option.label.clone()).collect()
    }

    /// Resolve one segment against this field's options, trimmed, in the
    /// module doc's order: a 0-based index, then an exact case-insensitive
    /// key or label, then (permission fields only) a shorthand word. Returns
    /// the option's `key` (the wire value) on success.
    fn resolve(&self, segment: &str) -> Option<&str> {
        let trimmed = segment.trim();
        trimmed
            .parse::<usize>()
            .ok()
            .and_then(|index| self.options.get(index))
            .or_else(|| {
                self.options.iter().find(|option| {
                    option.key.eq_ignore_ascii_case(trimmed)
                        || option.label.eq_ignore_ascii_case(trimmed)
                })
            })
            .or_else(|| self.by_shorthand(trimmed))
            .map(|option| option.key.as_str())
    }

    /// The kinds `word` selects, most preferred first, when this is a
    /// permission field and `word` is a shorthand (case-insensitively);
    /// `None` otherwise.
    fn shorthand_kinds(&self, word: &str) -> Option<&'static [PermissionKind]> {
        if !self.permission {
            return None;
        }
        SHORTHANDS
            .iter()
            .find(|(shorthand, _kinds)| shorthand.eq_ignore_ascii_case(word))
            .map(|(_shorthand, kinds)| *kinds)
    }

    /// The option a shorthand `word` selects: for each of its kinds in
    /// preference order, the first offered option of that kind (in declaration
    /// order, as a duplicate key or label resolves to its first option).
    fn by_shorthand(&self, word: &str) -> Option<&OptionEntry> {
        self.shorthand_kinds(word)?.iter().find_map(|kind| {
            self.options.iter().find(|option| option.kind == Some(*kind))
        })
    }

    /// Why `segment` (at `index`) did not resolve. A shorthand on a
    /// permission field names the kinds it looked for and lists the option
    /// labels (issue #476); any other miss keeps the resolver's original
    /// wording.
    fn unresolved(&self, index: usize, segment: &str) -> String {
        let Some(kinds) = self.shorthand_kinds(segment.trim()) else {
            return format!(
                "segment {index} ({segment:?}) does not resolve to any of this field's options"
            );
        };
        let wanted: Vec<&str> = kinds.iter().map(|kind| kind.wire_name()).collect();
        let labels: Vec<String> =
            self.options.iter().map(|option| format!("{:?}", option.label)).collect();
        format!(
            "segment {index} ({segment:?}) does not resolve: this permission offers no option \
             of kind {}; its options are {}",
            wanted.join(" or "),
            labels.join(", ")
        )
    }
}

/// Resolve `choice` against `fields`, one comma-separated segment per field,
/// in order. A single-field pending item first tries the whole `choice` as
/// its one segment (issue #477), and on a miss falls through to the same
/// split as any other item. Fails closed: a segment-count mismatch or any
/// unresolved segment returns `Err` and resolves *nothing* — the caller must
/// not send a partial reply. On success, returns one resolved wire value per
/// field, in the same order as `fields`.
pub fn resolve_choice(fields: &[OptionSet], choice: &str) -> Result<Vec<String>, String> {
    if fields.is_empty() {
        return Err("no resolvable fields on this pending item".to_string());
    }
    if let [field] = fields {
        if let Some(key) = field.resolve(choice) {
            return Ok(vec![key.to_string()]);
        }
    }
    let segments: Vec<&str> = choice.split(',').collect();
    if segments.len() != fields.len() {
        return Err(format!(
            "expected {} comma-separated choice segment(s) for {} field(s), got {}",
            fields.len(),
            fields.len(),
            segments.len()
        ));
    }
    let mut resolved = Vec::with_capacity(fields.len());
    for (index, (field, segment)) in fields.iter().zip(segments.iter()).enumerate() {
        match field.resolve(segment) {
            Some(key) => resolved.push(key.to_string()),
            None => return Err(field.unresolved(index, segment)),
        }
    }
    Ok(resolved)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #188
mod tests {
    use super::*;

    fn opts(pairs: &[(&str, &str)]) -> OptionSet {
        OptionSet::new(
            pairs
                .iter()
                .map(|(k, l)| (k.to_string(), l.to_string()))
                .collect(),
        )
    }

    #[test]
    fn resolves_by_zero_based_index() {
        let fields = vec![opts(&[("allow", "Allow"), ("deny", "Deny")])];
        assert_eq!(
            resolve_choice(&fields, "0").unwrap(),
            vec!["allow".to_string()]
        );
        assert_eq!(
            resolve_choice(&fields, "1").unwrap(),
            vec!["deny".to_string()]
        );
    }

    #[test]
    fn resolves_by_key_or_label_case_insensitively() {
        let fields = vec![opts(&[("allow", "Allow this"), ("deny", "Deny this")])];
        assert_eq!(
            resolve_choice(&fields, "ALLOW").unwrap(),
            vec!["allow".to_string()]
        );
        assert_eq!(
            resolve_choice(&fields, "deny this").unwrap(),
            vec!["deny".to_string()]
        );
    }

    #[test]
    fn unresolvable_single_choice_fails_closed() {
        let fields = vec![opts(&[("allow", "Allow"), ("deny", "Deny")])];
        assert!(resolve_choice(&fields, "maybe").is_err());
        assert!(resolve_choice(&fields, "5").is_err());
    }

    #[test]
    fn multi_field_comma_separated_resolves_each_independently() {
        let fields = vec![
            opts(&[("red", "Red"), ("blue", "Blue")]),
            opts(&[("s", "Small"), ("m", "Medium")]),
        ];
        assert_eq!(
            resolve_choice(&fields, "red,m").unwrap(),
            vec!["red".to_string(), "m".to_string()]
        );
        assert_eq!(
            resolve_choice(&fields, "1,0").unwrap(),
            vec!["blue".to_string(), "s".to_string()]
        );
    }

    #[test]
    fn multi_field_wrong_segment_count_fails_closed() {
        let fields = vec![
            opts(&[("red", "Red"), ("blue", "Blue")]),
            opts(&[("s", "Small"), ("m", "Medium")]),
        ];
        assert!(resolve_choice(&fields, "red").is_err());
        assert!(resolve_choice(&fields, "red,m,extra").is_err());
    }

    #[test]
    fn multi_field_any_unresolved_segment_fails_the_whole_choice() {
        let fields = vec![
            opts(&[("red", "Red"), ("blue", "Blue")]),
            opts(&[("s", "Small"), ("m", "Medium")]),
        ];
        assert!(resolve_choice(&fields, "red,xl").is_err());
    }

    #[test]
    fn empty_field_list_is_an_error_not_a_vacuous_success() {
        assert!(resolve_choice(&[], "anything").is_err());
    }

    /// The label the real Codex adapter gives its reject option (#477).
    const CODEX_REJECT: &str = "No, and tell Codex what to do differently";

    #[test]
    fn single_field_label_containing_a_comma_resolves_as_the_whole_string() {
        // #477 / AC5: a single-field item tries the whole choice before any
        // comma split, so a comma inside a label is selectable by that label.
        let fields = vec![opts(&[("approved", "Proceed"), ("abort", CODEX_REJECT)])];
        let resolved = resolve_choice(&fields, CODEX_REJECT);
        assert_eq!(resolved, Ok(vec!["abort".to_string()]), "whole comma label must resolve");
        let lower = CODEX_REJECT.to_lowercase();
        assert_eq!(
            resolve_choice(&fields, &lower),
            Ok(vec!["abort".to_string()]),
            "the whole-string match is case-insensitive, like any label match"
        );
    }

    #[test]
    fn single_field_comma_label_no_longer_hits_the_segment_count_error() {
        // AC7: the live #477 failure text must not occur for the whole label.
        let fields = vec![opts(&[("approved", "Proceed"), ("abort", CODEX_REJECT)])];
        if let Err(e) = resolve_choice(&fields, CODEX_REJECT) {
            panic!("the Codex reject label must resolve, got: {e}");
        }
    }

    #[test]
    fn single_field_unmatched_comma_choice_keeps_todays_error_text() {
        // Decision 4: when the whole string misses, the comma split runs
        // unchanged, so a bad single choice reports exactly what it does today.
        let fields = vec![opts(&[("allow", "Allow"), ("deny", "Deny")])];
        let err = resolve_choice(&fields, "nope, nothing").unwrap_err();
        assert_eq!(err, "expected 1 comma-separated choice segment(s) for 1 field(s), got 2");
    }

    #[test]
    fn two_field_prompt_still_splits_on_commas_even_when_a_label_has_one() {
        // AC5: whole-string-first is single-field only; a two-field item keeps
        // one comma segment per field, so a comma label is not matched whole.
        let fields = vec![
            opts(&[("go", "Yes, go"), ("no", "No")]),
            opts(&[("s", "Small"), ("m", "Medium")]),
        ];
        assert!(resolve_choice(&fields, "Yes, go").is_err(), "must split, not match the label whole");
        assert_eq!(
            resolve_choice(&fields, "no,m").unwrap(),
            vec!["no".to_string(), "m".to_string()]
        );
    }

    #[test]
    fn shorthand_words_do_not_resolve_on_a_kindless_elicitation_field() {
        // AC4: shorthands are permission-only; an elicitation field (built by
        // `OptionSet::new`, no kinds) fails closed on each of them.
        let fields = vec![opts(&[("red", "Red"), ("blue", "Blue")])];
        for word in ["once", "always", "reject", "ONCE"] {
            assert!(resolve_choice(&fields, word).is_err(), "{word:?} must not resolve on an elicitation field");
        }
    }

    #[test]
    fn permission_shorthand_misses_fail_closed_with_the_kind_and_labels() {
        // Decision 3 / Risk: a `reject` with only allow options, or any shorthand
        // on a permission whose options all have unknown kinds, fails with the
        // shorthand-specific error (never the generic one, never a match).
        let perm = |opts: &[(&str, &str, Option<PermissionKind>)]| {
            vec![OptionSet::permission(
                opts.iter().map(|(k, l, kind)| (k.to_string(), l.to_string(), *kind)).collect(),
            )]
        };
        let allow_only = perm(&[
            ("a1", "Proceed", Some(PermissionKind::AllowOnce)),
            ("a2", "Proceed always", Some(PermissionKind::AllowAlways)),
        ]);
        let err = resolve_choice(&allow_only, "reject").unwrap_err();
        assert!(err.contains("reject_once or reject_always"), "names both kinds: {err:?}");
        assert!(err.contains("\"Proceed\", \"Proceed always\""), "lists the labels: {err:?}");
        let unknown = perm(&[("x", "Custom", None), ("y", "Other", None)]);
        for word in ["once", "always", "reject"] {
            let err = resolve_choice(&unknown, word).unwrap_err();
            assert!(err.contains("offers no option of kind"), "{word:?}: {err:?}");
        }
        // Several options of one kind: the first declared wins, as a duplicate
        // key or label does.
        let dup = perm(&[
            ("r1", "Stop", Some(PermissionKind::RejectOnce)),
            ("r2", "Halt", Some(PermissionKind::RejectOnce)),
        ]);
        assert_eq!(resolve_choice(&dup, "reject"), Ok(vec!["r1".to_string()]));
    }
}
