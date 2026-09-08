//! A2A 1.0 object model, reused verbatim on the inward wire (ADR 0004).
//!
//! `Message`, `Part`, and the task-state vocabulary are copied from the A2A
//! 1.0 JSON schema. **`Part` is member-name discriminated** — exactly one of
//! `text` | `raw` | `url` | `data` is present; the others are absent. This is
//! *not* the v0.3 kind-discriminated `{kind, content}` wire.
//!
//! A2A's JSON binding serialises enums as their **protocol-constant strings**
//! (`"ROLE_USER"`, `"TASK_STATE_WORKING"`, …); these types match that binding
//! exactly, so a `Part` on the Holler wire is byte-identical to a `Part` an
//! A2A client receives from the hub. See `tests/fixtures/a2a/README.md` for
//! the pinned revision (v1.0.1) and the examples these types must round-trip.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A2A's sender/role enum, as A2A's JSON binding encodes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    /// The role is unspecified.
    RoleUnspecified,
    /// The message is from the client to the server.
    RoleUser,
    /// The message is from the server to the client.
    RoleAgent,
}

/// A2A's task-state vocabulary, as A2A's JSON binding encodes it.
///
/// A2A states are **per task**. The subset Holler v2 exposes on the wire
/// (`working`, `input-required`, `completed`, `canceled`, `failed`,
/// `rejected`) is a subset of this. `idle` — a Holler *session* state, not an
/// A2A task state (ADR 0005) — has **no** A2A constant and so appears only on
/// the Holler wire (see `presence::SessionAd`), never in this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    TaskStateUnspecified,
    TaskStateSubmitted,
    TaskStateWorking,
    TaskStateInputRequired,
    TaskStateAuthRequired,
    TaskStateCompleted,
    TaskStateFailed,
    TaskStateCanceled,
    TaskStateRejected,
}

/// A section of a message (A2A `Part`).
///
/// A2A v1.0 parts are **member-name discriminated**: exactly one of `text`,
/// `raw`, `url`, or `data` is present, and its member name is the
/// discriminator. `filename`, `media_type`, and `metadata` are **optional
/// sibling fields** that may accompany any variant (A2A §4.1.6; the v0.3
/// `kind` discriminator and its wrappers were removed in 1.0). The wire form
/// is a flat object — `{"text":"hi"}`, `{"raw":"…","filename":"x","mediaType":"…"}`
/// — so a `Part` on the Holler wire is byte-identical to a `Part` an A2A
/// client receives from the hub.
///
/// On the wire `raw` is a base64 **string** (A2A's JSON binding); `data` is an
/// arbitrary JSON value. See [`Part::deserialize`]/[`Part::serialize`] for the
/// exact (de)serialisation, including the "at most one discriminator" rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// A text part.
    Text {
        /// The text content of the part.
        value: String,
        /// An optional filename (A2A allows this on any part).
        filename: Option<String>,
        /// The MIME type of the part content.
        media_type: Option<String>,
        /// Metadata associated with this part.
        metadata: Option<BTreeMap<String, Value>>,
    },
    /// A raw-bytes part (a file). `raw` is base64 on the wire.
    Raw {
        /// The raw bytes of the file.
        value: Vec<u8>,
        /// An optional filename (e.g. `"document.pdf"`).
        filename: Option<String>,
        /// The MIME type of the file (e.g. `"image/png"`).
        media_type: Option<String>,
        /// Metadata associated with this part.
        metadata: Option<BTreeMap<String, Value>>,
    },
    /// A URL part pointing at the file's content.
    Url {
        /// The URL.
        value: String,
        /// An optional filename (A2A allows this on any part).
        filename: Option<String>,
        /// The MIME type of the referenced content.
        media_type: Option<String>,
        /// Metadata associated with this part.
        metadata: Option<BTreeMap<String, Value>>,
    },
    /// A structured-data part (`data` is an arbitrary JSON value).
    Data {
        /// The structured data.
        value: Value,
        /// An optional filename (A2A allows this on any part).
        filename: Option<String>,
        /// The MIME type (e.g. `"application/json"`).
        media_type: Option<String>,
        /// Metadata associated with this part.
        metadata: Option<BTreeMap<String, Value>>,
    },
    /// A part with no content member — an empty object `{}`. A2A permits the
    /// OneOf members to all be absent; the shared fields alone are legal.
    Empty {
        /// An optional filename (A2A allows this on any part).
        filename: Option<String>,
        /// The MIME type.
        media_type: Option<String>,
        /// Metadata associated with this part.
        metadata: Option<BTreeMap<String, Value>>,
    },
}

