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
/// The report that taplo writes about a format run
mod report;

use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rakko_action::{
    Action, Context, FilePath, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason,
    Summary, action_name,
};
use rakko_tool::{Execution, Tool, ToolName};

pub use self::args::FormatTomlArgs;
pub use self::error::FormatTomlError;
use self::report::{ProblemDetail, TaploProblem, TaploReport};

/// The number of times one taplo operation starts before the action gives up
///
/// Taplo can lose the tail of its report when it exits, and a fresh run
/// almost always answers completely, so a few attempts separate a lost tail
/// from a report that the action genuinely does not understand.
const ATTEMPTS: u32 = 6;

/// The pause that grows between the attempts of one operation
///
/// The loss correlates with the load of the machine, so attempts that follow
/// each other immediately tend to lose together. A growing pause carries the
/// later attempts past the moment of load.
const BACKOFF: Duration = Duration::from_millis(25);

/// The name that mise knows the tool by
const TAPLO: &str = "taplo";

/// The subcommand of taplo that formats TOML files
const FORMAT: &str = "fmt";

/// The flag that asks taplo to report instead of rewriting
const CHECK: &str = "--check";

/// The flag that selects how taplo colors its report
const COLORS: &str = "--colors";

/// The value that asks taplo for a report without color codes
///
/// The action reads the report as data, and a color code inside a path would
/// corrupt the parse. The flag selects the presentation of the report and
/// not the behavior of the tool: what taplo does to the project comes from
/// the configuration of the project alone.
const PLAIN: &str = "never";

/// The extension of the files that the action formats
const TOML_EXTENSION: &str = "toml";

/// The directory entry that the applicability look does not read
const GIT_DIRECTORY: &str = ".git";

