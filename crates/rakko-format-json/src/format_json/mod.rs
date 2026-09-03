//! The action that formats the JSON files of a project
//!
//! This module holds the action, the arguments that a run reads, and the error
//! that stops a run. The action wraps prettier as a subprocess: it names the
//! JSON files, prettier reads its own configuration and does the
//! formatting, and the action translates what prettier reported into an
//! outcome.

/// The arguments that a run of the action reads
mod args;
/// The error that stops a run of the action
mod error;

use std::path::PathBuf;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason,
    action_name,
};
use rakko_prettier::{
    FileExtension, Filter, Observation, Operation, Prettier, PrettierProblem, ProblemDetail,
};

pub use self::args::FormatJsonArgs;
pub use self::error::FormatJsonError;

/// The extension of the files that the action formats
const JSON_EXTENSION: &str = "json";

/// The extension of the JSON5 files that the action formats
const JSON5_EXTENSION: &str = "json5";

/// The reason of a run that found no JSON file
const NO_JSON_FILES: &str = "the project holds no file with the .json or .json5 extension";

/// The reason of a run whose prettier found nothing to examine
///
/// The look of the action found a file, and prettier then matched none. The
/// ignore files of the project explain the difference.
const NOTHING_TO_EXAMINE: &str = "prettier found no JSON file to examine";

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not formatted";

/// The action that formats the JSON files of a project
///
/// The action wraps [prettier]: the action names the JSON files, prettier
/// reads its own configuration and formats what its ignore files leave, so a
/// run agrees with an editor and with a contributor that runs prettier bare.
/// The prettier that runs is the one that [mise] installed for the project, at
/// the version that the project pinned, and the action installs nothing.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file, and a file that prettier cannot parse becomes a
/// finding at the position that prettier reports. With the `fix` argument,
/// prettier rewrites the files that it can format, and the outcome carries one
/// repair for each file that it rewrote, next to the problems that remain.
///
/// The action applies to a project that holds JSON files, and it skips
/// visibly otherwise. A run stops with an error when mise reports no prettier,
/// when a configuration of the project did not reach the run, and when
/// prettier writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_format_json::FormatJson;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatJson)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [prettier]: https://prettier.io
#[derive(Copy, Clone, Debug, Default)]
pub struct FormatJson;

impl Action for FormatJson {
    type Args = FormatJsonArgs;

    // formatjson[impl name]
    fn name(&self) -> Name {
        action_name!("format-json")
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            // formatjson[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves prettier, and then reports or
/// rewrites, depending on the arguments. An error that this function returns
/// stops the run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, a prettier run, or the reading of the report.
async fn drive(context: &Context, args: &FormatJsonArgs) -> Result<Outcome, FormatJsonError> {
    let filter = filter();

    // formatjson[impl skip.dependencies]
    // formatjson[impl skip.git]
    // formatjson[impl skip.links]
    // formatjson[impl skip.missing]
    if !Prettier::applies(context.root(), &filter).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_JSON_FILES),
        });
    }

    // formatjson[impl tool.prettier]
    // formatjson[impl tool.missing]
    let prettier = Prettier::resolve(context.root().clone())
        .await
        .map_err(|source| FormatJsonError::UnresolvedTool { source })?;

    if args.fix() {
        fix(&prettier, &filter, context.root()).await
    } else {
        report(&prettier, &filter, context.root()).await
    }
}

/// Returns the filter that selects the files of the action
// formatjson[impl skip.missing]
fn filter() -> Filter {
    Filter::new([
        FileExtension::new(JSON_EXTENSION),
        FileExtension::new(JSON5_EXTENSION),
    ])
}

