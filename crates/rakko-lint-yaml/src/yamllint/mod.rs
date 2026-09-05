//! The yamllint that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether yamllint has anything to do there, and the runs that
//! produce a listing and a report. An action asks for a run, and everything
//! between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The files that yamllint examines in a project
pub mod listing;
/// The reading of the report that yamllint wrote
pub mod report;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveYamllintError;
pub use self::listing::ListFilesError;
use crate::observation::Observation;

/// The name that mise knows the tool by
const YAMLLINT: &str = "yamllint";

/// The option that asks yamllint for its report in the parsable format
///
/// Yamllint groups its findings under a heading per file by default, and it
/// writes one self-contained line per problem in this format, which carries
/// the file, the position, the level, and the description in fields instead
/// of in a block that a reader has to take apart. The format also protects a
/// run from its environment, because the default format changes on a terminal
/// and on a build server. The option selects the presentation of the report
/// and not the behavior of the tool: which rules apply to which file comes
/// from the configuration of the project alone.
const FORMAT: &str = "--format";

/// The name of the format that carries one problem per line
const PARSABLE: &str = "parsable";

/// The place that a run tells yamllint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and yamllint reports every path relative to it.
const HERE: &str = ".";

/// The status of a yamllint that examined every file and found a problem
///
/// Yamllint separates the problems of a project from its own failures. It
/// ends this way when a rule of an error level was broken, and it ends with
/// success when nothing was broken and when the problems were warnings alone.
/// Every other status belongs to a run that stopped before it was done.
const PROBLEMS_FOUND: i32 = 1;

/// The extensions of the files that yamllint collects below a directory
const YAML_EXTENSIONS: [&str; 2] = ["yaml", "yml"];

/// The name of the configuration file that yamllint examines as well
const CONFIGURATION: &str = ".yamllint";

