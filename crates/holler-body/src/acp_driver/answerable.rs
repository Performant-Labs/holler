//! Resolution logic for "answerable blocking" (issue #188): turning a
//! caller-supplied `choice` string into the exact option(s) a pending
//! `session/request_permission` or `elicitation/create` request offered.
//!
//! Split out of `acp_driver.rs` on purpose (rather than folded into the
//! driver's connection/actor plumbing) so this pure, easily-unit-tested piece
//! stays small and keeps the parent file under the workspace's 900-line guard
//! (`scripts/lint.sh`). Mirrors the shape of `holler-client`'s
//! `resolve_question_choice`/`resolve_question_choices` (see the issue's
//! "Design reference"): a 0-based index, or an exact case-insensitive match
//! against the option's key or label; fail closed (no partial resolution) on
//! an unresolved segment or a segment-count mismatch.

/// One resolvable field's option set, in the order the request declared them.
/// `key` is what gets echoed back to the agent on the wire (a
/// `PermissionOptionId`, or an elicitation enum's declared value); `label` is
/// the human-readable name a caller may match against instead of the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSet {
    options: Vec<(String, String)>,
}

impl OptionSet {
    /// Build an option set from `(key, label)` pairs, in declaration order.
    pub fn new(options: Vec<(String, String)>) -> Self {
        Self { options }
    }

    /// The human-readable label of every option, in declaration order (issue
    /// #151: the roster's `PENDING` column and `session/presence`'s
    /// `PendingItem.options` render from these).
    pub fn labels(&self) -> Vec<String> {
        self.options.iter().map(|(_key, label)| label.clone()).collect()
    }

    /// Resolve one trimmed segment against this field's options: a 0-based
    /// index, or an exact case-insensitive match against the option's key or
    /// label. Returns the option's `key` (the wire value) on success.
    fn resolve(&self, segment: &str) -> Option<&str> {
        let trimmed = segment.trim();
        if let Ok(index) = trimmed.parse::<usize>() {
            if let Some((key, _label)) = self.options.get(index) {
                return Some(key.as_str());
            }
        }
        self.options
            .iter()
            .find(|(key, label)| {
                key.eq_ignore_ascii_case(trimmed) || label.eq_ignore_ascii_case(trimmed)
            })
            .map(|(key, _label)| key.as_str())
    }
}

/// Resolve `choice` against `fields`, one comma-separated segment per field,
/// in order (a single-field pending item is just the one-element case of
/// this). Fails closed: a segment-count mismatch or any unresolved segment
/// returns `Err` and resolves *nothing* — the caller must not send a partial
/// reply. On success, returns one resolved wire value per field, in the same
/// order as `fields`.
pub fn resolve_choice(fields: &[OptionSet], choice: &str) -> Result<Vec<String>, String> {
    if fields.is_empty() {
        return Err("no resolvable fields on this pending item".to_string());
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
            None => {
                return Err(format!(
                    "segment {index} ({segment:?}) does not resolve to any of this field's options"
                ));
            }
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
}
