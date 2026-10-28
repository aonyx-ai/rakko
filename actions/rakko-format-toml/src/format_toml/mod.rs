//! The action that formats the TOML files of a project
//!
//! This module holds the action, the arguments that a run reads, and the
//! error that stops a run. The action wraps taplo as a subprocess: taplo
//! discovers the files, reads its own configuration, and does the
//! formatting, and the action translates what taplo reported into an
//! outcome.

/// The arguments that a run of the action reads
mod args;
/// The error that stops a run of the action
mod error;

use std::collections::HashSet;
use std::path::PathBuf;

use rakko_action::{
    Access, Action, Context, FileAccess, Finding, Location, Name, Outcome, Position, ProjectRoot,
    Summary, action_name,
};
use rakko_taplo::{Observation, Operation, ProblemDetail, Taplo, TaploProblem};

pub use self::args::FormatTomlArgs;
pub use self::error::FormatTomlError;

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not properly formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not properly formatted";

/// The action that formats the TOML files of a project
///
/// The action wraps [taplo]: taplo discovers the TOML files of the project,
/// reads its own configuration, and formats what its configuration selects,
/// so a run agrees with an editor and with a contributor that runs taplo
/// bare. The taplo that runs is the one that [mise] installed for the
/// project, at the version that the project pinned, and the action installs
/// nothing.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file, and a file that taplo cannot parse becomes a finding
/// at the position that taplo reports. With the `fix` argument, taplo
/// rewrites the files that it can format, and the outcome carries one repair
/// for each file that it rewrote, next to the problems that remain.
///
/// A run that reports declares a read, and a run with `fix` declares a write.
/// Each run reads the configuration of taplo and the files that mise reads to
/// find taplo. A run that reports reads the TOML files as well, and a run with
/// `fix` writes them. A caller that orders runs by their declarations can then
/// run a rewrite before the actions that read, and beside the runs that touch
/// none of these files.
///
/// A project that holds no TOML file passes, having formatted nothing. A run stops
/// with an error when mise reports no taplo,
/// when taplo rejects a configuration file of the project, and when taplo
/// writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_format_toml::FormatToml;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatToml)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [taplo]: https://taplo.tamasfe.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct FormatToml;

impl Action for FormatToml {
    type Args = FormatTomlArgs;

    fn name(&self) -> Name {
        action_name!("format-toml")
    }

    fn access(&self, _context: &Context, args: &Self::Args) -> Access {
        if args.fix() {
            Access::Write
        } else {
            Access::Read
        }
    }

    fn files(&self, context: &Context, args: &Self::Args) -> FileAccess {
        Taplo::files(self.access(context, args))
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
/// The run examines the project, resolves taplo, checks, and fixes when the
/// arguments ask for it. An error that this function returns stops the run,
/// and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, a taplo run, or the reading of the report.
async fn drive(context: &Context, args: &FormatTomlArgs) -> Result<Outcome, FormatTomlError> {
    let taplo = Taplo::resolve(context.root().clone())
        .await
        .map_err(|source| FormatTomlError::UnresolvedTool { source })?;

    let observation = taplo.observe(Operation::CheckFormat).await?;

    rejected(&observation)?;

    if observation.problems().is_empty() {
        return Ok(passed(&observation));
    }

    if args.fix() {
        fix(&taplo, observation.problems(), context.root()).await
    } else {
        Ok(Outcome::Failed {
            findings: findings(observation.problems(), context.root())?,
            repairs: Vec::new(),
        })
    }
}

/// Returns the finding that reports one problem of the project
///
/// A file that is not formatted gets a finding at the level of the file, and
/// a diagnostic gets one at the position that taplo named. The message for
/// an unformatted file comes from the caller, because a finding states a
/// problem that the project has and a repair states one that the run took
/// away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the problem.
///
/// [foreign]: FormatTomlError::ForeignPath
fn finding(
    problem: &TaploProblem,
    root: &ProjectRoot,
    unformatted_message: &str,
) -> Result<Finding, FormatTomlError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| FormatTomlError::ForeignPath {
            path: problem.path().clone(),
        })?;

    let finding = match problem.detail() {
        ProblemDetail::Diagnostic {
            line,
            column,
            message,
        } => Finding::builder()
            .message(message.clone())
            .location(Location::Position {
                path,
                position: Position::builder().line(*line).column(*column).build(),
            })
            .build(),
        ProblemDetail::Invalid { reason } => Finding::builder()
            .message(reason.clone())
            .location(Location::File { path })
            .build(),
        ProblemDetail::Unformatted => Finding::builder()
            .message(unformatted_message)
            .location(Location::File { path })
            .build(),
    };

