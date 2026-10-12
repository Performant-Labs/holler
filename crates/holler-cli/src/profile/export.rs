//! `holler profile export NAME [--out FILE]` (story #665, epic #633): write a profile as one
//! JSON document of its specs. The document's type, its writing and its reading live here, and
//! `holler profile import` reads it as `super::export::Document` (the frozen `profile/mod.rs`
//! admits no new module), as `create`, `delete` and `show` share their helpers.
//!
//! **The document** (D7, ADR-0021 section 3, "Profile files as built"), pretty-printed with two
//! spaces and a trailing newline: `{"format": "holler-profile", "version": 1, "name", "panes"}`.
//! Each pane is the spec's own serde value with one change, `herdr.grid` written `rRcC`, so its
//! members keep the spec's order (holler-cli's `serde_json` keeps insertion order), and a
//! member a spec leaves out (`opencode_agent`, `command`, `check`) stays out. The document holds
//! no slug, generation, timestamp or log, and no environment value: a spec holds the names of
//! its variables, and the verb reads nothing from the process environment.
//!
//! **Reading it back** ([`Document::read`], [`Document::specs`]): the text must decode as the
//! document, with no other member, `format` and `version` as written here, else `usage`; the
//! message gives the position and never quotes the text. A position may be any form
//! `GridPos::parse` reads, or the record's own object form. The specs then decode through
//! `holler_pane::decode_params`, so the typed guards answer with their own codes:
//! `profile-secret-refused`, `command-not-argv`, `env-name-invalid`, the grid codes. Any other
//! spec that does not decode is `usage`, again without quoting it: nothing scans the text for a
//! secret a second time, and nothing from the file is echoed.
//!
//! **The file** (D9): with `--out` the document is written by `write_atomic` at mode 0600,
//! replacing a file that exists, and nothing else is written. A path that names nothing the
//! verb can use is `usage`; another I/O failure is `unavailable` (ADR-0021 section 9, as
//! `pane import` classes its source root). Export writes nothing to the hub.

use std::io::{self, ErrorKind};
use std::path::Path;

use clap::Args;
use holler_pane::{decode_params, GridPos, PaneError, Ports, Profile, ProfileName, ProfileSpec};
use holler_proto::atomic_file::write_atomic;
use serde::{Deserialize, Serialize};
use serde_json::error::Category;
use serde_json::Value;

use super::show::text;
use crate::output::{emit, ErrorBody, VerbCtx};

/// The `format` of every profile file.
const FORMAT: &str = "holler-profile";

/// The document version this build writes and reads.
const VERSION: u64 = 1;

/// The mode of an exported file: names and non-secret values only, still the operator's own.
const FILE_MODE: u32 = 0o600;

/// Write a profile to a file, or print it: its specs as one JSON document.
///
/// The document is `{"format": "holler-profile", "version": 1, "name", "panes"}`, each pane a
/// spec with its position written `rRcC`. It holds names and non-secret values only: no slug,
/// generation, timestamp or log, and an environment variable by its name, never its value.
/// `holler profile import FILE` reads it back. With `--out FILE` it is written to FILE at mode
/// 0600, replacing a file there, and the verb prints `exported profile NAME (SLUG) to FILE: N
/// specs`; without, the document is the output.
///
/// `--format=json` prints one envelope whose data is `{"name", "slug", "specs", "out",
/// "document"}`: `out` is FILE or null, and `document` is the document when `out` is null, else
/// null.
#[derive(Args, Debug)]
pub struct ProfileExport {
    /// The profile's name (spaces allowed).
    pub name: String,
    /// Write the document to FILE, replacing it, instead of printing it.
    #[arg(long, value_name = "FILE")]
    pub out: Option<String>,
}

/// Run `holler profile export`: read the profile, build its document, write or print it once.
pub fn run(args: &ProfileExport, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = export(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// A profile file (see the module docs). Its JSON keys are its fields, in this order.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Document {
    format: String,
    version: u64,
    /// The profile's name, as the file spells it; a reader parses it only when it uses it.
    pub(crate) name: String,
    /// The specs, as the file holds them (positions `rRcC` when written here).
    panes: Vec<Value>,
}

impl Document {
    /// The document of `profile`.
    fn of(profile: &Profile) -> Result<Self, PaneError> {
        Ok(Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            name: profile.name.as_str().to_owned(),
            panes: profile
                .panes
                .iter()
                .map(spec_value)
                .collect::<Result<_, _>>()?,
        })
    }

    /// The document's text: pretty-printed with two spaces, with one newline at the end.
    fn text(&self) -> Result<String, PaneError> {
        let mut text = serde_json::to_string_pretty(self).map_err(|_| not_encoded())?;
        text.push('\n');
        Ok(text)
    }

    /// Read a profile file's text as a document of this format and version, else `usage`
    /// naming the position and never quoting the text.
    pub(crate) fn read(bytes: &[u8]) -> Result<Self, PaneError> {
        let document: Self = serde_json::from_slice(bytes).map_err(|error| {
            let at = format!("line {}, column {}", error.line(), error.column());
            not_a_document(&format!("{} at {at}", what_failed(&error)))
        })?;
        if document.format != FORMAT {
            return Err(not_a_document(&format!("its format is not {FORMAT:?}")));
        }
        if document.version != VERSION {
            return Err(not_a_document(&format!(
                "it is version {}, and this build reads version {VERSION}",
                document.version
            )));
        }
        Ok(document)
    }

    /// The document's specs: each position given as text read by `GridPos::parse`, then every
    /// spec decoded by the typed guards (see the module docs).
    pub(crate) fn specs(self) -> Result<Vec<ProfileSpec>, PaneError> {
        let mut panes = self.panes;
        for pane in &mut panes {
            read_position(pane)?;
        }
        panes
            .into_iter()
            .enumerate()
            .map(|(index, pane)| decode_params(pane).map_err(|error| spec_refused(index, error)))
            .collect()
    }
}

