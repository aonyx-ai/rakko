/// The arguments that a run of the command reads
mod args;
/// The errors that stop a run of the command
mod error;
/// The version in the root manifest
mod manifest;
/// The Rust pins in the configuration of mise
mod pins;

use std::path::Path;

use rakko_action::{Context, Name, action_name};
use rakko_cli::Command;
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_tool::Invocation;
use toml_edit::{DocumentMut, Value};

pub use self::args::{Msrv, Reason, SetMsrvArgs};
pub use self::error::SetMsrvError;

/// The root manifest of a project
const MANIFEST: &str = "Cargo.toml";

/// The configuration of mise that pins the toolchains of a project
const CONFIGURATION: &str = "mise.toml";

/// The lock in which mise records what each pin resolved to
const LOCK: &str = "mise.lock";

/// The program that locks the pins of a project
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches the command reaches mise as well.
const MISE: &str = "mise";

/// The arguments that ask mise to lock the Rust pins of the project again
const LOCK_RUST: [&str; 2] = ["lock", "rust"];

/// The details of a run of mise that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// The command that sets the minimum supported Rust version of a project
///
/// Three places state the version, and all three must agree: the
/// `rust-version` of the root manifest, the Rust pin in `mise.toml` that
/// check-msrv runs the compiler on, and the entry of that pin in
/// `mise.lock`. A run writes the version and its reason to the manifest,
/// moves the pin that matched the old version, and asks mise to lock the
/// pins again.
///
/// A run changes only the version, its comment, the pin, and the lock, so a
/// reviewer reads a diff of a few lines. Mise locks every Rust pin again, so
/// in a project whose pins name exact versions, only the entry of the moved
/// pin changes in the lock. It installs nothing. The user runs
/// `mise install` and check-msrv afterwards.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct SetMsrv;

/// The characters that end a line in a file of the project
///
/// The TOML documents of a run end every line with a line feed, so a file
/// that ends its lines with a carriage return as well gets them back when
/// the run writes it.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum LineEnding {
    /// A line feed alone
    LineFeed,

    /// A carriage return and a line feed
    CarriageReturnLineFeed,
}

impl Command for SetMsrv {
    type Args = SetMsrvArgs;

    // setmsrv[impl name]
    fn name(&self) -> Name {
        action_name!("set-msrv")
    }

    /// Sets the version in the manifest, the pin, and the lock
    ///
    /// # Errors
    ///
    /// Returns an error when a file cannot be read, parsed, or written, when
    /// the manifest declares no version or declares it in an inline table,
    /// when no pin matches the version of the manifest, and when mise does
    /// not lock the new pin.
    async fn run(
        &self,
        project: &Context,
        _clawless: &ClawlessContext,
        args: &Self::Args,
    ) -> CommandResult {
        set(project.root().get(), args).await?;

        Ok(())
    }
}

/// Sets the version of the project in its root directory
///
/// The run finds everything that it changes before it writes the first file,
/// so a project that the run refuses stays as it was.
///
/// # Errors
///
/// Returns the errors that [`SetMsrv::run`] describes.
// setmsrv[impl files.unreadable]
// setmsrv[impl pin.unmatched]
// setmsrv[impl lock.update]
// setmsrv[impl lock.absent]
async fn set(root: &Path, args: &SetMsrvArgs) -> Result<(), SetMsrvError> {
    let manifest_path = root.join(MANIFEST);
    let configuration_path = root.join(CONFIGURATION);
    let (mut manifest, manifest_ending) = read(&manifest_path).await?;
    let (mut configuration, configuration_ending) = read(&configuration_path).await?;

    let current = self::manifest::set(&mut manifest, &manifest_path, args.msrv(), args.reason())?;
    if !self::pins::set(&mut configuration, &current, args.msrv()) {
        return Err(SetMsrvError::UnmatchedPin { version: current });
    }

    write(&manifest_path, &manifest, manifest_ending).await?;
    write(&configuration_path, &configuration, configuration_ending).await?;

    // A project that keeps no lock has chosen not to, and mise would create
    // one. A lock whose presence is unknown is locked, because a stale lock
    // makes `mise install --locked` fail.
    if matches!(tokio::fs::try_exists(root.join(LOCK)).await, Ok(false)) {
        return Ok(());
    }

    lock(root).await
}

/// Returns the document in a file of the project and the end of its lines
///
/// # Errors
///
/// Returns [`SetMsrvError::UnreadableFile`] when the file cannot be read,
/// and [`SetMsrvError::MalformedFile`] when it is not valid TOML.
async fn read(path: &Path) -> Result<(DocumentMut, LineEnding), SetMsrvError> {
    let text =
        tokio::fs::read_to_string(path)
            .await
            .map_err(|source| SetMsrvError::UnreadableFile {
                path: path.to_path_buf(),
                source,
            })?;

    let ending = if text.contains("\r\n") {
        LineEnding::CarriageReturnLineFeed
    } else {
        LineEnding::LineFeed
    };
    let document = text.parse().map_err(|source| SetMsrvError::MalformedFile {
        path: path.to_path_buf(),
        source,
    })?;

    Ok((document, ending))
}

/// Writes a document to a file of the project
///
/// The document keeps the comments, the order, and the whitespace of the file
/// that it was read from, so the file changes only where the run changed the
/// document. The lines end as they ended in that file.
///
/// # Errors
///
/// Returns [`SetMsrvError::UnwritableFile`] when the file cannot be written.
// setmsrv[impl files.layout]
async fn write(
    path: &Path,
    document: &DocumentMut,
    ending: LineEnding,
) -> Result<(), SetMsrvError> {
    let text = match ending {
        LineEnding::LineFeed => document.to_string(),
        LineEnding::CarriageReturnLineFeed => document
            .to_string()
            .replace("\r\n", "\n")
            .replace('\n', "\r\n"),
    };

    tokio::fs::write(path, text)
        .await
        .map_err(|source| SetMsrvError::UnwritableFile {
            path: path.to_path_buf(),
            source,
        })
}

/// Has mise lock the Rust pins of the project again
///
/// Mise generates the lock, so it writes the entry of the new pin in its own
/// format and removes the entry of the old one. It locks every Rust pin of
/// the project, so a pin that names a channel or a partial version, such as
/// `nightly` or `1.94`, can move to the newest version that it resolves to.
///
/// # Errors
///
/// Returns [`SetMsrvError::MiseUnavailable`] when mise does not start, and
/// [`SetMsrvError::UnlockedPin`] with what mise reported when it ends
/// without success.
// setmsrv[impl lock.update]
// setmsrv[impl lock.failed]
async fn lock(root: &Path) -> Result<(), SetMsrvError> {
    let execution = Invocation::new(MISE)
        .args(LOCK_RUST)
        .in_directory(root)
        .run()
        .await
        .map_err(|source| SetMsrvError::MiseUnavailable { source })?;

    if execution.status().success() {
        return Ok(());
    }

    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    Err(SetMsrvError::UnlockedPin {
        details: if text.is_empty() {
            NO_DIAGNOSIS.to_owned()
        } else {
            text.to_owned()
        },
    })
}

/// Replaces a version and keeps the whitespace and the comment around it
// setmsrv[impl files.layout]
fn replace(value: &mut Value, msrv: &Msrv) {
    let decor = value.decor().clone();
    *value = Value::from(msrv.get());
    *value.decor_mut() = decor;
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // setmsrv[verify name]
    #[test]
    fn name_is_set_msrv() {
        let command = SetMsrv;

        assert_eq!(command.name().get(), "set-msrv");
    }
}
