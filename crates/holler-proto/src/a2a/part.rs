//! A single A2A message part (v1.0.1) and its flat wire form.

use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single A2A message part (v1.0.1) — a flat object that carries **at most
/// one** of the one-of members `text` / `raw` / `url` / `data`, plus the
/// shared `filename` / `mediaType` / `metadata` fields. A2A's binding has no
/// type discriminator (the present member name is the discriminator);
/// deserialising is strict — zero members is a legal "empty" part, two or
/// more is an error. The in-memory one-of is an optional [`Content`]; the
/// wire keeps A2A's flat JSON via the private [`PartWire`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PartWire", into = "PartWire")]
pub struct Part {
    /// Which one-of member is present, or `None` for a part that carries none.
    pub content: Option<Content>,
    /// A short filename for the part (file-ish parts only).
    pub filename: Option<String>,
    /// The MIME type of the part (file-ish parts only).
    pub media_type: Option<String>,
    /// Arbitrary part metadata.
    pub metadata: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// A `text` part: a string.
    Text(String),
    /// A `raw` part: raw bytes, base64-encoded on the wire.
    Raw(Vec<u8>),
    /// A `url` part: a URI string.
    Url(String),
    /// A `data` part: an arbitrary JSON value.
    Data(Value),
}

/// The exact flat JSON shape of an A2A v1.0.1 `Part` (the wire form): each
/// optional member is absent when unset, so the present name is the discriminator.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PartWire {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<BTreeMap<String, Value>>,
}

impl TryFrom<PartWire> for Part {
    type Error = String;

    fn try_from(mut w: PartWire) -> Result<Self, Self::Error> {
        // A Part carries at most one OneOf member; two or more is a wire error.
        let n = [w.text.is_some(), w.raw.is_some(), w.url.is_some(), w.data.is_some()]
            .iter().filter(|b| **b).count();
        let content = match n {
            0 => None,
            1 => Some(match w.text {
                Some(v) => Content::Text(v),
                // `w.raw` / `w.url` / `w.data` are `None` here (n == 1); `?`
                // propagates an invalid-base64 `raw` as an error.
                None => match w.raw {
                    Some(v) => Content::Raw(B64.decode(v.as_bytes()).map_err(|e| e.to_string())?),
                    None => match w.url {
                        Some(v) => Content::Url(v),
                        None => Content::Data(w.data.take().unwrap_or(Value::Null)),
                    },
                },
            }),
            _ => return Err("a Part carries at most one of text/raw/url/data".to_owned()),
        };
        Ok(Part {
            content,
            filename: w.filename,
            media_type: w.media_type,
            metadata: w.metadata,
        })
    }
}

impl From<Part> for PartWire {
    fn from(p: Part) -> Self {
        let (text, raw, url, data) = match p.content {
            Some(Content::Text(v)) => (Some(v), None, None, None),
            Some(Content::Raw(bytes)) => (None, Some(B64.encode(&bytes)), None, None),
            Some(Content::Url(v)) => (None, None, Some(v), None),
            Some(Content::Data(v)) => (None, None, None, Some(v)),
            None => (None, None, None, None),
        };
        PartWire {
            text,
            raw,
            url,
            data,
            filename: p.filename,
            media_type: p.media_type,
            metadata: p.metadata,
        }
    }
}

impl Part {
    /// The `text` member, if this is a text part.
    #[inline]
    pub fn text(&self) -> Option<&str> {
        match self.content.as_ref() {
            Some(Content::Text(s)) => Some(s),
            _ => None,
        }
    }

    /// The decoded bytes of a `raw` part.
    #[inline]
    pub fn raw(&self) -> Option<&[u8]> {
        match self.content.as_ref() {
            Some(Content::Raw(b)) => Some(b),
            _ => None,
        }
    }

    /// The `url` member, if this is a url part.
    #[inline]
    pub fn url(&self) -> Option<&str> {
        match self.content.as_ref() {
            Some(Content::Url(s)) => Some(s),
            _ => None,
        }
    }

    /// The `data` member, if this is a data part.
    #[inline]
    pub fn data(&self) -> Option<&Value> {
        match self.content.as_ref() {
            Some(Content::Data(v)) => Some(v),
            _ => None,
        }
    }

    /// The shared `filename` / `mediaType` / `metadata` fields.
    #[inline]
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }
    #[inline]
    pub fn media_type(&self) -> Option<&str> {
        self.media_type.as_deref()
    }
    #[inline]
    pub fn metadata(&self) -> Option<&BTreeMap<String, Value>> {
        self.metadata.as_ref()
    }

    /// Constructors for each of the four one-of members.
    pub fn text_part(value: impl Into<String>) -> Self {
        Part {
            content: Some(Content::Text(value.into())),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
    pub fn raw_part(value: impl Into<Vec<u8>>) -> Self {
        Part {
            content: Some(Content::Raw(value.into())),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
    pub fn url_part(value: impl Into<String>) -> Self {
        Part {
            content: Some(Content::Url(value.into())),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
    pub fn data_part(value: Value) -> Self {
        Part {
            content: Some(Content::Data(value)),
            filename: None,
            media_type: None,
            metadata: None,
        }
    }
}
