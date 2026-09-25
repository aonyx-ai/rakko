use std::io;
use std::path::PathBuf;

use rakko_action::Name;
use rakko_cli::clawless::event::SendError;
use rakko_tool::RunCommandError;
use thiserror::Error;
use toml_edit::TomlError;

use super::args::Msrv;
use super::step::MiseStep;

/// An error that stops a run of the set-msrv command
///
/// A run reads both files and finds what it changes before it writes, so an
/// error that names a file it read, the manifest, or the pin leaves the
/// project as it was. An error that names a write, the lock, the install, or
/// the check comes after a write, and the files that the run wrote stay.
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

    /// Mise did not start, so the step that the run handed to it did not
    /// happen
    ///
    /// Mise reported nothing, so the cause is the error that kept it from
    /// starting.
    // setmsrv[impl lock.failed]
    // setmsrv[impl install.failed]
    #[error("failed to start `{step}`")]
    MiseUnavailable {
        /// The step that the run handed to mise
        step: MiseStep,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise refused to lock the new pin, so the install and the check did not
    /// run
    // setmsrv[impl lock.failed]
    #[error(
        "`{}` did not lock the new pin, so fix the cause, run it again, and then run `{}` and the {check} action; mise reported: {details}",
        MiseStep::Lock,
        MiseStep::Install { msrv: msrv.clone() }
    )]
    UnlockedPin {
        /// The version that the run set
        msrv: Msrv,

        /// The check that the run did not reach
        check: Name,

        /// What mise reported
        details: String,
    },

    /// Mise refused to install the toolchain at the new version, so the
    /// check did not run
    // setmsrv[impl install.failed]
    #[error(
        "`{}` did not install the new toolchain, so fix the cause, run it again, and then run the {check} action; mise reported: {details}",
        MiseStep::Install { msrv: msrv.clone() }
    )]
    UninstalledToolchain {
        /// The version that the run set
        msrv: Msrv,

        /// The check that the run did not reach
        check: Name,

        /// What mise reported
        details: String,
    },

    /// The check did not confirm the new version
    ///
    /// The check found problems on the toolchain at the new version, stopped,
    /// or skipped. The run reported its outcome before, so the message names
    /// the check instead of repeating what it said.
    // setmsrv[impl check.failed]
    #[error(
        "the {check} action did not pass on Rust {msrv}, so fix what it reported and run it again"
    )]
    FailedCheck {
        /// The check that the run ran
        check: Name,

        /// The version that the run set
        msrv: Msrv,
    },

    /// The outcome of the check did not reach the reader
    // setmsrv[impl check.report]
    #[error("failed to report the outcome of the {action} action")]
    UnreportedOutcome {
        /// The check whose outcome the run could not report
        action: Name,

        /// The cause of the failure
        source: SendError,
    },
}
