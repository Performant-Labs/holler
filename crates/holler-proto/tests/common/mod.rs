#![allow(dead_code)] // #155 — shared by codec_test and golden_test; not every fn is used by both
//! Shared helpers for the holler-proto test suite (issue #155 §4).

use serde_json::Value;
use std::fmt::Write as _;

/// A canonical (sorted-keys, one-line) JSON rendering, so comparisons are
/// independent of `serde_json::Map`'s iteration order and of whitespace.
pub fn canonical(v: &Value) -> String {
    let mut s = String::new();
    write_canonical(&mut s, v);
    s
}

fn write_canonical(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        Value::Number(n) => {
            let _ = write!(out, "{n}");
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap_or_default()),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(out, x);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                if let Some(x) = m.get(*k) {
                    write_canonical(out, x);
                }
            }
            out.push('}');
        }
    }
}

/// Canonical form of a JSON text (parses first).
pub fn canonical_str(text: &str) -> Result<String, serde_json::Error> {
    serde_json::from_str::<Value>(text).map(|v| canonical(&v))
}
