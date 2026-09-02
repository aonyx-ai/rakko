//! The prettier that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether prettier has anything to do there, and the run of one
//! operation. An action states which operation it wants and which files it
//! cares about, and everything between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObservePrettierError;
use crate::filter::Filter;
use crate::observation::Observation;
use crate::operation::Operation;

/// The name that mise knows the tool by
const PRETTIER: &str = "prettier";

/// The flag that asks prettier to name the files that it would rewrite
const LIST_DIFFERENT: &str = "--list-different";

/// The flag that asks prettier to rewrite the files that it can format
const WRITE: &str = "--write";

/// The flag that lets prettier skip a file of a language it does not know
///
/// A pattern selects files by their extension, and prettier refuses a file
/// that it cannot assign to a language. The flag turns that refusal into a
/// skip, which is what keeps a pattern for every extension usable.
const IGNORE_UNKNOWN: &str = "--ignore-unknown";

/// The directory entry that the look does not read
const GIT_DIRECTORY: &str = ".git";

/// The directory entry of installed packages, which prettier excludes
const DEPENDENCY_DIRECTORY: &str = "node_modules";

/// The prettier that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a prettier that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_prettier::{FileExtension, Filter, Operation, Prettier};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let filter = Filter::new([FileExtension::new("md")]);
/// let prettier = Prettier::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
///
/// let observation = prettier.observe(Operation::Report, &filter).await?;
///
/// println!("{} problems", observation.problems().len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Prettier {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Prettier {
    /// Returns whether the project holds a file that the filter selects
    ///
    /// The look walks the project from its root and stops at the first file
    /// that the filter matches. It reads hidden directories, because prettier
    /// reads them, and it reads neither the `.git` entry, which holds no file
    /// of the project, nor a `node_modules` entry, which prettier excludes.
    /// It follows no symbolic link, so a cycle of links cannot trap it.
    ///
    /// A directory that the look cannot read counts as holding files. A look
    /// that cannot prove absence must not hide a real check behind a skip,
    /// and prettier reports its own failure when a run reaches it.
    ///
    /// The look and prettier can still disagree at the margins, because the
    /// ignore files of a project can exclude every file that the look found.
    /// A caller that reaches prettier therefore reports what prettier saw.
    // prettier[impl look.dependencies]
    // prettier[impl look.files]
    // prettier[impl look.git]
    // prettier[impl look.links]
    // prettier[impl look.unreadable]
    pub async fn applies(root: &ProjectRoot, filter: &Filter) -> bool {
        let mut pending = vec![root.get().to_path_buf()];

        while let Some(directory) = pending.pop() {
            let Ok(mut entries) = tokio::fs::read_dir(&directory).await else {
                return true;
            };

            loop {
                match entries.next_entry().await {
                    Ok(Some(entry)) => {
                        let name = entry.file_name();

                        if name == GIT_DIRECTORY || name == DEPENDENCY_DIRECTORY {
                            continue;
                        }

                        let Ok(kind) = entry.file_type().await else {
                            return true;
                        };

                        if kind.is_dir() {
                            pending.push(entry.path());
                        } else if kind.is_file() && filter.matches(&entry.path()) {
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

    /// Runs one operation over the files that the filter selects
    ///
    /// # Errors
    ///
    /// Returns [`PrettierUnavailable`][unavailable] when prettier does not
    /// run.
    ///
    /// [unavailable]: ObservePrettierError::PrettierUnavailable
    // prettier[impl run.operation]
    // prettier[impl select.unknown]
    pub async fn observe(
        &self,
        operation: Operation,
        filter: &Filter,
    ) -> Result<Observation, ObservePrettierError> {
        let flag = match operation {
            Operation::Report => LIST_DIFFERENT,
            Operation::Rewrite => WRITE,
        };

        let execution = self
            .tool
            .invocation()
            .arg(flag)
            .arg(IGNORE_UNKNOWN)
            .arg(filter.pattern())
            .run()
            .await
            .map_err(|source| ObservePrettierError::PrettierUnavailable { source })?;

        Ok(Observation::read(&execution, operation))
    }

    /// Returns the prettier that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no prettier for the
    /// project.
    // prettier[impl tool.missing]
    // prettier[impl tool.resolve]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(PRETTIER), root).await?;

        Ok(Self { tool })
    }
}
