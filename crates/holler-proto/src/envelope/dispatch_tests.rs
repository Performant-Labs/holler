//! Unit tests for the dispatch helper: projecting a call frame's raw
//! `params` into a method's typed params `T`.

#![allow(clippy::unwrap_used, clippy::panic)] // #143

use serde_json::json;

use crate::{Code, typed_params};

/// A test consumer for the dispatch helper (it would otherwise be dead code —
/// the in-process dispatcher lives in the hub/body crates).
#[derive(Debug, serde::Deserialize)]
struct Q {
    #[serde(default)]
    #[allow(dead_code)] // #146 — test helper for the dispatch projection
    n: Option<i32>,
}

/// A present `params` object decodes into the typed params; `None` params
/// decode from an empty object (a parameterless method); a wrong-shape
/// `params` is an `invalid_params` error (docs §8), not a framing error.
#[test]
fn typed_projects_raw_params_into_the_typed_params() {
    // A present `params` object decodes into the typed params.
    let p: Q = typed_params(Some(&json!({"n": 7}))).unwrap();
    assert_eq!(p.n, Some(7));
    // `None` params decodes from an empty object (a parameterless method).
    assert_eq!(typed_params::<Q>(None).unwrap().n, None);
    // A `params` of the wrong shape is an `invalid_params` error.
    let err = typed_params::<Q>(Some(&json!({"n": "not a number"}))).unwrap_err();
    assert_eq!(err.code, Code::InvalidParams.jsonrpc());
}
