//! The roster's own tiny RFC 3339 parser, split out of `roster.rs` (issue
//! #142) to keep that file under the workspace's 900-line build guard
//! (`scripts/lint.sh` check 4) — the same kind of split `session_manager.rs`/
//! `connection.rs`/`acp_driver.rs` already made for their own growth (a
//! `foo.rs` + `foo/bar.rs` pair, `mod` declared in the parent file).
//!
//! No `time`-crate dependency: the body always emits `SystemTime`-derived
//! UTC timestamps (`Z`/`+00:00`), so a manual 6-line civil-calendar
//! conversion (Howard Hinnant's `days_from_civil`) is enough, and it means
//! `holler-cli` doesn't need a second RFC3339 parser (or a new `time`-crate
//! dependency of its own) just to render "how long ago" — see
//! [`parse_rfc3339_secs_since`]'s own doc.

/// Parse an RFC3339 timestamp into whole epoch seconds, or `None` (an absent /
/// malformed timestamp is treated as "no update signal" — it never stalls a
/// row on its own). The body emits these from `SystemTime` (UTC, so the offset
/// is `Z`/`+00:00`); the parser accepts that profile and is lenient about a
/// fractional-second suffix. A non-UTC offset would be mis-parsed, but that
/// never happens on this wire — and even then the failure is only a missed
/// *display* hint (the row keeps showing `working`), never a correctness bug.
pub(crate) fn parse_rfc3339_secs(ts: Option<&str>) -> Option<u64> {
    ts.and_then(parse_rfc3339)
}

/// Seconds elapsed between `rfc3339` and now (the wall clock), or `None` on
/// an unparseable timestamp. **Public** (unlike [`parse_rfc3339`] itself):
/// `holler-cli`'s `roster_cmd.rs` renders the roster's `LAST TURN` column
/// from this rather than carrying a second RFC3339 parser (or a new
/// `time`-crate dependency) purely to compute "how long ago".
pub fn parse_rfc3339_secs_since(rfc3339: &str) -> Option<u64> {
    let then = parse_rfc3339(rfc3339)?;
    Some(crate::token::now_secs().saturating_sub(then))
}

/// Parse a single RFC3339 `YYYY-MM-DDTHH:MM:SS(.fff)?Z` timestamp to epoch
/// seconds. Returns `None` on any shape the body never produces.
///
/// `pub(crate)`: `control_server.rs`'s `wait` handler reuses this to turn a
/// matched row's `last_turn.ended_at` into an `age_secs` the CLI can render
/// as `<n>s/<n>m ago` with plain integer math — no second RFC3339 parser (and
/// no new `time`-crate dependency in `holler-cli`) for what is, on the wire,
/// exactly the same timestamp shape this module already parses.
pub(crate) fn parse_rfc3339(s: &str) -> Option<u64> {
    let bytes = s.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    // YYYY-MM-DD
    let year: i64 = s[0..4].parse().ok()?;
    if bytes[4] != b'-' {
        return None;
    }
    let month: i64 = s[5..7].parse().ok()?;
    if bytes[7] != b'-' {
        return None;
    }
    let day: i64 = s[8..10].parse().ok()?;
    // 'T' separator
    if !matches!(bytes[10], b'T' | b't' | b' ') {
        return None;
    }
    let hour: i64 = s[11..13].parse().ok()?;
    if bytes[13] != b':' {
        return None;
    }
    let minute: i64 = s[14..16].parse().ok()?;
    if bytes[16] != b':' {
        return None;
    }
    let second: i64 = s[17..19].parse().ok()?;
    // Optional fractional seconds: consume `.fff…` if present.
    let mut rest = 19usize;
    if bytes.get(rest) == Some(&b'.') {
        rest += 1;
        while rest < bytes.len() && bytes[rest].is_ascii_digit() {
            rest += 1;
        }
    }
    // Zone: accept a `Z` (or lowercase `z`) or a `+00:00` / `-00:00` offset.
    if matches!(bytes.get(rest), Some(&b'Z') | Some(&b'z')) {
        // (Trailing bytes after the zone, if any, are ignored — the body emits
        // exactly `...Z`.)
    } else if matches!(bytes.get(rest), Some(&b'+') | Some(&b'-')) {
        // A non-Z offset; only treat it as epoch-correct when it is UTC.
        if rest + 5 >= bytes.len() {
            return None;
        }
        let off = s.get(rest + 1..rest + 5)?;
        if off != "00:00" {
            return None;
        }
    } else {
        return None;
    }
    // The body emits post-1970 UTC timestamps, so the value is non-negative;
    // convert the `i64` day/time math to `u64` (rejecting any pre-epoch input).
    let secs = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    u64::try_from(secs).ok()
}

/// Howard Hinnant's `days_from_civil` — whole days since 1970-01-01 for a
/// proleptic Gregorian date. (The standard 6-line algorithm.)
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 {
        year - 1
    } else {
        year
    };
    let era = if y >= 0 {
        y
    } else {
        y - 399
    } / 400;
    let yoe = y - era * 400;
    let m = if month > 2 {
        month - 3
    } else {
        month + 9
    };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
