use std::path::PathBuf;

use rakko_cargo::ReadReportError;
use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::report::ReadNextestReportError;

/// An error that leaves a run of nextest without an answer
///
/// Nothing here is a problem of the project. A test that failed and a
/// diagnostic of a build that did not finish travel as findings of the
/// observation. The variants of this error describe a run whose reports
/// cannot be trusted, and the action that asked for the run reports the
/// error instead of answering from them.
#[derive(Debug, Error)]
pub enum ObserveNextestError {
    /// Cargo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run cargo")]
    CargoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Cargo wrote a record that the crate cannot read
    ///
    /// Nextest forwards the JSON of cargo on the same stream, and the shape
    /// of a record belongs to a version of cargo. A line that names a
    /// compiler message or the end of the build in a shape that the crate
    /// does not know leaves the diagnostics of the build untrusted, and an
    /// answer built on them would hide problems behind a green result.
    #[error("failed to read what cargo reported in {}", root.display())]
    UnreadableDiagnostics {
        /// The workspace root that the run worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadReportError,
    },

    /// Nextest wrote a record that the crate cannot read
    ///
    /// The shape of a record belongs to a version of nextest. A line about a
    /// test or a binary of tests in a shape that the crate does not know
    /// leaves the report untrusted, and an answer built on it would hide
    /// failures behind a green result.
    #[error("failed to read what nextest reported in {}", root.display())]
    UnreadableReport {
        /// The workspace root that nextest worked on
        root: PathBuf,

        /// The cause of the failure
        source: ReadNextestReportError,
    },

    /// Nextest wrote a report that the crate does not recognize
    ///
    /// The shape of the report belongs to a version of nextest. A run that
    /// ended without success and reported no failure, no diagnostic, and no
    /// absence of tests wrote something the crate could not read, and an
    /// answer built on such a report would hide failures behind a green
    /// result.
    #[error("nextest wrote a report in {} that the crate does not recognize: {stderr}", root.display())]
    UnrecognizedReport {
        /// The workspace root that nextest worked on
        root: PathBuf,

        /// What nextest wrote to its standard error stream
        stderr: String,
    },
}
