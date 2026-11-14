//! The action that formats the Terraform files of a project
//!
//! This module holds the action, the arguments that a run reads, and the
//! error that stops a run. The action wraps `terraform fmt` as a subprocess.
//! Terraform finds the files and formats them, and the action translates the
//! list of files that Terraform reported into an outcome.

mod args;
mod error;
mod format_mode;

use std::path::{Path, PathBuf};

use rakko::{
    Access, Action, Context, Documented, FilePath, Finding, Location, Name, Outcome, ProjectRoot,
    ToolRequirement, action_name,
};

pub use self::args::FormatTerraformArgs;
pub use self::error::FormatTerraformError;
pub use self::format_mode::FormatMode;
use crate::terraform::{Operation, Terraform};

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not properly formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not properly formatted";

/// Checks the format of the Terraform files of a project, or repairs it
///
/// The action wraps `terraform fmt`. Terraform walks the project from its
/// root and formats every Terraform file, such as a `.tf` or a `.tfvars` file.
/// It skips a directory whose name starts with a dot, such as the
/// `.terraform` directory of a module. Terraform takes no configuration for its formatter, so every
/// project that runs the same version formats in the same way.
///
/// Terraform is a toolchain of the project. So the version that runs is the
/// version that the project pins in its mise configuration, which [mise]
/// installed. A run stops when that version is older than 1.0. The action
/// installs nothing.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that gives the file. With the `fix` argument, Terraform rewrites those
/// files. The outcome then carries one repair for each file that it rewrote.
///
/// A run that reports declares a read of the project, and a run with `fix`
/// declares a write. A caller that orders runs by their declarations can
/// then run a rewrite before the actions that read.
///
/// Terraform finds no file in a project without Terraform, and the run passes.
/// A run stops with an error when mise reports no Terraform that the action
/// accepts. It also stops when Terraform cannot parse a file of the project,
/// because Terraform stops at that file and the rest stays unread.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_format_terraform::FormatTerraform;
///
/// let actions = Bundle::default().with(FormatTerraform);
/// ```
///
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct FormatTerraform;

impl Action for FormatTerraform {
    type Args = FormatTerraformArgs;

    fn name(&self) -> Name {
        action_name!("format-terraform")
    }

    fn access(&self, _context: &Context, args: &Self::Args) -> Access {
        match args.format_mode() {
            FormatMode::Rewrite => Access::Write,
            FormatMode::Report => Access::Read,
        }
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Terraform::requirement()]
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves Terraform, checks the files, and rewrites them when the
/// arguments ask for it. An error that this function returns stops the run,
/// and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish. That step is the
/// resolution of the tool, a run of Terraform, or the reading of its report.
async fn drive(
    context: &Context,
    args: &FormatTerraformArgs,
) -> Result<Outcome, FormatTerraformError> {
    let terraform = Terraform::resolve(context)
        .await
        .map_err(|source| FormatTerraformError::UnresolvedTool { source })?;
    let root = context.root();

    let unformatted = terraform.observe(Operation::Check, root.get()).await?;

    if unformatted.is_empty() {
        return Ok(Outcome::Passed { summary: None });
    }

    match args.format_mode() {
        FormatMode::Report => Ok(Outcome::Failed {
            findings: files(&unformatted, root, UNFORMATTED_FINDING)?,
            repairs: Vec::new(),
        }),
        FormatMode::Rewrite => {
            let rewritten = terraform.observe(Operation::Format, root.get()).await?;

            Ok(Outcome::Changed {
                repairs: files(&rewritten, root, UNFORMATTED_REPAIR)?,
            })
        }
    }
}

/// Returns the finding at the level of a file that Terraform listed
///
/// The message comes from the caller. This is because a finding states a
/// problem that the project has, and a repair states one that the run took
/// away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the file.
///
/// [foreign]: FormatTerraformError::ForeignPath
fn file(path: &Path, root: &ProjectRoot, message: &str) -> Result<Finding, FormatTerraformError> {
    let relative = FilePath::within()
        .path(path)
        .root(root)
        .call()
        .ok_or_else(|| FormatTerraformError::ForeignPath {
            path: path.to_path_buf(),
        })?;

    Ok(Finding::builder()
        .message(message)
        .location(Location::File { path: relative })
        .build())
}

/// Returns one finding with the given message for each listed file
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain a
/// listed file.
///
/// [foreign]: FormatTerraformError::ForeignPath
fn files(
    paths: &[PathBuf],
    root: &ProjectRoot,
    message: &str,
) -> Result<Vec<Finding>, FormatTerraformError> {
    paths.iter().map(|path| file(path, root, message)).collect()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{Args, ArgsValues, ArgumentValue, argument_name};
    use rakko_test_utils::path;

    use super::*;

    /// Returns a context whose root gives no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns the arguments of a run that asks for a repair
    fn fixing() -> FormatTerraformArgs {
        let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);

        FormatTerraformArgs::from_values(&values).unwrap()
    }

    /// The root that the files of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn access_with_fix_is_write() {
        let args = fixing();

        let access = FormatTerraform.access(&context(), &args);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn access_without_fix_is_read() {
        let args = FormatTerraformArgs::default();

        let access = FormatTerraform.access(&context(), &args);

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn file_in_the_project_drops_the_root() {
        let finding = file(
            &path("/home/otter/project/modules/dns/main.tf"),
            &root(),
            UNFORMATTED_FINDING,
        )
        .unwrap();

        assert_eq!(
            finding.location(),
            &Location::File {
                path: FilePath::try_from(path("modules/dns/main.tf")).unwrap(),
            }
        );
    }

    #[test]
    fn file_outside_the_project_reports_the_path() {
        let error = file(&path("/elsewhere/main.tf"), &root(), UNFORMATTED_FINDING).unwrap_err();

        assert!(matches!(error, FormatTerraformError::ForeignPath { .. }));
    }

    #[test]
    fn format_terraform_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatTerraform>();
    }

    #[test]
    fn tools_declare_terraform_as_a_toolchain() {
        let tools = FormatTerraform.tools();

        assert_eq!(tools, [Terraform::requirement()]);
    }
}
