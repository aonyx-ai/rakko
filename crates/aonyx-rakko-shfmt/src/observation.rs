//! What one operation of shfmt reported

use std::path::{Path, PathBuf};

use getset::Getters;

use crate::diagnostic::ShfmtDiagnostic;
use crate::shfmt::ObserveShfmtError;

/// How a run of shfmt ended
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Exit {
    /// Shfmt ended with success
    Succeeded,

    /// Shfmt ended without success
    Failed,
}

/// What one operation of shfmt reported about a project
///
/// Shfmt answers on two streams. It writes the path of a file on its standard
/// output stream, one path on each line. What the path means depends on the
/// operation:
///
/// - A check lists each file that is not formatted.
/// - A rewrite lists each file that it rewrote.
/// - A search lists each shell script that it found.
///
/// Shfmt writes an error that it found at a position in a file on its
/// standard error stream. A script that it cannot parse is such an error. A
/// rewrite leaves such a file as it is. So the errors of a rewrite are what
/// remains for a person to repair.
///
/// Every path is absolute. Shfmt writes it relative to the project root, where
/// it starts, and the reading joins the two.
///
/// # Examples
///
/// ```no_run
/// use rakko::{Context, ProjectRoot};
/// use aonyx_rakko_shfmt::{Operation, Shfmt};
/// use rakko_test_utils::path;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root(ProjectRoot::new(path("/home/otter/project"))).build();
/// let shfmt = Shfmt::resolve(&context).await?;
///
/// let observation = shfmt.observe(Operation::Find).await?;
///
/// for script in observation.files() {
///     println!("{}", script.display());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Debug, Default, Getters)]
pub struct Observation {
    /// The files that shfmt listed on its standard output stream
    #[getset(get = "pub")]
    files: Vec<PathBuf>,

    /// The errors that shfmt found at a position in a file
    #[getset(get = "pub")]
    diagnostics: Vec<ShfmtDiagnostic>,
}

#[bon::bon]
impl Observation {
    /// Reads what one run of shfmt wrote, and how it ended
    ///
    /// Shfmt ends without success when a check lists a file, and when it finds
    /// an error in a file. So the exit status alone does not say whether the
    /// run failed. A run that ended without success and reported neither a
    /// file nor an error wrote a report that the reading does not know.
    ///
    /// # Errors
    ///
    /// Returns [`UnrecognizedReport`][unrecognized] when the standard error
    /// stream holds a line that is not an error at a position. A file that
    /// shfmt cannot open gives such a line. It returns the same error for a
    /// run that ended without success and reported nothing.
    ///
    /// [unrecognized]: ObserveShfmtError::UnrecognizedReport
    #[builder]
    pub(crate) fn read(
        exit: Exit,
        stdout: &str,
        stderr: &str,
        root: &Path,
    ) -> Result<Self, ObserveShfmtError> {
        let unrecognized = || ObserveShfmtError::UnrecognizedReport {
            stderr: stderr.to_owned(),
        };

        let files: Vec<PathBuf> = lines(stdout).map(|line| root.join(line)).collect();
        let diagnostics = lines(stderr)
            .map(|line| ShfmtDiagnostic::parse().line(line).root(root).call())
            .collect::<Option<Vec<ShfmtDiagnostic>>>()
            .ok_or_else(unrecognized)?;

        if exit == Exit::Failed && files.is_empty() && diagnostics.is_empty() {
            return Err(unrecognized());
        }

        Ok(Self { files, diagnostics })
    }
}

/// Returns the lines of a stream that hold text
///
/// A line can end with a carriage return on Windows, which is no part of the
/// path or of the message.
fn lines(stream: &str) -> impl Iterator<Item = &str> {
    stream
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko_test_utils::path;

    use super::*;

    /// Returns the root of the project that the tests place their files in
    fn project() -> PathBuf {
        path("/home/otter/project")
    }

    #[test]
    fn exit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Exit>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }

    #[test]
    fn read_of_a_clean_run_reports_nothing() {
        let observation = Observation::read()
            .exit(Exit::Succeeded)
            .stdout("")
            .stderr("")
            .root(&project())
            .call()
            .unwrap();

        assert_eq!(observation, Observation::default());
    }

    #[test]
    fn read_of_a_failed_run_without_a_report_is_unrecognized() {
        let error = Observation::read()
            .exit(Exit::Failed)
            .stdout("")
            .stderr("")
            .root(&project())
            .call()
            .unwrap_err();

        assert!(matches!(
            error,
            ObserveShfmtError::UnrecognizedReport { .. }
        ));
    }

    #[test]
    fn read_of_a_line_without_a_position_is_unrecognized() {
        let error = Observation::read()
            .exit(Exit::Failed)
            .stdout("")
            .stderr("open locked.sh: permission denied\n")
            .root(&project())
            .call()
            .unwrap_err();

        assert!(matches!(
            error,
            ObserveShfmtError::UnrecognizedReport { stderr }
                if stderr == "open locked.sh: permission denied\n"
        ));
    }

    #[test]
    fn read_of_an_error_at_a_position_holds_the_diagnostic() {
        let observation = Observation::read()
            .exit(Exit::Failed)
            .stdout("")
            .stderr("run.sh:1:1: `if` must be followed by a statement list\n")
            .root(&project())
            .call()
            .unwrap();

        assert_eq!(
            observation.diagnostics(),
            &[ShfmtDiagnostic::builder()
                .path(project().join("run.sh"))
                .line(1)
                .column(1)
                .message("`if` must be followed by a statement list".to_owned())
                .build()]
        );
    }

    #[test]
    fn read_of_listed_files_joins_them_to_the_root() {
        let observation = Observation::read()
            .exit(Exit::Failed)
            .stdout("a.sh\r\nbin/tool\n")
            .stderr("")
            .root(&project())
            .call()
            .unwrap();

        assert_eq!(
            observation.files(),
            &[project().join("a.sh"), project().join("bin/tool")]
        );
    }
}