/// The reason of a run that found no TOML file
const NO_TOML_FILES: &str = "the project holds no file with the .toml extension";

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
/// The action applies to a project that holds TOML files, and it skips
/// visibly otherwise. A run stops with an error when mise reports no taplo,
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

    // formattoml[impl name]
    fn name(&self) -> Name {
        action_name!("format-toml")
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            // formattoml[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Returns whether the project holds a file that taplo would look at
///
/// The look walks the project from its root and stops at the first file with
/// the `.toml` extension, in the case that taplo matches. It reads hidden
/// directories, because taplo reads them, and it does not read the `.git`
/// entry, which holds no file of the project. It follows no symbolic link,
/// so a cycle of links cannot trap it.
///
/// A directory that the look cannot read counts as applicable. A look that
/// cannot prove absence must not hide a real check behind a skip, and taplo
/// reports its own failure when the run reaches it.
// formattoml[impl skip.git]
// formattoml[impl skip.links]
// formattoml[impl skip.missing]
async fn applies(root: &ProjectRoot) -> bool {
    let mut pending = vec![root.get().to_path_buf()];

    while let Some(directory) = pending.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&directory).await else {
            return true;
        };

        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => {
                    if entry.file_name() == GIT_DIRECTORY {
                        continue;
                    }

                    let Ok(kind) = entry.file_type().await else {
                        return true;
                    };

                    if kind.is_dir() {
                        pending.push(entry.path());
                    } else if kind.is_file()
                        && entry.path().extension() == Some(OsStr::new(TOML_EXTENSION))
                    {
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

/// The operation that one taplo run performs
///
/// Every run of the action starts with a check, and a fix follows it with a
/// rewrite, so the two operations share everything but one flag.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Operation {
    /// Report the problems and rewrite nothing
    Check,
    /// Rewrite the files that taplo can format
    Rewrite,
}

/// Returns whether a report carries everything that its exit status promises
///
/// A run that ended without success closes its report with the summary of
/// the failure, and a run that ended with success closes it with the count
/// of the files. A report without its closing line lost its tail, and the
/// problems that it holds can be incomplete.
fn complete(execution: &Execution, observed: &TaploReport) -> bool {
    if execution.status().success() {
        observed.checked.is_some()
    } else {
        observed.failure_reported
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
    // formattoml[impl skip.missing]
    if !applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_TOML_FILES),
        });
    }

    // formattoml[impl tool.taplo]
    // formattoml[impl tool.missing]
    let tool = Tool::resolve(ToolName::new(TAPLO), context.root().clone())
        .await
        .map_err(|source| FormatTomlError::UnresolvedTool { source })?;

    // formattoml[impl check.read]
    let (execution, observed, stderr) = observe(&tool, Operation::Check).await?;
    let TaploReport {
        rejected_configuration,
        checked,
        problems,
        ..
    } = observed;

    // formattoml[impl check.configuration]
    if let Some(details) = rejected_configuration {
        return Err(FormatTomlError::RejectedConfiguration { details });
    }

    if problems.is_empty() {
        return passed(&execution, checked, stderr);
    }

    if args.fix() {
        fix(&tool, problems, context.root()).await
    } else {
        // formattoml[impl check.unformatted]
        // formattoml[impl check.invalid]
        Ok(Outcome::Failed {
            findings: findings(problems, context.root())?,
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
    problem: TaploProblem,
    root: &ProjectRoot,
    unformatted_message: &str,
) -> Result<Finding, FormatTomlError> {
    let path = relative(&problem.path, root)?;

    let finding = match problem.detail {
        ProblemDetail::Unformatted => Finding::builder()
            .message(unformatted_message)
            .location(Location::File { path })
            .build(),
        ProblemDetail::Diagnostic {
            line,
            column,
            message,
        } => Finding::builder()
            .message(message)
            .location(Location::Position {
                path,
                position: Position::builder().line(line).column(column).build(),
            })
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
    problems: Vec<TaploProblem>,
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatTomlError> {
    problems
        .into_iter()
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
// formattoml[impl fix.write]
async fn fix(
    tool: &Tool,
    problems: Vec<TaploProblem>,
    root: &ProjectRoot,
) -> Result<Outcome, FormatTomlError> {
    let (execution, observed, stderr) = observe(tool, Operation::Rewrite).await?;
    let TaploReport {
        rejected_configuration,
        problems: unrepaired,
        ..
    } = observed;

    // formattoml[impl check.configuration]
    if let Some(details) = rejected_configuration {
        return Err(FormatTomlError::RejectedConfiguration { details });
    }

    // formattoml[impl check.unrecognized]
    if unrepaired.is_empty() && !execution.status().success() {
        return Err(FormatTomlError::UnrecognizedReport { stderr });
    }

    let remaining: HashSet<PathBuf> = unrepaired
        .iter()
        .map(|problem| problem.path.clone())
        .collect();
    let repaired = problems
        .into_iter()
        .filter(|problem| !remaining.contains(&problem.path));
    let repairs = repaired
        .map(|problem| finding(problem, root, UNFORMATTED_REPAIR))
        .collect::<Result<Vec<Finding>, FormatTomlError>>()?;
    let findings = findings(unrepaired, root)?;

    if findings.is_empty() {
        // formattoml[impl fix.changed]
        Ok(Outcome::Changed { repairs })
    } else {
        // formattoml[impl fix.partial]
        Ok(Outcome::Failed { findings, repairs })
    }
}

/// Runs one taplo operation until its report arrives complete
///
/// Taplo can lose the tail of its report when it exits, and a lost tail can
/// hide problems that taplo found. A run whose report is incomplete
/// therefore starts again, a few times, before the action gives up.
/// Repeating a check reads the project again, and repeating a rewrite
/// formats files that a previous attempt already formatted, so both
/// operations repeat safely.
///
/// # Errors
///
/// Returns [`TaploUnavailable`][unavailable] when taplo does not run, and
/// [`UnrecognizedReport`][unrecognized] when every attempt lost part of its
/// report.
///
/// [unavailable]: FormatTomlError::TaploUnavailable
/// [unrecognized]: FormatTomlError::UnrecognizedReport
// formattoml[impl check.unrecognized]
async fn observe(
    tool: &Tool,
    operation: Operation,
) -> Result<(Execution, TaploReport, String), FormatTomlError> {
    let mut stderr = String::new();

    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            tokio::time::sleep(BACKOFF * attempt).await;
        }

        let execution = start(tool, operation).await?;
        stderr = execution.stderr().to_string_lossy().into_owned();
        let observed = report::parse(&stderr);

        if complete(&execution, &observed) {
            return Ok((execution, observed, stderr));
        }
    }

    Err(FormatTomlError::UnrecognizedReport { stderr })
}

/// Returns the outcome of a check that reported no problem
///
/// A pass needs the count of the files that taplo checked, so that a reader
/// can question a pass that examined nothing. A check that ended without
/// success, and a check that passed without the count, both wrote a report
/// that the action does not recognize, and the run stops instead of hiding
/// problems behind a green result.
///
/// # Errors
///
/// Returns [`UnrecognizedReport`][unrecognized] when the check ended without
/// success or reported no count of the files.
///
/// [unrecognized]: FormatTomlError::UnrecognizedReport
// formattoml[impl check.passed]
// formattoml[impl check.unrecognized]
fn passed(
    execution: &Execution,
    checked: Option<u64>,
    stderr: String,
) -> Result<Outcome, FormatTomlError> {
    if !execution.status().success() {
        return Err(FormatTomlError::UnrecognizedReport { stderr });
    }

    let Some(checked) = checked else {
        return Err(FormatTomlError::UnrecognizedReport { stderr });
    };

    Ok(Outcome::Passed {
        summary: Some(summary(checked)),
    })
}

/// Returns the path of a finding, relative to the project root
///
/// Taplo starts in the project root and reports absolute paths, so the root
/// prefixes each of them. The root of the context can name the same
/// directory through a symbolic link, which is why the canonical root is
/// tried as well.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path.
///
/// [foreign]: FormatTomlError::ForeignPath
fn relative(path: &Path, root: &ProjectRoot) -> Result<FilePath, FormatTomlError> {
    let stripped = strip(path, root).ok_or_else(|| FormatTomlError::ForeignPath {
        path: path.to_path_buf(),
    })?;

    FilePath::try_from(stripped).map_err(|_| FormatTomlError::ForeignPath {
        path: path.to_path_buf(),
    })
}

/// Starts one taplo operation and collects what it produced
///
/// A check asks taplo to report and rewrites nothing, and a rewrite lets
/// taplo format the files that it can format. Both read the report as data,
/// so both ask for plain output: a color code inside a path would corrupt
/// the parse, and the flag selects the presentation of the report and not
/// the behavior of the tool.
///
/// # Errors
///
/// Returns [`TaploUnavailable`][unavailable] when taplo does not run.
///
/// [unavailable]: FormatTomlError::TaploUnavailable
// formattoml[impl check.read]
// formattoml[impl fix.write]
async fn start(tool: &Tool, operation: Operation) -> Result<Execution, FormatTomlError> {
    let invocation = tool.invocation().arg(FORMAT);
    let invocation = match operation {
        Operation::Check => invocation.arg(CHECK),
        Operation::Rewrite => invocation,
    };

    invocation
        .arg(COLORS)
        .arg(PLAIN)
        .run()
        .await
        .map_err(|source| FormatTomlError::TaploUnavailable { source })
}

/// Returns the path without the project root that prefixes it
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(root.get()) {
        return Some(stripped.to_path_buf());
    }

    let canonical = root.get().canonicalize().ok()?;

    path.strip_prefix(canonical).ok().map(Path::to_path_buf)
}

/// Returns the summary that tells how many files taplo checked
// formattoml[impl check.passed]
fn summary(checked: u64) -> Summary {
    if checked == 1 {
        Summary::new("checked 1 file")
    } else {
        Summary::new(format!("checked {checked} files"))
    }
}
