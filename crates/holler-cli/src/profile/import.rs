//! `holler profile import FILE [--name NAME]` (story #665, epic #633): create a profile from a
//! file `holler profile export` wrote (its document is `super::export::Document`).
//!
//! It reads, checks, and only then writes, each failure writing nothing (D8):
//!
//! 1. FILE is a regular file of at most 1 MiB, else `usage`; another I/O failure is
//!    `unavailable` (`super::export::file_error`).
//! 2. Its text is a document of the format and version export writes, else `usage`.
//! 3. NAME is `--name` when given, else the document's name, through `ProfileName::parse`.
//! 4. and 5. The specs: each position through `GridPos::parse`, then each spec through the
//!    typed guards (`profile-secret-refused`, `command-not-argv`, `env-name-invalid`, the grid
//!    codes), as `Document::specs` says.
//! 6. NAME is free, by name or slug, else `profile-exists`; then the one create write
//!    (`super::create::insert_profile`, which reports the create race as `profile-exists` too).
//!
//! The profile is created **detached**, at generation 1, by the actor `holler profile import`:
//! no pane is read or written, and a spec may name a pane that has no record or belongs to
//! another profile, as `profile create --from` copies one. `holler profile apply` is what makes
//! the panes match it. No message quotes what the file holds, and nothing is read from the
//! process environment.

use std::fs::File;
use std::io::Read;

use clap::Args;
use holler_pane::profile_snapshot::profile_from_panes;
use holler_pane::{Actor, PaneError, Ports, Profile, ProfileName};
use serde::Serialize;

use super::create::{insert_profile, taken};
use super::export::{file_error, Document};
use crate::output::{emit, ErrorBody, VerbCtx};

/// Who the profile's change log says created it.
const ACTOR: &str = "holler profile import";

/// The most a profile file may hold, in bytes (1 MiB).
const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// Create a profile from a file `holler profile export` wrote; no pane joins it.
///
/// FILE holds one profile document. The profile takes the document's name, or NAME with
/// `--name`, and is created detached: no pane joins it, and `holler profile apply` is what makes
/// the panes match it. A position may be written `r2c1`, `c1r2` or `2,1`. A file that carries an
/// environment value is refused (`profile-secret-refused`), and no refusal prints what the file
/// holds. Prints `imported profile NAME (SLUG): N specs, generation 1`.
///
/// `--format=json` prints one envelope whose data is `{"profile"}`, the profile as stored.
#[derive(Args, Debug)]
pub struct ProfileImport {
    /// The file to read.
    pub file: String,
    /// The profile's name, in place of the one the file gives (spaces allowed).
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
}

/// Run `holler profile import`: read and check the file, create the profile, and print the
/// result once.
pub fn run(args: &ProfileImport, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = import(args, ctx.ports).map_err(|error| ErrorBody::from(&error));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `profile import`.
#[derive(Debug, Serialize)]
struct Imported {
    /// The profile as the store returned it.
    profile: Profile,
}

/// The steps of the module docs, in order; nothing is written before the last.
fn import(args: &ProfileImport, ports: Ports<'_>) -> Result<Imported, PaneError> {
    let bytes = read_file(&args.file)?;
    let document = Document::read(&bytes)?;
    let name = ProfileName::parse(args.name.as_deref().unwrap_or(&document.name))?;
    let panes = document.specs()?;
    let actor = Actor::parse(ACTOR)?;
    if let Some(stored) = ports.profile_store.get(&name)? {
        return Err(taken(&stored.name));
    }
    let profile = Profile {
        panes,
        ..profile_from_panes(&name, &[])
    };
    let profile = insert_profile(ports.profile_store, &profile, &actor)?;
    Ok(Imported { profile })
}

/// The bytes of the regular file at `path`, at most [`MAX_FILE_BYTES`] of them (step 1).
fn read_file(path: &str) -> Result<Vec<u8>, PaneError> {
    let metadata = std::fs::metadata(path).map_err(|error| file_error(path, "read", &error))?;
    if !metadata.is_file() {
        return Err(PaneError::Usage {
            message: format!("{path:?} is not a regular file"),
        });
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| file_error(path, "read", &error))?;
    if u64::try_from(bytes.len()).map_or(true, |read| read > MAX_FILE_BYTES) {
        return Err(PaneError::Usage {
            message: format!("{path:?} is over 1 MiB, the most a profile file holds"),
        });
    }
    Ok(bytes)
}

/// The text form: `imported profile NAME (SLUG): N specs, generation G`. A profile name holds
/// no control character (`ProfileName` refuses one) and a slug is ASCII letters, digits and `-`,
/// so each prints as it is.
fn render(imported: &Imported) -> String {
    let profile = &imported.profile;
    format!(
        "imported profile {} ({}): {} specs, generation {}",
        profile.name,
        profile.slug,
        profile.panes.len(),
        profile.generation
    )
}
