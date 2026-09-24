use std::io;
use std::path::PathBuf;

use rakko_tool::RunCommandError;
use thiserror::Error;
use toml_edit::TomlError;

use super::args::Msrv;

/// An error that stops a run of the set-msrv command
///
/// A run reads both files and finds what it changes before it writes, so an
/// error that names a file it read, the manifest, or the pin leaves the
/// project as it was. An error that names a write or the lock comes after a
/// write, and the files that the run wrote stay.
#[derive(Debug, Error)]
pub enum SetMsrvError {
    /// A file of the project could not be read
    // setmsrv[impl files.unreadable]
    #[error("failed to read {}", path.display())]
    UnreadableFile {
        /// The file that the run tried to read
        path: PathBuf,

        /// The cause of the failure
        source: io::Error,
    },

    /// A file of the project is not valid TOML
    // setmsrv[impl files.unreadable]
    #[error("failed to parse {}", path.display())]
    MalformedFile {
        /// The file that the run tried to parse
        path: PathBuf,

        /// The cause of the failure
        source: TomlError,
    },

    /// The root manifest declares no version to change
    // setmsrv[impl manifest.undeclared]
    #[error("{} declares no rust-version", path.display())]
    UndeclaredMsrv {
        /// The root manifest
        path: PathBuf,
    },

    /// The root manifest declares the version in an inline table
    ///
    /// The TOML that Cargo reads allows no comment inside an inline table, so
    /// the reason has no place there.
    // setmsrv[impl manifest.inline]
    #[error(
        "{} declares rust-version in an inline table, which cannot hold the reason",
        path.display()
    )]
    InlineMsrv {
        /// The root manifest
        path: PathBuf,
    },

    /// No Rust pin of mise names the version that the manifest declares
    // setmsrv[impl pin.unmatched]
    #[error("mise.toml pins no Rust toolchain at {version}, the current rust-version")]
    UnmatchedPin {
        /// The version that the root manifest declares
        version: Msrv,
    },

    /// A file of the project could not be written
    #[error("failed to write {}", path.display())]
    UnwritableFile {
        /// The file that the run tried to write
        path: PathBuf,

        /// The cause of the failure
        source: io::Error,
    },

    /// Mise did not start, so the lock still holds the old pin
    ///
    /// Mise reported nothing, so the cause is the error that kept it from
    /// starting.
    // setmsrv[impl lock.failed]
    #[error("failed to start mise to lock the new pin")]
    MiseUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise refused to lock the new pin
    // setmsrv[impl lock.failed]
    #[error("mise did not lock the new pin: {details}")]
    UnlockedPin {
        /// What mise reported
        details: String,
    },
}