/// Returns the finding that reports one problem of the project
///
/// A file that is not formatted gets a finding at the level of the file, a
/// file that prettier could not read gets the reason, and a file that prettier
/// could not parse gets a finding at the position that prettier named. The
/// message for an unformatted file comes from the caller, because a finding
/// states a problem that the project has and a repair states one that the run
/// took away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of the problem.
///
/// [foreign]: FormatJsonError::ForeignPath
fn finding(
    problem: &PrettierProblem,
    root: &ProjectRoot,
    unformatted_message: &str,
) -> Result<Finding, FormatJsonError> {
    let path = problem
        .relative_path(root)
        .ok_or_else(|| FormatJsonError::ForeignPath {
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
        ProblemDetail::Unformatted => Finding::builder()
            .message(unformatted_message)
            .location(Location::File { path })
            .build(),
        ProblemDetail::Unreadable { reason } => Finding::builder()
            .message(reason.clone())
            .location(Location::File { path })
            .build(),
    };

    Ok(finding)
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a problem.
///
/// [foreign]: FormatJsonError::ForeignPath
fn findings(
    problems: &[PrettierProblem],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, FormatJsonError> {
    problems
        .iter()
        .map(|problem| finding(problem, root, UNFORMATTED_FINDING))
        .collect()
}

/// Lets prettier rewrite the project and reports what the rewrite left
///
/// Prettier names every file that it examined and marks the ones that it left
/// alone, so one run both repairs the project and says what it repaired. A
/// file that prettier cannot parse or cannot read remains, because a rewrite
/// repairs formatting and a broken file needs a hand.
///
/// # Errors
///
/// Returns the error of a rewrite that could not finish: prettier did not run,
/// a configuration of the project did not reach it, it wrote a report that the
/// action does not recognize, or it reported a path outside the project.
// formatjson[impl fix.write]
async fn fix(
    prettier: &Prettier,
    filter: &Filter,
    root: &ProjectRoot,
) -> Result<Outcome, FormatJsonError> {
    let observation = prettier.observe(Operation::Rewrite, filter).await?;

    if let Some(outcome) = guard(&observation)? {
        return Ok(outcome);
    }

    let repairs = repairs(observation.rewritten(), root)?;
    let findings = findings(observation.problems(), root)?;

    if !findings.is_empty() {
        // formatjson[impl fix.partial]
        return Ok(Outcome::Failed { findings, repairs });
    }

    if repairs.is_empty() {
        return Ok(Outcome::Passed { summary: None });
    }

    // formatjson[impl fix.changed]
    Ok(Outcome::Changed { repairs })
}

/// Returns what a run reports when it cannot answer from its own findings
///
/// Three conditions end a run before its findings matter. A configuration that
/// did not reach prettier stops it, because prettier ignores an option that it
/// does not know and then runs without it, and a run without it quietly does
/// what the project asked it not to do. A prettier that matched no file leaves
/// the action with nothing to examine, which the ignore files of the project
/// explain, so the run skips. A run that ended without success and named no
/// problem wrote a report that the action could not read, and an answer built
/// on it would hide every problem behind a green result.
///
/// Returns `None` when the caller reports what the run found.
///
/// # Errors
///
/// Returns [`RejectedConfiguration`][rejected] when a configuration did not
/// reach prettier, and [`UnrecognizedReport`][unrecognized] when the report
/// says nothing that the action can answer from.
///
/// [rejected]: FormatJsonError::RejectedConfiguration
/// [unrecognized]: FormatJsonError::UnrecognizedReport
fn guard(observation: &Observation) -> Result<Option<Outcome>, FormatJsonError> {
    // formatjson[impl check.configuration]
    if let Some(details) = observation.rejected_configuration() {
        return Err(FormatJsonError::RejectedConfiguration {
            details: details.clone(),
        });
    }

    // formatjson[impl skip.unmatched]
    if observation.unmatched_pattern() {
        return Ok(Some(Outcome::Skipped {
            reason: SkipReason::new(NOTHING_TO_EXAMINE),
        }));
    }

    // formatjson[impl check.unrecognized]
    if observation.problems().is_empty() && !observation.succeeded() {
        return Err(FormatJsonError::UnrecognizedReport {
            stderr: observation.stderr().clone(),
        });
    }

    Ok(None)
}

/// Asks prettier what it would change and reports the answer
///
/// The run changes nothing: prettier names the files that a rewrite would
/// change, and each of them becomes a finding next to the files that prettier
/// could not parse or read.
///
/// # Errors
///
/// Returns the error of a run that could not finish: prettier did not run, a
/// configuration of the project did not reach it, it wrote a report that the
/// action does not recognize, or it reported a path outside the project.
// formatjson[impl check.read]
async fn report(
    prettier: &Prettier,
    filter: &Filter,
    root: &ProjectRoot,
) -> Result<Outcome, FormatJsonError> {
    let observation = prettier.observe(Operation::Report, filter).await?;

    if let Some(outcome) = guard(&observation)? {
        return Ok(outcome);
    }

    if observation.problems().is_empty() {
        // formatjson[impl check.passed]
        return Ok(Outcome::Passed { summary: None });
    }

    // formatjson[impl check.invalid]
    // formatjson[impl check.unformatted]
    // formatjson[impl check.unreadable]
    Ok(Outcome::Failed {
        findings: findings(observation.problems(), root)?,
        repairs: Vec::new(),
    })
}

/// Returns the repairs that report the files a rewrite changed
///
/// A file that prettier rewrote was not formatted before the run, which is the
/// problem that the repair took away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a file.
///
/// [foreign]: FormatJsonError::ForeignPath
// formatjson[impl fix.changed]
fn repairs(rewritten: &[PathBuf], root: &ProjectRoot) -> Result<Vec<Finding>, FormatJsonError> {
    rewritten
        .iter()
        .map(|path| {
            let problem = PrettierProblem::new(path.clone(), ProblemDetail::Unformatted);

            finding(&problem, root, UNFORMATTED_REPAIR)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    // formatjson[verify check.invalid]
    #[test]
    fn finding_of_a_diagnostic_carries_the_message_of_prettier() {
        let problem = PrettierProblem::new(
            PathBuf::from("notes.json"),
            ProblemDetail::Diagnostic {
                line: 2,
                column: 5,
                message: "SyntaxError: Unexpected token".to_owned(),
            },
        );

        let finding = finding(&problem, &root(), UNFORMATTED_FINDING).unwrap();

        assert_eq!(finding.message().get(), "SyntaxError: Unexpected token");
    }

    // formatjson[verify check.invalid]
    #[test]
    fn finding_of_a_diagnostic_sits_at_the_position_of_prettier() {
        let problem = PrettierProblem::new(
            PathBuf::from("notes.json"),
            ProblemDetail::Diagnostic {
                line: 2,
                column: 5,
                message: "SyntaxError: Unexpected token".to_owned(),
            },
        );

        let finding = finding(&problem, &root(), UNFORMATTED_FINDING).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "notes.json".parse().unwrap(),
                position: Position::builder().line(2).column(5).build(),
            }
        );
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let problem = PrettierProblem::new(
            PathBuf::from("/elsewhere/notes.json"),
            ProblemDetail::Unformatted,
        );

        let error = finding(&problem, &root(), UNFORMATTED_FINDING).unwrap_err();

        assert!(matches!(error, FormatJsonError::ForeignPath { .. }));
    }

    // formatjson[verify skip.unmatched]
    #[test]
    fn guard_of_a_run_that_matched_no_file_skips() {
        let observation = Observation::builder()
            .succeeded(true)
            .unmatched_pattern(true)
            .build();

        let outcome = guard(&observation).unwrap();

        assert!(
            matches!(outcome, Some(Outcome::Skipped { .. })),
            "expected the run to skip, got {outcome:?}"
        );
    }

    // formatjson[verify check.unrecognized]
    #[test]
    fn guard_of_a_failed_run_without_a_problem_stops() {
        let observation = Observation::builder().stderr("something new").build();

        let error = guard(&observation).unwrap_err();

        assert!(matches!(error, FormatJsonError::UnrecognizedReport { .. }));
    }

    // formatjson[verify check.configuration]
    #[test]
    fn guard_of_a_run_without_its_configuration_stops() {
        let observation = Observation::builder()
            .succeeded(true)
            .rejected_configuration("Ignored unknown option { notAnOption: 5 }.".to_owned())
            .build();

        let error = guard(&observation).unwrap_err();

        assert!(matches!(
            error,
            FormatJsonError::RejectedConfiguration { .. }
        ));
    }

    #[test]
    fn guard_of_a_run_that_reported_normally_reports_nothing() {
        let observation = Observation::builder().succeeded(true).build();

        let outcome = guard(&observation).unwrap();

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }
}