/// The yamllint that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a yamllint that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_lint_yaml::Yamllint;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// if Yamllint::applies(&root).await {
///     let yamllint = Yamllint::resolve(root).await?;
///
///     if !yamllint.list().await?.is_empty() {
///         let observation = yamllint.observe().await?;
///
///         println!("{} problems", observation.problems().len());
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Yamllint {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Yamllint {
    /// Returns whether the project holds a file that yamllint would look at
    ///
    /// The look walks the project from its root and stops at the first file
    /// with the `.yaml` or the `.yml` extension, and at the first file named
    /// `.yamllint`, which are the three that yamllint collects below a
    /// directory by default. It reads an entry whose name starts with a dot,
    /// because yamllint reads one as well, so a project whose only YAML files
    /// sit in a directory such as `.github` applies. It follows no symbolic
    /// link, so a cycle of links cannot trap it.
    ///
    /// A directory that the look cannot read counts as holding YAML files. A
    /// look that cannot prove absence must not hide a real check behind a
    /// skip, and yamllint reports its own failure when a run reaches it.
    ///
    /// The look and yamllint can still disagree at the margins, because the
    /// configuration of a project can name other file patterns and can
    /// exclude every file that the look found. A caller therefore asks
    /// yamllint for its own selection before it lints.
    // lintyaml[impl skip.hidden]
    // lintyaml[impl skip.links]
    // lintyaml[impl skip.missing]
    pub async fn applies(root: &ProjectRoot) -> bool {
        let mut pending = vec![root.get().to_path_buf()];

        while let Some(directory) = pending.pop() {
            let Ok(mut entries) = tokio::fs::read_dir(&directory).await else {
                return true;
            };

            loop {
                match entries.next_entry().await {
                    Ok(Some(entry)) => {
                        let Ok(kind) = entry.file_type().await else {
                            return true;
                        };

                        if kind.is_dir() {
                            pending.push(entry.path());
                        } else if kind.is_file() && yaml(&entry.path()) {
                            return true;
                        }
                    }
                    Ok(None) => break,
                    Err(_) => return true,
                }
            }
        }

        false
    }

    /// Returns the files that yamllint examines in the project
    ///
    /// The listing reads the configuration of the project, so it answers with
    /// the selection that a lint run of the same project examines.
    ///
    /// # Errors
    ///
    /// Returns [`YamllintUnavailable`][unavailable] when yamllint does not
    /// run, and [`RejectedConfiguration`][rejected] when yamllint refuses the
    /// configuration of the project.
    ///
    /// [unavailable]: ListFilesError::YamllintUnavailable
    /// [rejected]: ListFilesError::RejectedConfiguration
    // lintyaml[impl run.listing]
    pub async fn list(&self) -> Result<Vec<PathBuf>, ListFilesError> {
        self::listing::files(&self.tool).await
    }

    /// Runs yamllint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report in the
    /// parsable format. Yamllint writes the report on its standard output
    /// stream, and it uses the standard error stream for the reason of a run
    /// that it could not finish.
    ///
    /// # Errors
    ///
    /// Returns [`YamllintUnavailable`][unavailable] when yamllint does not
    /// run, and [`UnreadableReport`][unreadable] when it wrote a line that
    /// reports no problem.
    ///
    /// [unavailable]: ObserveYamllintError::YamllintUnavailable
    /// [unreadable]: ObserveYamllintError::UnreadableReport
    // lintyaml[impl check.read]
    // lintyaml[impl run.project]
    // lintyaml[impl run.structured]
    pub async fn observe(&self) -> Result<Observation, ObserveYamllintError> {
        let execution = self
            .tool
            .invocation()
            .arg(FORMAT)
            .arg(PARSABLE)
            .arg(HERE)
            .run()
            .await
            .map_err(|source| ObserveYamllintError::YamllintUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy();
        let stderr = execution.stderr().to_string_lossy();

        // lintyaml[impl check.unreadable]
        let problems = self::report::problems(&stdout).map_err(|source| {
            ObserveYamllintError::UnreadableReport {
                report: stdout.to_string(),
                source,
            }
        })?;

        let status = execution.status();

        Ok(Observation::builder()
            .problems(problems)
            // lintyaml[impl check.incomplete]
            .finished(status.success() || status.code() == Some(PROBLEMS_FOUND))
            .stderr(stderr.trim().to_owned())
            .build())
    }

    /// Returns the yamllint that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no yamllint for the
    /// project.
    // lintyaml[impl tool.missing]
    // lintyaml[impl tool.yamllint]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(YAMLLINT), root).await?;

        Ok(Self { tool })
    }
}

/// Returns whether yamllint would collect the file below a directory
///
/// Yamllint matches the name of a file against the patterns that its
/// configuration names, and the patterns that it uses without a configuration
/// are the two YAML extensions and its own configuration file. The comparison
/// of the extension ignores case, which is what yamllint does on a file system
/// that ignores case, and is the wider answer everywhere else. A look that
/// answers for the wider set never hides a check behind a skip.
// lintyaml[impl skip.missing]
fn yaml(path: &Path) -> bool {
    if path.file_name() == Some(OsStr::new(CONFIGURATION)) {
        return true;
    }

    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };

    YAML_EXTENSIONS
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // lintyaml[verify skip.missing]
    #[test]
    fn yaml_of_a_configuration_of_yamllint_reports_a_match() {
        let matched = yaml(&PathBuf::from("sub/.yamllint"));

        assert!(matched);
    }

    // lintyaml[verify skip.missing]
    #[test]
    fn yaml_of_a_file_of_another_extension_reports_no_match() {
        let matched = yaml(&PathBuf::from("sub/notes.txt"));

        assert!(!matched);
    }

    // lintyaml[verify skip.missing]
    #[test]
    fn yaml_of_a_file_without_an_extension_reports_no_match() {
        let matched = yaml(&PathBuf::from("justfile"));

        assert!(!matched);
    }

    // lintyaml[verify skip.missing]
    #[test]
    fn yaml_of_a_long_extension_reports_a_match() {
        let matched = yaml(&PathBuf::from("sub/notes.yaml"));

        assert!(matched);
    }

    // lintyaml[verify skip.missing]
    #[test]
    fn yaml_of_a_short_extension_reports_a_match() {
        let matched = yaml(&PathBuf::from("sub/notes.yml"));

        assert!(matched);
    }
}
