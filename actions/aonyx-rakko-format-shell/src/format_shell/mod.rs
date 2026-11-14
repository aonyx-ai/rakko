//! The action that formats the shell scripts of a project
//!
//! This module holds the action, the arguments that a run reads, and the
//! error that stops a run. The action wraps shfmt as a subprocess. Shfmt
//! finds the scripts, reads its own configuration, and does the formatting.
//! The action translates what shfmt reported into an outcome.

mod args;
mod error;
mod format_mode;

use std::path::Path;

use aonyx_rakko_shfmt::{Observation, Operation, Shfmt, ShfmtDiagnostic};
use rakko::{
    Access, Action, Context, Documented, FilePath, Finding, Location, Name, Outcome, Position,
    ProjectRoot, SkipReason, Summary, ToolRequirement, action_name,
};

pub use self::args::FormatShellArgs;
pub use self::error::FormatShellError;
pub use self::format_mode::FormatMode;

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not properly formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not properly formatted";

/// The reason of a run that found no shell script
const NO_SCRIPT: &str = "shfmt found no shell script in the project";

/// Checks the format of the shell scripts of a project, or repairs it
///
/// The action wraps [shfmt]. Shfmt finds the shell scripts of the project. A
/// script has a shell extension, such as `.sh` or `.bash`, or a shebang that
/// starts a shell. Shfmt reads its options from the `.editorconfig` files
/// of the project, and it leaves out a file that such a file marks with
/// `ignore = true`. So a run agrees with an editor and with a contributor that
/// runs shfmt bare. The shfmt that runs is the version that the action
/// declares, which [mise] installed, whatever version the project pins. The
/// action installs nothing.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file. A script that shfmt cannot parse becomes a finding at
/// the position that shfmt reports. With the `fix` argument, shfmt rewrites
/// the files that it can format. The outcome carries one repair for each file
/// that it rewrote, next to the scripts that it could not parse.
///
/// A run that reports declares a read of the project, and a run with `fix`
/// declares a write. A caller that orders runs by their declarations can
/// then run a rewrite before the actions that read.
///
/// A project that holds no shell script is skipped, because shfmt found
/// nothing to format. A run stops with an error when mise reports no shfmt at
/// the declared version. It also stops when shfmt writes a report that the
/// action does not recognize, such as a file that shfmt cannot open.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_format_shell::FormatShell;
///
/// let actions = Bundle::default().with(FormatShell);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [shfmt]: https://github.com/mvdan/sh
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct FormatShell;

impl Action for FormatShell {
    type Args = FormatShellArgs;

    fn name(&self) -> Name {
        action_name!("format-shell")
    }

    fn access(&self, _context: &Context, args: &Self::Args) -> Access {
        match args.format_mode() {
            FormatMode::Rewrite => Access::Write,
            FormatMode::Report => Access::Read,
        }
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Shfmt::requirement()]
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
/// The run resolves shfmt, finds the scripts, checks them, and fixes them
/// when the arguments ask for it. An error that this function returns stops
/// the run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, a shfmt run, or the reading of the report.
async fn drive(context: &Context, args: &FormatShellArgs) -> Result<Outcome, FormatShellError> {
    let shfmt = Shfmt::resolve(context)
        .await
        .map_err(|source| FormatShellError::UnresolvedTool { source })?;

    let found = shfmt.observe(Operation::Find).await?;

    if found.files().is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_SCRIPT),
        });
    }

    let checked = shfmt.observe(Operation::CheckFormat).await?;

    if checked.files().is_empty() && checked.diagnostics().is_empty() {
        return Ok(Outcome::Passed {
            summary: Some(summary(found.files().len())),
        });
    }

    match args.format_mode() {
        FormatMode::Rewrite => fix(&shfmt, context.root()).await,
        FormatMode::Report => Ok(Outcome::Failed {
            findings: findings(&checked, context.root())?,
            repairs: Vec::new(),
        }),
    }
}

