//! The action that formats the TypeScript of a project
//!
//! This module holds the action, the arguments that a run reads, and the error
//! that stops a run. The action wraps oxfmt as a subprocess: it names the files
//! of the language, oxfmt reads its own configuration and does the formatting,
//! and the action translates what oxfmt reported into an outcome.

/// The arguments that a run of the action reads
mod args;
/// The error that stops a run of the action
mod error;

use std::path::Path;

use rakko_action::{
    Action, Context, FilePath, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason,
    action_name,
};

pub use self::args::FormatTypeScriptArgs;
pub use self::error::FormatTypeScriptError;
use crate::observation::Observation;
use crate::oxfmt::Oxfmt;
use crate::problem::{OxfmtProblem, relative_path};

/// The reason of a run whose oxfmt found nothing to examine
///
/// Oxfmt refuses a pattern that matches no file, so this is what a project
/// without files of the language reports. A project whose ignore files exclude
/// every one of them reports it as well.
const NOTHING_TO_EXAMINE: &str = "oxfmt found no TypeScript file to examine";

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not formatted";

/// The action that formats the TypeScript of a project
///
/// The action wraps [oxfmt]: the action names the files of the language, oxfmt
/// reads its own configuration and formats what its ignore files leave, so a
/// run agrees with an editor and with a contributor that runs oxfmt bare. The
/// oxfmt that runs is the one that [mise] installed for the project, at the
/// version that the project pinned, and the action installs nothing.
///
/// Oxfmt formats more languages than the action asks it about, and it would
/// otherwise rewrite the Markdown, the JSON, and the YAML of a project that
/// another action already formats. The action therefore names the extensions
/// of TypeScript and of JavaScript, and oxfmt never sees the rest.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file, a file that oxfmt cannot parse becomes a finding at
/// the position that oxfmt reports, and a failure that oxfmt placed nowhere
/// becomes a finding about the project. With the `fix` argument, oxfmt
/// rewrites the files that it can format, and the outcome carries one repair
/// for each file that changed, next to the problems that remain.
///
/// A run whose oxfmt found no file of the language skips visibly, and says so.
/// A run stops with an error when mise reports no oxfmt, when oxfmt refused a
/// configuration of the project, and when oxfmt writes a report that the
/// action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_format_typescript::FormatTypeScript;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatTypeScript)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [oxfmt]: https://oxc.rs/docs/guide/usage/formatter.html
#[derive(Copy, Clone, Debug, Default)]
pub struct FormatTypeScript;

impl Action for FormatTypeScript {
    type Args = FormatTypeScriptArgs;