/// `spec` as the document holds it: its own serde value, with `herdr.grid` written `rRcC`.
fn spec_value(spec: &ProfileSpec) -> Result<Value, PaneError> {
    let mut value = serde_json::to_value(spec).map_err(|_| not_encoded())?;
    let grid = value
        .get_mut("herdr")
        .and_then(|herdr| herdr.get_mut("grid"))
        .ok_or_else(not_encoded)?;
    *grid = Value::String(spec.herdr.grid.to_string());
    Ok(value)
}

/// A spec's position, when the file gives it as text, read by `GridPos::parse` (its own
/// codes) and put back in the record's object form, which the spec's decode reads. Any other
/// shape is left to the decode.
fn read_position(pane: &mut Value) -> Result<(), PaneError> {
    let Some(grid) = pane
        .get_mut("herdr")
        .and_then(|herdr| herdr.get_mut("grid"))
    else {
        return Ok(());
    };
    if let Value::String(position) = grid {
        let cell = GridPos::parse(position)?;
        *grid = serde_json::to_value(cell).map_err(|_| not_encoded())?;
    }
    Ok(())
}

/// The error of the spec at `index` that did not decode: a typed guard's own code as it is,
/// and `usage` without serde's message, which can quote the file.
fn spec_refused(index: usize, error: PaneError) -> PaneError {
    match error {
        PaneError::Usage { .. } => not_a_document(&format!(
            "spec {} has a member that is unknown, missing or of the wrong type \
             (holler profile export writes the shape)",
            index + 1
        )),
        other => other,
    }
}

/// What kept a text from decoding as the document, by serde's category alone.
fn what_failed(error: &serde_json::Error) -> &'static str {
    match error.classify() {
        Category::Syntax => "it is not valid JSON",
        Category::Eof => "its JSON ends early",
        Category::Data => "its shape is wrong (a member unknown, missing or of the wrong type)",
        Category::Io => "it could not be read",
    }
}

/// `usage`: the file is not a profile document, for the reason `why`.
fn not_a_document(why: &str) -> PaneError {
    PaneError::Usage {
        message: format!("the file is not a holler-profile document (version {VERSION}): {why}"),
    }
}

/// A document that could not be encoded: a JSON value always encodes, so this is not met.
fn not_encoded() -> PaneError {
    PaneError::Unavailable {
        what: "the profile document could not be encoded".to_owned(),
    }
}

/// The error of the profile file at `path` that could not be `done` (`read`, `written`):
/// `usage` when the path names nothing the verb can use (nothing there, a missing or non-
/// directory parent, a directory, no file name), `unavailable` for any other I/O failure. It
/// names the path, escaped, and the kind of the error, never the file's content.
pub(crate) fn file_error(path: &str, done: &str, error: &io::Error) -> PaneError {
    let message = format!("{path:?} cannot be {done} ({})", error.kind());
    match error.kind() {
        ErrorKind::NotFound
        | ErrorKind::NotADirectory
        | ErrorKind::IsADirectory
        | ErrorKind::InvalidInput => PaneError::Usage { message },
        _ => PaneError::Unavailable { what: message },
    }
}

/// The data of `profile export`. Its JSON keys are its serialized fields, in this order.
#[derive(Debug, Serialize)]
struct Exported {
    name: ProfileName,
    slug: String,
    /// The number of specs.
    specs: usize,
    /// The file written, or `None` when the document is the output.
    out: Option<String>,
    /// The document when it is the output, else `None` (it is in the file).
    document: Option<Document>,
    /// The document's text, for the text form without `--out`.
    #[serde(skip)]
    text: String,
}

/// Read the profile (a bad name is `usage`, a missing one `profile-not-found`), build its
/// document, and with `--out` write it.
fn export(args: &ProfileExport, ports: Ports<'_>) -> Result<Exported, PaneError> {
    let name = ProfileName::parse(&args.name)?;
    let profile = ports
        .profile_store
        .get(&name)?
        .ok_or_else(|| PaneError::ProfileNotFound {
            what: format!("{:?}", name.as_str()),
        })?;
    let document = Document::of(&profile)?;
    let text = document.text()?;
    if let Some(out) = &args.out {
        write_atomic(Path::new(out), text.as_bytes(), FILE_MODE)
            .map_err(|error| file_error(out, "written", &error))?;
    }
    Ok(Exported {
        specs: profile.panes.len(),
        name: profile.name,
        slug: profile.slug,
        out: args.out.clone(),
        document: args.out.is_none().then_some(document),
        text,
    })
}

/// The text form: the document itself, or with `--out` `exported profile NAME (SLUG) to FILE:
/// N specs`, the path escaped as `profile show` escapes a stored string.
fn render(exported: &Exported) -> String {
    match &exported.out {
        Some(out) => format!(
            "exported profile {} ({}) to {}: {} specs",
            exported.name,
            exported.slug,
            text(out),
            exported.specs
        ),
        None => exported.text.clone(),
    }
}