/// Returns the finding at the position of an error that shfmt found
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the error.
///
/// [foreign]: FormatShellError::ForeignPath
fn diagnostic(
    diagnostic: &ShfmtDiagnostic,
    root: &ProjectRoot,
) -> Result<Finding, FormatShellError> {
    let path = diagnostic
        .relative_path(root)
        .ok_or_else(|| FormatShellError::ForeignPath {
            path: diagnostic.path().clone(),
        })?;

    Ok(Finding::builder()
        .message(diagnostic.message().clone())
        .location(Location::Position {
            path,
            position: Position::builder()
                .line(diagnostic.line())
                .column(diagnostic.column())
                .build(),
        })
        .build())
}

/// Returns the finding at the level of a file that shfmt listed
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
/// [foreign]: FormatShellError::ForeignPath
fn file(path: &Path, root: &ProjectRoot, message: &str) -> Result<Finding, FormatShellError> {
    let relative = FilePath::within()
        .path(path)
        .root(root)
        .call()
        .ok_or_else(|| FormatShellError::ForeignPath {
            path: path.to_path_buf(),
        })?;

    Ok(Finding::builder()
        .message(message)
        .location(Location::File { path: relative })
        .build())
}

/// Returns the findings of a check: each file that is not formatted, and
/// each error that shfmt found in a script
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of a file or of an error.
///
/// [foreign]: FormatShellError::ForeignPath
fn findings(
    observation: &Observation,
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatShellError> {
    let unformatted = observation
        .files()
        .iter()
        .map(|path| file(path, root, UNFORMATTED_FINDING));
    let invalid = observation
        .diagnostics()
        .iter()
        .map(|error| diagnostic(error, root));

    unformatted.chain(invalid).collect()
}

/// Rewrites the files that shfmt can format and reports what remains
///
/// Shfmt lists each file that a rewrite changed, and each of them becomes a
/// repair. A script that shfmt cannot parse stays as it is, because a rewrite
/// repairs formatting and a syntax error needs a hand. Each of its errors
/// remains as a finding.
///
/// # Errors
///
/// Returns the error of a rewrite that could not finish. Shfmt did not run,
/// or it wrote a report that the action does not recognize, or it reported a
/// path outside the project.
async fn fix(shfmt: &Shfmt, root: &ProjectRoot) -> Result<Outcome, FormatShellError> {
    let rewritten = shfmt.observe(Operation::Format).await?;

    let repairs = rewritten
        .files()
        .iter()
        .map(|path| file(path, root, UNFORMATTED_REPAIR))
        .collect::<Result<Vec<Finding>, FormatShellError>>()?;
    let findings = rewritten
        .diagnostics()
        .iter()
        .map(|error| diagnostic(error, root))
        .collect::<Result<Vec<Finding>, FormatShellError>>()?;

    if findings.is_empty() {
        Ok(Outcome::Changed { repairs })
    } else {
        Ok(Outcome::Failed { findings, repairs })
    }
}

/// Returns the summary that tells how many scripts shfmt checked
///
/// A pass says how many scripts shfmt found, so that a reader can question a
/// pass that examined fewer scripts than the project holds.
fn summary(checked: usize) -> Summary {
    if checked == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {checked} files"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{Args, ArgsValues, ArgumentValue, argument_name};

    use super::*;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns the arguments of a run that asks for a repair
    fn fixing() -> FormatShellArgs {
        let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);

        FormatShellArgs::from_values(&values).unwrap()
    }

    #[test]
    fn access_with_fix_is_write() {
        let args = fixing();

        let access = FormatShell.access(&context(), &args);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn access_without_fix_is_read() {
        let args = FormatShellArgs::default();

        let access = FormatShell.access(&context(), &args);

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn format_shell_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatShell>();
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
    fn tools_declare_the_shfmt_of_the_crate() {
        let tools = FormatShell.tools();

        assert_eq!(tools, [Shfmt::requirement()]);
    }
}
