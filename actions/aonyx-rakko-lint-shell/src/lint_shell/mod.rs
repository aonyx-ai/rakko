//! The action that lints the shell scripts of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps two tools as subprocesses. Shfmt finds the scripts, and ShellCheck
//! reads its own configuration and examines them. The action translates what
//! ShellCheck reported into an outcome.

mod error;

use std::path::Path;

use aonyx_rakko_shfmt::{Operation, Shfmt};
use rakko::{
    Access, Action, Context, Documented, FilePath, Finding, Location, Name, Outcome, ProjectRoot,
    SkipReason, Summary, ToolRequirement, action_name,
};

pub use self::error::LintShellError;
use crate::comment::ShellcheckComment;
use crate::shellcheck::Shellcheck;

/// The reason of a run that found no shell script
const NO_SCRIPT: &str = "shfmt found no shell script in the project";

/// Lints the shell scripts of a project
///
/// The action wraps two tools. [Shfmt] finds the shell scripts of the project.
/// A script has a shell extension, such as `.sh` or `.bash`, or a shebang that
/// starts a shell. Shfmt leaves out a file that an `.editorconfig` file marks
/// with `ignore = true`. So the action lints the scripts that `format-shell`
/// formats. [ShellCheck] then examines those scripts. It reads the
/// `.shellcheckrc` files of the project, so a run agrees with a contributor
/// that runs ShellCheck bare on the same scripts. The versions that run are
/// the versions that the action declares, which [mise] installed, whatever
/// versions the project pins. The action installs nothing.
///
/// A run only reports, and it takes no argument. Every comment of ShellCheck
/// becomes a finding over the range that ShellCheck gave. The message of the
/// finding is the sentence that ShellCheck wrote, with the level and the code
/// of the check. A comment at the style level becomes a finding like one at
/// the error level. This is because a project that wants a check to stay
/// quiet turns that check off with its code.
///
/// A project that holds no shell script is skipped, because shfmt found
/// nothing to lint. A run stops with an error when mise reports no shfmt or no
/// ShellCheck at the declared version. It also stops when a tool cannot open
/// a script or a directory, and when a tool writes a report that the action
/// cannot read.
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
/// use aonyx_rakko_lint_shell::LintShell;
///
/// let actions = Bundle::default().with(LintShell);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [shellcheck]: https://www.shellcheck.net
/// [shfmt]: https://github.com/mvdan/sh
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct LintShell;

impl Action for LintShell {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("lint-shell")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Shfmt::requirement(), Shellcheck::requirement()]
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
/// The run resolves both tools, finds the scripts with shfmt, and examines
/// them with ShellCheck. An error that this function returns stops the run,
/// and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of a
/// tool, the search, the examination, or the reading of a report.
async fn drive(context: &Context) -> Result<Outcome, LintShellError> {
    let shfmt = Shfmt::resolve(context)
        .await
        .map_err(|source| LintShellError::UnresolvedShfmt { source })?;
    let shellcheck = Shellcheck::resolve(context)
        .await
        .map_err(|source| LintShellError::UnresolvedShellcheck { source })?;

    let found = shfmt.observe(Operation::Find).await?;

    if found.files().is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_SCRIPT),
        });
    }

    let scripts = found
        .files()
        .iter()
        .map(|path| relative(path, context.root()))
        .collect::<Result<Vec<FilePath>, LintShellError>>()?;

    let comments = shellcheck.observe(&scripts).await?;

    if comments.is_empty() {
        return Ok(Outcome::Passed {
            summary: Some(summary(scripts.len())),
        });
    }

    Ok(Outcome::Failed {
        findings: comments
            .iter()
            .map(|comment| finding(comment, context.root()))
            .collect::<Result<Vec<Finding>, LintShellError>>()?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one comment of ShellCheck
///
/// The finding covers the range that ShellCheck gave. The message comes from
/// ShellCheck, with the level and the code of the check.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the comment.
///
/// [foreign]: LintShellError::ForeignPath
fn finding(comment: &ShellcheckComment, root: &ProjectRoot) -> Result<Finding, LintShellError> {
    let path = comment
        .relative_path(root)
        .ok_or_else(|| LintShellError::ForeignPath {
            path: comment.path().clone(),
        })?;

    Ok(Finding::builder()
        .message(comment.message())
        .location(Location::Span {
            path,
            span: comment.span(),
        })
        .build())
}

/// Returns the path of a script that shfmt found, relative to the root
///
/// A run starts ShellCheck in the root and gives it each script in this form.
/// ShellCheck then writes the same form in its report, and its own output
/// stays short for a reader.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the script.
///
/// [foreign]: LintShellError::ForeignPath
fn relative(path: &Path, root: &ProjectRoot) -> Result<FilePath, LintShellError> {
    FilePath::within()
        .path(path)
        .root(root)
        .call()
        .ok_or_else(|| LintShellError::ForeignPath {
            path: path.to_path_buf(),
        })
}

/// Returns the summary that tells how many scripts ShellCheck examined
///
/// A pass says how many scripts shfmt found, so that a reader can question a
/// pass that examined fewer scripts than the project holds.
fn summary(examined: usize) -> Summary {
    if examined == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {examined} files"))
    }
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
    use crate::comment::CommentLevel;

    /// Returns a context whose root gives no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns a comment about the given path
    fn comment(path: &str) -> ShellcheckComment {
        ShellcheckComment::builder()
            .path(path.into())
            .span(span())
            .level(CommentLevel::Warning)
            .code(2010)
            .description("Don't use ls | grep.".to_owned())
            .build()
    }

    /// The root that the comments of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    /// Returns the range that the comments of a test cover
    fn span() -> Span {
        Span::builder()
            .start(Position::builder().line(3).column(1).build())
            .end(Position::builder().line(3).column(3).build())
            .build()
    }

    #[test]
    fn access_is_read() {
        let access = LintShell.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn finding_of_a_comment_carries_the_message_of_shellcheck() {
        let finding = finding(&comment("run.sh"), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[warning] Don't use ls | grep. (SC2010)"
        );
    }

    #[test]
    fn finding_of_a_comment_covers_the_range_of_shellcheck() {
        let finding = finding(&comment("run.sh"), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Span {
                path: "run.sh".parse().unwrap(),
                span: span(),
            }
        );
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let error = finding(&comment("/elsewhere/run.sh"), &root()).unwrap_err();

        assert!(matches!(error, LintShellError::ForeignPath { .. }));
    }

    #[test]
    fn lint_shell_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<LintShell>();
    }

    #[test]
    fn relative_of_a_script_in_the_root_drops_the_root() {
        let script = relative(&path("/home/otter/project/bin/run.sh"), &root()).unwrap();

        assert_eq!(script, FilePath::try_from(path("bin/run.sh")).unwrap());
    }

    #[test]
    fn relative_of_a_script_outside_the_root_reports_the_path() {
        let error = relative(&path("/elsewhere/run.sh"), &root()).unwrap_err();

        assert!(matches!(error, LintShellError::ForeignPath { .. }));
    }

    #[test]
    fn summary_of_one_file_uses_the_singular() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 file");
    }

    #[test]
    fn summary_of_several_files_uses_the_plural() {
        let summary = summary(3);

        assert_eq!(summary.get(), "checked 3 files");
    }

    #[test]
    fn tools_declare_shfmt_and_shellcheck() {
        let tools = LintShell.tools();

        assert_eq!(tools, [Shfmt::requirement(), Shellcheck::requirement()]);
    }
}