    Ok(finding)
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of a problem.
///
/// [foreign]: FormatTomlError::ForeignPath
fn findings(
    problems: &[TaploProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatTomlError> {
    problems
        .iter()
        .map(|problem| finding(problem, root, UNFORMATTED_FINDING))
        .collect()
}

/// Repairs the problems that the check found and reports what remains
///
/// Taplo does not report which files a rewrite touched, so the repairs
/// derive from the two reports: a problem of the check whose file the
/// rewrite does not report again was repaired, and what the rewrite still
/// reports remains. The files that taplo cannot parse remain this way,
/// because a rewrite repairs formatting and a syntax error needs a hand.
///
/// # Errors
///
/// Returns the error of a rewrite that could not finish: taplo did not run,
/// it rejected a configuration file, it wrote a report that the action does
/// not recognize, or it reported a path outside the project.
async fn fix(
    taplo: &Taplo,
    problems: &[TaploProblem],
    root: &ProjectRoot,
) -> Result<Outcome, FormatTomlError> {
    let observation = taplo.observe(Operation::Format).await?;

    rejected(&observation)?;

    let remaining: HashSet<&PathBuf> = observation
        .problems()
        .iter()
        .map(TaploProblem::path)
        .collect();
    let repaired = problems
        .iter()
        .filter(|problem| !remaining.contains(problem.path()));
    let repairs = repaired
        .map(|problem| finding(problem, root, UNFORMATTED_REPAIR))
        .collect::<Result<Vec<Finding>, FormatTomlError>>()?;
    let findings = findings(observation.problems(), root)?;

    if findings.is_empty() {
        Ok(Outcome::Changed { repairs })
    } else {
        Ok(Outcome::Failed { findings, repairs })
    }
}

/// Returns the outcome of a check that reported no problem
///
/// A pass says how many files taplo checked, so that a reader can question a
/// pass that examined nothing. Taplo can lose the line that carries the
/// count, and a pass without it stays a pass: taplo ended the check with
/// success, and no line that a report loses can turn a check that found
/// problems into one that found none.
fn passed(observation: &Observation) -> Outcome {
    Outcome::Passed {
        summary: observation.checked().map(summary),
    }
}

/// Stops a run whose taplo rejected a configuration file of the project
///
/// Taplo warns and then runs with its defaults, and a run on the defaults
/// quietly does what the project asked it not to do.
///
/// # Errors
///
/// Returns [`RejectedConfiguration`][rejected] when taplo rejected a
/// configuration file.
///
/// [rejected]: FormatTomlError::RejectedConfiguration
fn rejected(observation: &Observation) -> Result<(), FormatTomlError> {
    match observation.rejected_configuration() {
        Some(details) => Err(FormatTomlError::RejectedConfiguration {
            details: details.clone(),
        }),
        None => Ok(()),
    }
}

/// Returns the summary that tells how many files taplo checked
fn summary(checked: u64) -> Summary {
    if checked == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {checked} files"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{Args, ArgsValues, ArgumentValue, Files, Pattern, argument_name};
    use rakko_tool::Tool;

    use super::*;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns the patterns of the files that every run reads as configuration
    ///
    /// They are the configuration of taplo and the files that mise reads to
    /// find taplo.
    fn configuration() -> Vec<Pattern> {
        let mut configuration: Vec<Pattern> = [".taplo.toml", "taplo.toml"]
            .into_iter()
            .map(Pattern::from_static)
            .collect();
        configuration.extend(Tool::resolution_files());

        configuration
    }

    /// Returns the arguments of a run that asks for a repair
    fn fixing() -> FormatTomlArgs {
        let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);

        FormatTomlArgs::from_values(&values).unwrap()
    }

    #[test]
    fn access_with_fix_is_write() {
        let args = fixing();

        let access = FormatToml.access(&context(), &args);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn access_without_fix_is_read() {
        let args = FormatTomlArgs::default();

        let access = FormatToml.access(&context(), &args);

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn files_with_fix_read_the_configuration() {
        let args = fixing();

        let files = FormatToml.files(&context(), &args);

        assert_eq!(files.reads(), &Files::Matching(configuration()));
    }

    #[test]
    fn files_with_fix_write_the_toml_files() {
        let args = fixing();

        let files = FormatToml.files(&context(), &args);

        assert_eq!(
            files.writes(),
            &Files::Matching(vec![Pattern::from_static("**/*.toml")])
        );
    }

    #[test]
    fn files_without_fix_read_the_toml_files_and_the_configuration() {
        let args = FormatTomlArgs::default();
        let mut expected = vec![Pattern::from_static("**/*.toml")];
        expected.extend(configuration());

        let files = FormatToml.files(&context(), &args);

        assert_eq!(files.reads(), &Files::Matching(expected));
    }

    #[test]
    fn files_without_fix_write_no_file() {
        let args = FormatTomlArgs::default();

        let files = FormatToml.files(&context(), &args);

        assert_eq!(files.writes(), &Files::Matching(Vec::new()));
    }
}
