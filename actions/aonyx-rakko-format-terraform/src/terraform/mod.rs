//! The Terraform that a project runs
//!
//! This module holds the runs of the formatter of Terraform, and the program
//! that mise installed at the version that the project pins. An action asks
//! for a run, and this module holds all the code between the action and the
//! process.

mod error;

use std::path::{Path, PathBuf};

use rakko::{Action, Context, ToolOutput};
use rakko_tool::{Execution, Invocation, Launch, ResolveToolchainError, ShownStreams, Tool};

pub(crate) use self::error::ObserveTerraformError;

/// The name that mise knows Terraform by, which is also the name of its
/// program
const TERRAFORM: &str = "terraform";

/// The command of Terraform that formats files
const FMT: &str = "fmt";

/// The option that asks Terraform to rewrite nothing
const CHECK: &str = "-check";

/// The option that asks Terraform to list each file that it would rewrite
const LIST: &str = "-list=true";

/// The option that keeps the sequences of color out of the diagnostics
const NO_COLOR: &str = "-no-color";

/// The option that asks Terraform to walk the directories below the root
///
/// Terraform formats the files of one directory by default. It skips a
/// directory whose name starts with a dot during the walk, such as the
/// `.terraform` directory of a module.
const RECURSIVE: &str = "-recursive";

/// The status of a check that found a file that is not formatted
const UNFORMATTED: i32 = 3;

/// What a run of the formatter of Terraform does
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Operation {
    /// List the files that are not formatted, and change nothing
    Check,

    /// Rewrite the files that are not formatted, and list them
    Format,
}

/// The Terraform that a project runs
///
/// The value holds the program that mise installed at the version that the
/// project pins. Nothing here installs a tool: provisioning is the job of
/// mise, and a Terraform that mise does not report stops the caller.
///
/// Terraform takes no configuration for its formatter. Every project that
/// runs the same version formats in the same way.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Terraform {
    /// The program that mise installed at the version that the project pins
    tool: Tool,

    /// The tool output of the context that resolved Terraform
    ///
    /// Each run sends what Terraform writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Terraform {
    /// Runs the formatter over the project and returns the files it listed
    ///
    /// A run starts in the project root and walks every directory below it.
    /// Terraform writes the path of each file on its standard output stream,
    /// relative to the root. The list joins each path to the root.
    ///
    /// # Errors
    ///
    /// Returns [`TerraformUnavailable`][unavailable] when Terraform does not
    /// run, and [`UnreadableConfiguration`][unreadable] when it cannot parse
    /// a file. Returns [`UnrecognizedReport`][unrecognized] when a check
    /// reported a file that is not formatted and listed none.
    ///
    /// [unavailable]: ObserveTerraformError::TerraformUnavailable
    /// [unreadable]: ObserveTerraformError::UnreadableConfiguration
    /// [unrecognized]: ObserveTerraformError::UnrecognizedReport
    pub(crate) async fn observe(
        &self,
        operation: Operation,
        root: &Path,
    ) -> Result<Vec<PathBuf>, ObserveTerraformError> {
        let execution = invocation(&self.tool, operation)
            .launch_with(&self.output, ShownStreams::Both)
            .await
            .map_err(|source| ObserveTerraformError::TerraformUnavailable { source })?;

        listed(&execution, operation, root)
    }

    /// Returns the Terraform that mise installed at the version that the
    /// project pins
    ///
    /// Terraform is a toolchain of the project, because the configuration of
    /// the project runs on it. So the lookup takes the first pin of Terraform
    /// in the project, and the action requires no range of versions. A release
    /// can lack an option of the formatter. Then the run of Terraform stops,
    /// and the error of the run carries what Terraform wrote about the option.
    /// A run of Terraform starts in the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolchainError`] when mise does not run, or when the
    /// project pins no Terraform. It also returns one when mise reports no
    /// Terraform at the pinned version, for example because nothing installed
    /// it.
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        context: &Context,
        action: &A,
    ) -> Result<Self, ResolveToolchainError> {
        let tool = Tool::resolve_toolchain()
            .action(action)
            .toolchain(TERRAFORM)
            .executable(TERRAFORM)
            .context(context)
            .call()
            .await?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
    }
}

/// Returns the command that runs one operation of the formatter
fn invocation(tool: &Tool, operation: Operation) -> Invocation {
    let invocation = tool.invocation().arg(FMT);
    let invocation = match operation {
        Operation::Check => invocation.arg(CHECK),
        Operation::Format => invocation,
    };

    invocation.arg(LIST).arg(NO_COLOR).arg(RECURSIVE)
}

/// Returns the files that a run of the formatter listed
///
/// Terraform lists one file on each line of its standard output stream. A
/// check ends with success when every file is formatted, and with a status of
/// its own when it listed a file. A rewrite ends with success and lists the
/// files that it rewrote. Any other status means that Terraform stopped before
/// it read the whole project, and its diagnostics say why. Terraform stops at a
/// file that it cannot parse, and a release stops at an option that it does
/// not know.
///
/// # Errors
///
/// Returns [`UnreadableConfiguration`][unreadable] when Terraform could not
/// read a file, and [`UnrecognizedReport`][unrecognized] when a check reported
/// a file that is not formatted and listed none.
///
/// [unreadable]: ObserveTerraformError::UnreadableConfiguration
/// [unrecognized]: ObserveTerraformError::UnrecognizedReport
fn listed(
    execution: &Execution,
    operation: Operation,
    root: &Path,
) -> Result<Vec<PathBuf>, ObserveTerraformError> {
    let stderr = || execution.stderr().to_string_lossy().trim().to_owned();
    let files: Vec<PathBuf> = execution
        .stdout()
        .to_string_lossy()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| root.join(line))
        .collect();

    match (operation, execution.status().code()) {
        (_, Some(0)) => Ok(files),
        (Operation::Check, Some(UNFORMATTED)) if !files.is_empty() => Ok(files),
        (Operation::Check, Some(UNFORMATTED)) => {
            Err(ObserveTerraformError::UnrecognizedReport { stderr: stderr() })
        }
        _ => Err(ObserveTerraformError::UnreadableConfiguration {
            diagnostics: stderr(),
        }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn operation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Operation>();
    }

    #[test]
    fn terraform_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Terraform>();
    }
}
