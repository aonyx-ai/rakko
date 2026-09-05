//! What one run of nextest produced
//!
//! Nextest and cargo write their JSON to the same stream, and this module
//! turns both into data. The reading recognizes the lines that carry an
//! answer and ignores everything else, so a line that a new version adds does
//! not break it. A run whose reports leave no answer at all stops here, so
//! that no caller reads a green result out of a report it could not read.

use getset::{CopyGetters, Getters};
use rakko_action::{Finding, ProjectRoot};
use rakko_cargo::{CargoReport, CargoRoot};
use rakko_tool::Execution;

use crate::nextest::ObserveNextestError;
use crate::report::NextestReport;

/// The exit status of a nextest run that found no test to run
///
/// Nextest documents the status, and it is the one signal for a workspace
/// without tests, which is not a failure of the project.
const NO_TESTS: i32 = 4;

/// What one run of nextest produced
///
/// The value holds the answer of one run at one workspace root: the findings
/// of the run, and how many tests it ran. A test that failed and a diagnostic
/// of a build that did not finish are both problems of the project, and both
/// arrive as findings, because a caller reports them the same way.
///
/// An observation describes the run and judges nothing. A run without a
/// finding says nothing about whether the caller passes, because a caller
/// that tests several workspaces sums the answers of all of them.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The findings of the run: the tests that failed, and the diagnostics
    /// of a build that did not finish
    #[getset(get = "pub")]
    findings: Vec<Finding>,

    /// How many tests ran, which is every test that passed or failed
    #[getset(get_copy = "pub")]
    ran: u64,
}

impl Observation {
    /// Reads the observation of a run from what nextest and cargo wrote
    ///
    /// The findings name their files relative to the project root, and a
    /// file that the project root does not contain leaves its finding at the
    /// level of the project.
    ///
    /// # Errors
    ///
    /// Returns [`UnreadableReport`][report] or
    /// [`UnreadableDiagnostics`][diagnostics] when a stream holds a record
    /// that the crate cannot read, and [`UnrecognizedReport`][unrecognized]
    /// when the run ended without success and reported nothing that the
    /// crate can answer from. An answer built on such a report would hide
    /// failures behind a green result.
    ///
    /// [diagnostics]: ObserveNextestError::UnreadableDiagnostics
    /// [report]: ObserveNextestError::UnreadableReport
    /// [unrecognized]: ObserveNextestError::UnrecognizedReport
    // nextest[impl finding.build]
    // nextest[impl finding.failed]
    // nextest[impl report.ran]
    pub fn read(
        execution: &Execution,
        root: &CargoRoot,
        project: &ProjectRoot,
    ) -> Result<Self, ObserveNextestError> {
        let stdout = execution.stdout().to_string_lossy();
        let nextest = read_report(root, &stdout)?;
        let diagnostics = read_diagnostics(root, &stdout)?;

        // nextest[impl report.unrecognized]
        if !recognized(&nextest, &diagnostics, execution) {
            return Err(ObserveNextestError::UnrecognizedReport {
                root: root.directory().clone(),
                stderr: execution.stderr().to_string_lossy().into_owned(),
            });
        }

        // nextest[impl finding.build]
        let mut findings: Vec<Finding> = diagnostics
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.finding(root, project))
            .collect();

        // nextest[impl finding.failed]
        findings.extend(
            nextest
                .failures()
                .iter()
                .map(|failure| failure.finding(root, project)),
        );

        Ok(Self {
            findings,
            ran: nextest.ran(),
        })
    }
}

/// Reads the diagnostics that cargo reported at a root
///
/// # Errors
///
/// Returns [`UnreadableDiagnostics`][unreadable] when the stream holds a
/// record of cargo that the crate cannot read.
///
/// [unreadable]: ObserveNextestError::UnreadableDiagnostics
// nextest[impl report.unreadable]
fn read_diagnostics(root: &CargoRoot, stdout: &str) -> Result<CargoReport, ObserveNextestError> {
    CargoReport::read(stdout).map_err(|source| ObserveNextestError::UnreadableDiagnostics {
        root: root.directory().clone(),
        source,
    })
}

/// Reads what nextest reported at a root
///
/// # Errors
///
/// Returns [`UnreadableReport`][unreadable] when the stream holds a record
/// of nextest that the crate cannot read.
///
/// [unreadable]: ObserveNextestError::UnreadableReport
// nextest[impl report.unreadable]
fn read_report(root: &CargoRoot, stdout: &str) -> Result<NextestReport, ObserveNextestError> {
    NextestReport::read(stdout).map_err(|source| ObserveNextestError::UnreadableReport {
        root: root.directory().clone(),
        source,
    })
}

/// Returns whether the crate can answer from the reports of a run
///
/// A run that ended with success answered. A run that ended without success
/// answered when it named a test that failed, a diagnostic of the build, or
/// the absence of tests. Everything else is a report that the crate cannot
/// read, and an answer built on it would hide failures behind a green
/// result.
// nextest[impl report.none]
// nextest[impl report.unrecognized]
fn recognized(nextest: &NextestReport, cargo: &CargoReport, execution: &Execution) -> bool {
    execution.status().success()
        || !nextest.failures().is_empty()
        || !cargo.diagnostics().is_empty()
        || execution.status().code() == Some(NO_TESTS)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::{Path, PathBuf};

    use super::*;

    // nextest[verify report.unreadable]
    #[test]
    fn read_diagnostics_in_a_shape_the_crate_does_not_know_names_the_root() {
        let root = CargoRoot::new(PathBuf::from("/home/otter/project"));

        let diagnostics = read_diagnostics(
            &root,
            r#"{"reason":"compiler-message","message":{"level":5}}"#,
        );

        assert!(
            matches!(
                &diagnostics,
                Err(ObserveNextestError::UnreadableDiagnostics { root, .. })
                    if root == Path::new("/home/otter/project")
            ),
            "expected unreadable diagnostics, got {diagnostics:?}"
        );
    }

    // nextest[verify report.unreadable]
    #[test]
    fn read_report_in_a_shape_the_crate_does_not_know_names_the_root() {
        let root = CargoRoot::new(PathBuf::from("/home/otter/project"));

        let report = read_report(&root, r#"{"type":"test","name":"probe::suite$fails"}"#);

        assert!(
            matches!(
                &report,
                Err(ObserveNextestError::UnreadableReport { root, .. })
                    if root == Path::new("/home/otter/project")
            ),
            "expected an unreadable report, got {report:?}"
        );
    }
}