impl Part {
    /// `Some` text when this is a `Text` part, else `None`.
    pub fn text(&self) -> Option<&str> {
        match self {
            Part::Text { value, .. } => Some(value),
            _ => None,
        }
    }
    /// `Some` bytes when this is a `Raw` part, else `None`.
    pub fn raw(&self) -> Option<&[u8]> {
        match self {
            Part::Raw { value, .. } => Some(value),
            _ => None,
        }
    }
    /// `Some` URL when this is a `Url` part, else `None`.
    pub fn url(&self) -> Option<&str> {
        match self {
            Part::Url { value, .. } => Some(value),
            _ => None,
        }
    }
    /// `Some` data when this is a `Data` part, else `None`.
    pub fn data(&self) -> Option<&Value> {
        match self {
            Part::Data { value, .. } => Some(value),
            _ => None,
        }
    }
    /// The shared `filename`, present on whichever variant carries it.
    pub fn filename(&self) -> Option<&str> {
        match self {
            Part::Text { filename, .. }
            | Part::Raw { filename, .. }
            | Part::Url { filename, .. }
            | Part::Data { filename, .. }
            | Part::Empty { filename, .. } => filename.as_deref(),
        }
    }
    /// The shared `media_type`.
    pub fn media_type(&self) -> Option<&str> {
        match self {
            Part::Text { media_type, .. }
            | Part::Raw { media_type, .. }
            | Part::Url { media_type, .. }
            | Part::Data { media_type, .. }
            | Part::Empty { media_type, .. } => media_type.as_deref(),
        }
    }
    /// The shared `metadata`.
    pub fn metadata(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Part::Text { metadata, .. }
            | Part::Raw { metadata, .. }
            | Part::Url { metadata, .. }
            | Part::Data { metadata, .. }
            | Part::Empty { metadata, .. } => metadata.as_ref(),
        }
    }
    /// Construct a `Text` part with no shared fields.
    pub fn text_part(value: impl Into<String>) -> Self {
        Part::Text {
            value: value.into(),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
    /// Construct a `Raw` part with no shared fields.
    pub fn raw_part(value: impl Into<Vec<u8>>) -> Self {
        Part::Raw {
            value: value.into(),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
    /// Construct a `Url` part with no shared fields.
    pub fn url_part(value: impl Into<String>) -> Self {
        Part::Url {
            value: value.into(),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
    /// Construct a `Data` part with no shared fields.
    pub fn data_part(value: Value) -> Self {
        Part::Data {
            value,
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
}

// --- Part (de)serialisation ------------------------------------------------
//
// A2A's JSON binding has no "untagged" discriminator member (the member name
// IS the discriminator), so serde's built-in tag representations cannot model
// it: an internally-tagged enum wraps the value in an extra key
// (`{"text":{"text":…}}`) and an untagged enum cannot report a clean error
// when zero or several discriminators are present. We implement it by hand,
// over a `serde_json::Value` (Holler's only wire is JSON), so that the wire
// form is exactly the flat object the A2A spec shows, and bad input yields a
// precise error.

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartWire {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, with = "base64_vec")]
    raw: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    metadata: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    media_type: Option<String>,
}

impl Serialize for Part {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let (text, raw, url, data, metadata, filename, media_type) = match self {
            Part::Text {
                value,
                filename,
                media_type,
                metadata,
            } => (
                Some(value.clone()),
                None,
                None,
                None,
                metadata.clone(),
                filename.clone(),
                media_type.clone(),
            ),
            Part::Raw {
                value,
                filename,
                media_type,
                metadata,
            } => (
                None,
                Some(value.clone()),
                None,
                None,
                metadata.clone(),
                filename.clone(),
                media_type.clone(),
            ),
            Part::Url {
                value,
                filename,
                media_type,
                metadata,
            } => (
                None,
                None,
                Some(value.clone()),
                None,
                metadata.clone(),
                filename.clone(),
                media_type.clone(),
            ),
            Part::Data {
                value,
                filename,
                media_type,
                metadata,
            } => (
                None,
                None,
                None,
                Some(value.clone()),
                metadata.clone(),
                filename.clone(),
                media_type.clone(),
            ),
            Part::Empty {
                filename,
                media_type,
                metadata,
            } => (
                None,
                None,
                None,
                None,
                metadata.clone(),
                filename.clone(),
                media_type.clone(),
            ),
        };
        PartWire {
            text,
            raw,
            url,
            data,
            metadata,
            filename,
            media_type,
        }
        .serialize(ser)
    }
}

// The workspace denies `expect` (issue #149): a helper panicking masks the
// caller. `Part::deserialize` is the one place an `expect` is still the
// honest choice — see the arm it guards. The item allow is linked to #149 so
// scripts/lint.sh admits it.
#[allow(clippy::expect_used, clippy::panic)] // #149
impl<'de> Deserialize<'de> for Part {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = PartWire::deserialize(d)?;
        // Count the OneOf discriminators that are present.
        let n = w.text.is_some() as u8
            + w.raw.is_some() as u8
            + w.url.is_some() as u8
            + w.data.is_some() as u8;
        let (filename, media_type, metadata) = (w.filename, w.media_type, w.metadata);
        match n {
            0 => Ok(Part::Empty {
                filename,
                media_type,
                metadata,
            }),
            1 => {
                if let Some(text) = w.text {
                    Ok(Part::Text {
                        value: text,
                        filename,
                        media_type,
                        metadata,
                    })
                } else if let Some(raw) = w.raw {
                    Ok(Part::Raw {
                        value: raw,
                        filename,
                        media_type,
                        metadata,
                    })
                } else if let Some(url) = w.url {
                    Ok(Part::Url {
                        value: url,
                        filename,
                        media_type,
                        metadata,
                    })
                } else {
                    // Exactly one discriminator is present (n == 1) and the other
                    // three are exhausted, so `data` is `Some` here: if it were
                    // `None` the invariant that this arm only runs when `data`
                    // is the one present discriminator would be broken, which
                    // `PartWire` (the single deserialisation point) cannot
                    // produce. `expect` is the honest way to say that.
                    Ok(Part::Data {
                        value: w.data.expect("exactly one discriminator present"),
                        filename,
                        media_type,
                        metadata,
                    })
                }
            }
            _ => Err(serde::de::Error::custom(
                "Part must have exactly one of `text`, `raw`, `url`, or `data` present",
            )),
        }
    }
}

/// A unit of communication between the two ends of the circuit (A2A `Message`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The unique identifier (e.g. a UUID) of the message.
    pub message_id: String,
    /// The context id the message is associated with (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// The task id the message is associated with (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// The sender of the message.
    pub role: Role,
    /// The container of the message content.
    pub parts: Vec<Part>,
    /// Optional metadata provided with the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, Value>>,
    /// URIs of extensions present or contributed to this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    /// Task ids this message references for additional context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_task_ids: Option<Vec<String>>,
}

// --- base64 (in-house; this crate is dependency-free beyond serde) ---------
// A2A's JSON binding encodes `bytes` as a base64 *string*; serde_json would
// otherwise serialise `Vec<u8>` as a JSON array of numbers.

const B64_ALPH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_ALPH[(n >> 18) as usize & 63] as char);
        out.push(B64_ALPH[(n >> 12) as usize & 63] as char);
        out.push(B64_ALPH[(n >> 6) as usize & 63] as char);
        out.push(B64_ALPH[n as usize & 63] as char);
    }
    // Apply standard base64 padding for a trailing group of 1 or 2 bytes.
    // The 3-byte-group loop above always emits 4 chars, so the last group has
    // zero-padded phantom bytes that must be removed:
    //   rem == 1 -> 1 real byte  -> 2 data chars + "=="
    //   rem == 2 -> 2 real bytes -> 3 data chars + "="
    // (rem == 0 -> full group, already 4 data chars, no padding.)
    match bytes.len() % 3 {
        1 => {
            out.pop(); // drop the phantom 4th char
            out.pop(); // drop the phantom 3rd char (zero-filled)
            out.push('=');
            out.push('=');
        }
        2 => {
            out.pop(); // drop the phantom 4th char
            out.push('=');
        }
        _ => {}
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    // +62 and /63 are already in B64_ALPH, so the table is complete as-is.
    let lookup: [i64; 256] = {
        let mut l = [-1i64; 256];
        for (i, &c) in B64_ALPH.iter().enumerate() {
            l[c as usize] = i as i64;
        }
        l
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4 + 1);
    let mut acc = 0i64;
    let mut bits = 0i64;
    // Base64 padding is a *run* of `=` at the end (0, 1, or 2 of them). `pad`
    // tracks that we've entered the run; any non-`=` char seen after padding
    // has begun is "trailing data after base64 padding". A second `=` is still
    // padding, not trailing data, so `seen_pad` is only latched once the run is
    // over — which, by construction, is never mid-string.
    let mut pad = 0i64;
    for ch in s.chars() {
        if pad > 0 && ch != '=' {
            return Err("trailing data after base64 padding".to_string());
        }
        if ch == '=' {
            pad += 1;
            if pad > 2 {
                return Err("too much base64 padding".to_string());
            }
            continue;
        }
        let v = *lookup.get(ch as usize).ok_or("bad base64 char")?;
        if v < 0 {
            return Err(format!("invalid base64 character {ch:?}"));
        }
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

mod base64_vec {
    use super::base64_decode;
    use super::base64_encode;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(bytes) => s.serialize_str(&base64_encode(bytes)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let Some(s) = Option::<String>::deserialize(d)? else {
            return Ok(None);
        };
        base64_decode(&s).map(Some).map_err(D::Error::custom)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #149: unit-test module — asserts panic on failure by design
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc_examples() {
        // RFC 4648 §10 vectors.
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_round_trips_arbitrary_bytes() {
        let bytes: Vec<u8> = (0..=255u16).map(|i| i as u8).cycle().take(700).collect();
        let enc = base64_encode(&bytes);
        assert_eq!(base64_decode(&enc).unwrap(), bytes);
    }
}
