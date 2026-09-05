//! The files that yamllint examines in a project
//!
//! Yamllint collects the files of a run from the place that the run names and
//! from the configuration of the project, and it can report that selection
//! without linting anything. This module asks for it.
//!
//! The selection answers what the report of a lint run leaves open. A yamllint
//! that collected no file writes the same empty report as a yamllint that
//! examined the whole project and found nothing, and the two runs mean
//! opposite things for the outcome of an action.

/// The error that leaves a caller without the files of a project
mod error;

use std::path::PathBuf;

use rakko_tool::Tool;

pub use self::error::ListFilesError;

/// The flag that asks yamllint which files it examines
const LIST_FILES: &str = "--list-files";

/// The place that a run tells yamllint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and yamllint reports every path relative to it.
const HERE: &str = ".";

/// Returns the files that yamllint examines in the project of the tool
///
/// The listing names the root of the project, so it collects what a lint run
/// of the same project collects. Yamllint reads the configuration of the
/// project first, and the file patterns and the exclusions of that
/// configuration therefore answer here as well.
///
/// # Errors
///
/// Returns [`YamllintUnavailable`][unavailable] when yamllint does not run,
/// and [`RejectedConfiguration`][rejected] when yamllint refuses the
/// configuration of the project.
///
/// [unavailable]: ListFilesError::YamllintUnavailable
/// [rejected]: ListFilesError::RejectedConfiguration
// lintyaml[impl run.listing]
// lintyaml[impl run.project]
pub async fn files(tool: &Tool) -> Result<Vec<PathBuf>, ListFilesError> {
    let execution = tool
        .invocation()
        .arg(LIST_FILES)
        .arg(HERE)
        .run()
        .await
        .map_err(|source| ListFilesError::YamllintUnavailable { source })?;

    // lintyaml[impl check.configuration]
    if !execution.status().success() {
        return Err(ListFilesError::RejectedConfiguration {
            details: execution.stderr().to_string_lossy().trim().to_owned(),
        });
    }

    Ok(paths(&execution.stdout().to_string_lossy()))
}

/// Returns the paths that a listing of yamllint holds
///
/// Yamllint writes one path per line, and a project without a file that it
/// examines answers with nothing at all.
fn paths(listing: &str) -> Vec<PathBuf> {
    listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // lintyaml[verify skip.unexamined]
    #[test]
    fn paths_of_an_empty_listing_hold_nothing() {
        let paths = paths("\n");

        assert!(paths.is_empty(), "expected no path, got {paths:?}");
    }

    // lintyaml[verify check.passed]
    #[test]
    fn paths_of_a_listing_hold_one_path_per_line() {
        let paths = paths("./a.yaml\n./sub/b.yml\n");

        assert_eq!(
            paths,
            [PathBuf::from("./a.yaml"), PathBuf::from("./sub/b.yml")]
        );
    }
}
