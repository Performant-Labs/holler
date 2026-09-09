//! A tiny pure time-formatting helper, split out of `main.rs` (issue #190)
//! to keep that file under the workspace's 900-line build guard
//! (`scripts/lint.sh` check 4) — the same kind of split `query_cmd.rs`
//! (issue #185) and `say_cmd.rs` (issue #190) already made for their own
//! logic.

/// Format a unix epoch second as a local-time `YYYY-MM-DD HH:MM:SS` string
/// (the `hub token list`/`mint`/`ping` EXPIRES column). No chrono
/// dependency: a manual civil calendar conversion (Howard Hinnant's
/// algorithm) is enough for epoch seconds.
pub fn format_epoch(secs: u64) -> String {
    let days = (secs as i64) / 86400;
    let rem = secs as i64 % 86400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Days from 1970-01-01 → year, month, day (Hinnant's civil-from-days).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 3) / 153;
    let day = doy - (153 * mp + 2) / 5;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}
