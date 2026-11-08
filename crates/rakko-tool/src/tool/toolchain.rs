//! The version of a toolchain that a project pins
//!
//! A toolchain is a decision of the project, so the project pins its
//! version, and an action states only the range of versions that it accepts.
//! This module asks mise which versions the configuration of the project
//! pins, in the order of its pins, and picks the first one that the range
//! accepts.

use std::path::{Path, PathBuf};

use kawauso_process::{Execution, Invocation};
use rakko::{MiseTool, ProjectRoot, ToolVersion, VersionRange};
use semver::{Version, VersionReq};
use serde::Deserialize;

use super::{CURRENT, MISE, diagnosis};
use crate::tool::ResolveRequirementError;

/// The subcommand of mise that lists the versions of a tool
const LIST: &str = "ls";

/// The flag that limits a list to the versions that a configuration pins
const PINNED: &str = "--current";

/// The flag that asks for a list in JSON, with the source of each pin
const JSON: &str = "--json";

/// The details of a toolchain that only a configuration outside the project
/// pins, or that nothing pins
const NO_PROJECT_PIN: &str = "no configuration inside the project pins a version";

/// One version of a tool that mise lists, with the file that pins it
#[derive(Debug, Deserialize)]
struct Pin {
    /// The version that the pin resolves to
    version: String,

    /// Where the pin comes from, when it comes from a file
    #[serde(default)]
    source: Option<Source>,
}

/// The origin of a pin that mise lists
#[derive(Debug, Deserialize)]
struct Source {
    /// The configuration file that holds the pin
    #[serde(default)]
    path: Option<PathBuf>,
}

/// Returns the first version of a toolchain that the project pins and that
/// the range accepts
///
/// Only a pin in a configuration file inside the root of the project counts.
/// Mise also applies the global configuration of the user and the files of
/// the directories above the project. These files are different on each
/// machine, so a run on another machine then chooses another version. A local
/// mise file inside the project, such as `mise.local.toml`, counts. So a
/// developer can override the pin of the project on their own machine.
///
/// # Errors
///
/// Returns [`InvalidRange`][invalid] when the range is not a range,
/// [`MiseUnavailable`][unavailable] when mise does not run,
/// [`UnrecognizedReport`][unrecognized] when mise lists the pins in a form
/// that the crate does not read, and [`UnresolvedToolchain`][unresolved]
/// when no configuration inside the project pins the toolchain. When the
/// project pins versions and the range accepts none of them, returns
/// [`VersionOutOfRange`][outside] for the first pin with three numbers to
/// compare, or [`UnreadableVersion`][unreadable] when no pin has them.
///
/// [invalid]: ResolveRequirementError::InvalidRange
/// [outside]: ResolveRequirementError::VersionOutOfRange
/// [unavailable]: ResolveRequirementError::MiseUnavailable
/// [unreadable]: ResolveRequirementError::UnreadableVersion
/// [unrecognized]: ResolveRequirementError::UnrecognizedReport
/// [unresolved]: ResolveRequirementError::UnresolvedToolchain
pub(super) async fn pinned_toolchain(
    tool: &MiseTool,
    range: &VersionRange,
    root: &ProjectRoot,
) -> Result<ToolVersion, ResolveRequirementError> {
    let accepted =
        VersionReq::parse(range.get()).map_err(|source| ResolveRequirementError::InvalidRange {
            tool: tool.clone(),
            range: range.clone(),
            source,
        })?;

    let project = project_pins(tool, root).await?;

    let current = ask(tool, root, &[CURRENT, tool.get()]).await?;

    let output = current.stdout().to_string_lossy();
    let ordered = output
        .split_whitespace()
        .filter(|version| project.iter().any(|pin| pin == version));

    let mut outside: Option<&str> = None;
    let mut unreadable: Option<(&str, semver::Error)> = None;

    for pin in ordered {
        match Version::parse(pin) {
            Ok(version) if accepted.matches(&version) => return Ok(ToolVersion::new(pin)),
            Ok(_) => {
                outside.get_or_insert(pin);
            }
            Err(source) => {
                unreadable.get_or_insert((pin, source));
            }
        }
    }

    match (outside, unreadable) {
        (Some(pin), _) => Err(ResolveRequirementError::VersionOutOfRange {
            tool: tool.clone(),
            version: ToolVersion::new(pin),
            range: range.clone(),
        }),
        (None, Some((pin, source))) => Err(ResolveRequirementError::UnreadableVersion {
            tool: tool.clone(),
            version: ToolVersion::new(pin),
            source,
        }),
        (None, None) => Err(ResolveRequirementError::UnresolvedToolchain {
            tool: tool.clone(),
            details: NO_PROJECT_PIN.to_owned(),
        }),
    }
}

/// Returns the versions of a toolchain that a configuration inside the
/// project pins
///
/// The list of mise gives the file of each pin, but it sorts the versions,
/// so the caller takes their order from another answer of mise.
///
/// # Errors
///
/// Returns [`MiseUnavailable`][unavailable] when mise does not run,
/// [`UnresolvedToolchain`][unresolved] when mise ends without success, and
/// [`UnrecognizedReport`][unrecognized] when the list is not the JSON that
/// the crate reads.
///
/// [unavailable]: ResolveRequirementError::MiseUnavailable
/// [unrecognized]: ResolveRequirementError::UnrecognizedReport
/// [unresolved]: ResolveRequirementError::UnresolvedToolchain
async fn project_pins(
    tool: &MiseTool,
    root: &ProjectRoot,
) -> Result<Vec<String>, ResolveRequirementError> {
    let listing = ask(tool, root, &[LIST, PINNED, JSON, tool.get()]).await?;

    if !listing.status().success() {
        return Err(ResolveRequirementError::UnresolvedToolchain {
            tool: tool.clone(),
            details: diagnosis(&listing),
        });
    }

    let pins: Vec<Pin> = serde_json::from_slice(listing.stdout().get()).map_err(|source| {
        ResolveRequirementError::UnrecognizedReport {
            tool: tool.clone(),
            source,
        }
    })?;

    let inside = canonical(root.get());

    let project: Vec<String> = pins
        .into_iter()
        .filter(|pin| {
            pin.source
                .as_ref()
                .and_then(|source| source.path.as_deref())
                .is_some_and(|path| canonical(path).starts_with(&inside))
        })
        .map(|pin| pin.version)
        .collect();

    Ok(project)
}

/// Runs mise with the arguments in the root of the project
///
/// # Errors
///
/// Returns [`MiseUnavailable`][unavailable] when mise does not run.
///
/// [unavailable]: ResolveRequirementError::MiseUnavailable
async fn ask(
    tool: &MiseTool,
    root: &ProjectRoot,
    arguments: &[&str],
) -> Result<Execution, ResolveRequirementError> {
    Invocation::new(MISE)
        .args(arguments.iter().copied())
        .in_directory(root.get())
        .run()
        .await
        .map_err(|source| ResolveRequirementError::MiseUnavailable {
            tool: tool.clone(),
            source,
        })
}

/// Returns the path with its links resolved, or the path as it is when it
/// does not resolve
///
/// Mise gives the file of a pin with its links resolved, such as the
/// temporary directory of macOS under `/private`, so the root of the project
/// needs the same form before a comparison.
fn canonical(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
