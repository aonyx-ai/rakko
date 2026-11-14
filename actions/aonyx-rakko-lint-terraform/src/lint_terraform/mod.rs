//! The action that lints the Terraform modules of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps TFLint as a subprocess. TFLint finds the modules, reads its own
//! configuration, and inspects them. The action translates the report that
//! TFLint wrote into an outcome.

mod error;

use std::path::Path;

use rakko::{
    Access, Action, Context, Documented, FilePath, Finding, Location, Name, Outcome, ProjectRoot,
    ToolRequirement, action_name,
};

pub use self::error::LintTerraformError;
use crate::issue::{Place, TflintIssue};
use crate::tflint::Tflint;

/// Lints the Terraform modules of a project
///
/// The action wraps [TFLint]. TFLint walks the project from its root and
/// inspects every directory that holds Terraform files. It skips a directory
/// whose name starts with a dot, such as the `.terraform` directory of a
/// module. TFLint reads the `.tflint.hcl` file of the project, so a run agrees
/// with a contributor that runs `tflint --recursive` bare. The TFLint that runs
/// is the version that the action declares, which [mise] installed, whatever
/// version the project pins. The action installs nothing, and that includes
/// the plugins that `.tflint.hcl` enables: `tflint --init` installs them.
///
/// A run only reports, and it takes no argument. Every issue of TFLint becomes
/// a finding over the range that TFLint gave, whatever its severity. This is
/// because TFLint fails a bare run for each of them as well. An issue about a
/// module as a whole becomes a finding about the project. The message holds
/// the severity and the name of the rule.
///
/// A project without Terraform has no module to inspect, and the run passes.
/// A run stops with an error when mise reports no TFLint at the declared
/// version. It also stops when TFLint cannot load a module, and when TFLint
/// writes a report that the action does not recognize.
///
/// A run declares a read of the project. A caller that orders runs by their
/// declarations runs it after the actions that write.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_lint_terraform::LintTerraform;
///
/// let actions = Bundle::default().with(LintTerraform);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [tflint]: https://github.com/terraform-linters/tflint
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct LintTerraform;

impl Action for LintTerraform {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("lint-terraform")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Tflint::requirement()]
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// # Errors
///
/// Returns the error of the step that could not finish. That step is the
/// resolution of the tool, the run of TFLint, or the reading of its report.
async fn drive(context: &Context) -> Result<Outcome, LintTerraformError> {
    let tflint = Tflint::resolve(context)
        .await
        .map_err(|source| LintTerraformError::UnresolvedTool { source })?;

    let issues = tflint.observe().await?;

    if issues.is_empty() {
        return Ok(Outcome::Passed { summary: None });
    }

    Ok(Outcome::Failed {
        findings: issues
            .iter()
            .map(|issue| finding(issue, context.root()))
            .collect::<Result<Vec<Finding>, LintTerraformError>>()?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one issue of TFLint
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the file of the issue.
///
/// [foreign]: LintTerraformError::ForeignPath
fn finding(issue: &TflintIssue, root: &ProjectRoot) -> Result<Finding, LintTerraformError> {
    let location = match issue.place() {
        Place::Module => Location::Project,
        Place::Range { path, span } => Location::Span {
            path: relative(path, root)?,
            span: *span,
        },
    };

    Ok(Finding::builder()
        .message(issue.message())
        .location(location)
        .build())
}

/// Returns the path of a file that TFLint reported, relative to the root
///
/// TFLint writes the path relative to the directory where it started, which
/// is the project root. It can write the path through the real location of
/// that directory, with `..` in it, when the root sits behind a symbolic link.
/// The real location of the file then gives the path within the root.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the file.
///
/// [foreign]: LintTerraformError::ForeignPath
fn relative(path: &Path, root: &ProjectRoot) -> Result<FilePath, LintTerraformError> {
    let joined = root.get().join(path);
    let resolved = std::fs::canonicalize(&joined).unwrap_or(joined);

    FilePath::within()
        .path(&resolved)
        .root(root)
        .call()
        .ok_or_else(|| LintTerraformError::ForeignPath {
            path: path.to_path_buf(),
        })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{Position, Span};
    use rakko_test_utils::path;

    use super::*;
    use crate::issue::Severity;

    /// Returns a context whose root gives no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns an issue about the given place
    fn issue(place: Place) -> TflintIssue {
        TflintIssue::builder()
            .place(place)
            .severity(Severity::Warning)
            .rule("terraform_typed_variables".to_owned())
            .description("`name` variable has no type".to_owned())
            .build()
    }

    /// The root that the issues of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    /// Returns the range that the issues of a test cover
    fn span() -> Span {
        Span::builder()
            .start(Position::builder().line(1).column(1).build())
            .end(Position::builder().line(1).column(18).build())
            .build()
    }

    #[test]
    fn access_is_read() {
        let access = LintTerraform.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn finding_of_a_module_issue_belongs_to_the_project() {
        let finding = finding(&issue(Place::Module), &root()).unwrap();

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let place = Place::Range {
            path: path("/elsewhere/main.tf"),
            span: span(),
        };

        let error = finding(&issue(place), &root()).unwrap_err();

        assert!(matches!(error, LintTerraformError::ForeignPath { .. }));
    }

    #[test]
    fn finding_of_an_issue_covers_the_range_of_tflint() {
        let place = Place::Range {
            path: path("modules/dns/main.tf"),
            span: span(),
        };

        let finding = finding(&issue(place), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Span {
                path: FilePath::try_from(path("modules/dns/main.tf")).unwrap(),
                span: span(),
            }
        );
    }

    #[test]
    fn lint_terraform_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintTerraform>();
    }

    #[test]
    fn tools_declare_the_tflint_of_the_crate() {
        let tools = LintTerraform.tools();

        assert_eq!(tools, [Tflint::requirement()]);
    }
}