    // formattypescript[impl name]
    fn name(&self) -> Name {
        action_name!("format-typescript")
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            // formattypescript[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves oxfmt, and then reports or rewrites, depending on the
/// arguments. An error that this function returns stops the run, and the
/// caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, or an oxfmt run.
async fn drive(
    context: &Context,
    args: &FormatTypeScriptArgs,
) -> Result<Outcome, FormatTypeScriptError> {
    // formattypescript[impl tool.missing]
    // formattypescript[impl tool.oxfmt]
    let oxfmt = Oxfmt::resolve(context.root().clone())
        .await
        .map_err(|source| FormatTypeScriptError::UnresolvedTool { source })?;

    if args.fix() {
        fix(&oxfmt, context.root()).await
    } else {
        report(&oxfmt, context.root()).await
    }
}

/// Lets oxfmt rewrite the project and reports what the rewrite left
///
/// Oxfmt names no file that it rewrote, so the rewrite alone says only that it
/// happened. The action therefore asks for the list of files before the
/// rewrite and again after it: what the first list holds and the second one
/// does not is what the run repaired, and what the second list still holds is
/// what remains.
///
/// A file that oxfmt cannot parse remains, and so does a file that it cannot
/// write. A rewrite repairs formatting, and a file that the tool cannot read
/// or a tree that refuses a write needs a hand.
///
/// # Errors
///
/// Returns the error of a run that could not finish: oxfmt did not run, it
/// refused a configuration of the project, it wrote a report that the action
/// does not recognize, or it reported a path outside the project.
async fn fix(oxfmt: &Oxfmt, root: &ProjectRoot) -> Result<Outcome, FormatTypeScriptError> {
    let before = oxfmt.list().await?;

    if let Some(outcome) = guard(&before)? {
        return Ok(outcome);
    }

    if before.problems().is_empty() {
        // formattypescript[impl check.passed]
        return Ok(Outcome::Passed { summary: None });
    }

    // formattypescript[impl fix.write]
    oxfmt.rewrite().await?;

    let after = oxfmt.list().await?;

    if let Some(outcome) = guard(&after)? {
        return Ok(outcome);
    }

    let repairs = repairs(&before, &after, root)?;
    let findings = findings(after.problems(), root)?;

    if !findings.is_empty() {
        // formattypescript[impl fix.partial]
        return Ok(Outcome::Failed { findings, repairs });
    }

    // formattypescript[impl fix.changed]
    Ok(Outcome::Changed { repairs })
}

/// Returns the repairs that report the files the rewrite changed
///
/// A file that the first list named and the second one does not was rewritten,
/// which is the problem that the repair took away. A file that both lists name
/// is a file that the rewrite could not take away, and it travels as a finding
/// instead.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a file.
///
/// [foreign]: FormatTypeScriptError::ForeignPath
// formattypescript[impl fix.changed]
// formattypescript[impl fix.partial]
fn repairs(
    before: &Observation,
    after: &Observation,
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatTypeScriptError> {
    let remaining: Vec<&Path> = after
        .problems()
        .iter()
        .filter_map(OxfmtProblem::path)
        .collect();

    before
        .problems()
        .iter()
        .filter(|problem| matches!(problem, OxfmtProblem::Unformatted { .. }))
        .filter(|problem| !problem.path().is_some_and(|path| remaining.contains(&path)))
        .map(|problem| finding(problem, root, UNFORMATTED_REPAIR))
        .collect()
}

/// Returns the finding that reports one problem of the project
///
/// A file that is not formatted gets a finding at the level of the file, a
/// file that oxfmt could not format gets a finding at the place that oxfmt
/// named, and a failure that oxfmt placed nowhere gets a finding about the
/// project, because oxfmt writes the file into the sentence instead. The
/// message for an unformatted file comes from the caller, because a finding
/// states a problem that the project has and a repair states one that the run
/// took away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of the problem.
///
/// [foreign]: FormatTypeScriptError::ForeignPath
fn finding(
    problem: &OxfmtProblem,
    root: &ProjectRoot,
    unformatted_message: &str,
) -> Result<Finding, FormatTypeScriptError> {
    let finding = match problem {
        // formattypescript[impl check.invalid]
        OxfmtProblem::Invalid {
            path,
            line,
            column,
            message,
        } => Finding::builder()
            .message(message.clone())
            .location(Location::Position {
                path: relative(path, root)?,
                position: Position::builder().line(*line).column(*column).build(),
            })
            .build(),
        // formattypescript[impl check.unformatted]
        OxfmtProblem::Unformatted { path } => Finding::builder()
            .message(unformatted_message)
            .location(Location::File {
                path: relative(path, root)?,
            })
            .build(),
        // formattypescript[impl check.unplaced]
        OxfmtProblem::Unplaced { message } => Finding::builder()
            .message(message.clone())
            .location(Location::Project)
            .build(),
    };

    Ok(finding)
}

/// Returns the path of a file, relative to the project root
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// file.
///
/// [foreign]: FormatTypeScriptError::ForeignPath
fn relative(path: &Path, root: &ProjectRoot) -> Result<FilePath, FormatTypeScriptError> {
    relative_path(path, root).ok_or_else(|| FormatTypeScriptError::ForeignPath {
        path: path.to_path_buf(),
    })
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a problem.
///
/// [foreign]: FormatTypeScriptError::ForeignPath
fn findings(
    problems: &[OxfmtProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatTypeScriptError> {
    problems
        .iter()
        .map(|problem| finding(problem, root, UNFORMATTED_FINDING))
        .collect()
}

/// Returns what a run reports when it cannot answer from its own findings
///
/// Three conditions end a run before its findings matter. A configuration that
/// oxfmt refused stops it, because oxfmt then formatted nothing at all and the
/// project asked for rules that never applied. An oxfmt that matched no file
/// leaves the action with nothing to examine, which the ignore files of the
/// project explain, so the run skips. A run that ended without success and
/// named no problem wrote a report that the action could not read, and an
/// answer built on it would hide every problem behind a green result.
///
/// Returns `None` when the caller reports what the run found.
///
/// # Errors
///
/// Returns [`RejectedConfiguration`][rejected] when oxfmt refused a
/// configuration, and [`UnrecognizedReport`][unrecognized] when the report says
/// nothing that the action can answer from.
///
/// [rejected]: FormatTypeScriptError::RejectedConfiguration
/// [unrecognized]: FormatTypeScriptError::UnrecognizedReport
fn guard(observation: &Observation) -> Result<Option<Outcome>, FormatTypeScriptError> {
    // formattypescript[impl check.configuration]
    if let Some(details) = observation.rejected_configuration() {
        return Err(FormatTypeScriptError::RejectedConfiguration {
            details: details.clone(),
        });
    }

    // formattypescript[impl skip.unmatched]
    if observation.unmatched_pattern() {
        return Ok(Some(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_EXAMINE),
        }));
    }

    // formattypescript[impl check.unrecognized]
    if observation.problems().is_empty() && !observation.succeeded() {
        return Err(FormatTypeScriptError::UnrecognizedReport {
            stderr: observation.stderr().clone(),
        });
    }

    Ok(None)
}

/// Asks oxfmt what it would change and reports the answer
///
/// The run changes nothing: oxfmt names the files that a rewrite would change,
/// and each of them becomes a finding next to the files that oxfmt could not
/// format.
///
/// # Errors
///
/// Returns the error of a run that could not finish: oxfmt did not run, it
/// refused a configuration of the project, it wrote a report that the action
/// does not recognize, or it reported a path outside the project.
// formattypescript[impl check.read]
async fn report(oxfmt: &Oxfmt, root: &ProjectRoot) -> Result<Outcome, FormatTypeScriptError> {
    let observation = oxfmt.list().await?;

    if let Some(outcome) = guard(&observation)? {
        return Ok(outcome);
    }

    if observation.problems().is_empty() {
        // formattypescript[impl check.passed]
        return Ok(Outcome::Passed { summary: None });
    }

    Ok(Outcome::Failed {
        findings: findings(observation.problems(), root)?,
        repairs: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    /// The sentence of a failure that oxfmt placed nowhere
    const UNPLACED: &str = "Failed to read file: /home/otter/project/src/index.ts";

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    // formattypescript[verify check.unplaced]
    #[test]
    fn finding_of_a_failure_without_a_place_belongs_to_the_project() {
        let problem = OxfmtProblem::Unplaced {
            message: String::from(UNPLACED),
        };

        let finding = finding(&problem, &root(), UNFORMATTED_FINDING).unwrap();

        assert_eq!(finding.location(), &Location::Project);
    }

    // formattypescript[verify check.unplaced]
    #[test]
    fn finding_of_a_failure_without_a_place_carries_the_sentence_of_oxfmt() {
        let problem = OxfmtProblem::Unplaced {
            message: String::from(UNPLACED),
        };

        let finding = finding(&problem, &root(), UNFORMATTED_FINDING).unwrap();

        assert_eq!(finding.message().get(), UNPLACED);
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = OxfmtProblem::Unformatted {
            path: path("/elsewhere/index.ts"),
        };

        let error = finding(&problem, &root(), UNFORMATTED_FINDING).unwrap_err();

        assert!(matches!(error, FormatTypeScriptError::ForeignPath { .. }));
    }

    // formattypescript[verify check.unrecognized]
    #[test]
    fn guard_of_an_unrecognized_report_holds_what_oxfmt_wrote() {
        let observation = Observation::builder().stderr(UNPLACED).build();

        let error = guard(&observation).unwrap_err();

        assert!(matches!(
            error,
            FormatTypeScriptError::UnrecognizedReport { stderr } if stderr == UNPLACED
        ));
    }
}
