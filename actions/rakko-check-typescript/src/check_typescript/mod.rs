//! The action that type checks the TypeScript of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps tsc as a subprocess: tsc reads the configuration of the project,
//! collects the files that it selects, and reports what it found, and the
//! action translates that into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, ProjectRoot, SkipReason,
    action_name,
};

pub use self::error::CheckTypeScriptError;
use crate::diagnostic::Diagnostic;
use crate::tsc::Tsc;

/// The numbers of the diagnostic that says tsc found no configuration file
///
/// A project without a `tsconfig.json` at its root is no TypeScript project,
/// and this is how tsc says so. TypeScript renumbered the diagnostic between
/// its major releases, and the action reads the version that the project
/// pinned, so it knows both numbers.
const UNCONFIGURED: [&str; 2] = ["TS5057", "TS5081"];

/// The number of the diagnostic that says the configuration selected no input
///
/// The project states where its TypeScript is, and this is how tsc says that
/// there is none there.
const UNINHABITED: &str = "TS18003";

/// The action that type checks the TypeScript of a project
///
/// The action wraps [tsc]: tsc reads the `tsconfig.json` of the project,
/// collects the files that the configuration selects, and reports every rule
/// of the language that the code breaks. The tsc that runs is the one that
/// [mise] installed for the project, at the version that the project pinned,
/// and the action installs nothing.
///
/// A run reports and emits nothing, and it takes no argument. Every diagnostic
/// of tsc becomes a finding, at the place that tsc marked, and the message of
/// the finding is the sentence that tsc wrote for a reader.
///
/// The action applies to a project that tsc has something to check in, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// tsc, when tsc reports nothing and ends without success, and when tsc writes
/// a report that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_typescript::CheckTypeScript;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckTypeScript)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [tsc]: https://www.typescriptlang.org/docs/handbook/compiler-options.html
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckTypeScript;

impl Action for CheckTypeScript {
    // checktypescript[impl args.none]
    type Args = ();

    // checktypescript[impl name]
    fn name(&self) -> Name {
        action_name!("check-typescript")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checktypescript[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves tsc, checks the project with it, and turns the diagnostics
/// into an outcome. An error that this function returns stops the run, and the
/// caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, the tsc run, or the reading of the report.
async fn drive(context: &Context) -> Result<Outcome, CheckTypeScriptError> {
    // checktypescript[impl tool.missing]
    // checktypescript[impl tool.tsc]
    let tsc = Tsc::resolve(context.root().clone())
        .await
        .map_err(|source| CheckTypeScriptError::UnresolvedTool { source })?;

    // checktypescript[impl check.read]
    let diagnostics = tsc.observe().await?;

    if let Some(outcome) = guard(&diagnostics) {
        return Ok(outcome);
    }

    Ok(Outcome::Failed {
        findings: findings(&diagnostics, context.root())?,
        repairs: Vec::new(),
    })
}

/// Returns the finding that reports one diagnostic of tsc
///
/// The finding sits at the place that tsc marked, and it belongs to the
/// project where tsc marked none. The message comes from tsc, so a reader of a
/// finding reads what the compiler itself would have told them.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of the diagnostic.
///
/// [foreign]: CheckTypeScriptError::ForeignPath
// checktypescript[impl check.diagnostic]
// checktypescript[impl check.elaboration]
// checktypescript[impl check.project]
fn finding(diagnostic: &Diagnostic, root: &ProjectRoot) -> Result<Finding, CheckTypeScriptError> {
    let location = match diagnostic.origin() {
        Some(origin) => {
            let path =
                origin
                    .relative_path(root)
                    .ok_or_else(|| CheckTypeScriptError::ForeignPath {
                        path: origin.path().clone(),
                    })?;

            let position = Position::builder()
                .line(origin.line())
                .column(origin.column())
                .build();

            Location::Position { path, position }
        }
        None => Location::Project,
    };

    Ok(Finding::builder()
        .message(diagnostic.message())
        .location(location)
        .build())
}

/// Returns the findings that report the given diagnostics
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain the
/// path of a diagnostic.
///
/// [foreign]: CheckTypeScriptError::ForeignPath
fn findings(
    diagnostics: &[Diagnostic],
    root: &ProjectRoot,
) -> Result<Vec<Finding>, CheckTypeScriptError> {
    diagnostics
        .iter()
        .map(|diagnostic| finding(diagnostic, root))
        .collect()
}

/// Returns what a run reports when it cannot answer from its findings
///
/// Two answers of tsc end a run before its findings matter. A run that
/// reported one diagnostic about having nothing to check found no TypeScript
/// project here, which is not a clean project but an action that does not
/// apply. A run that reported nothing found a project that holds together.
///
/// A diagnostic about having nothing to check is the only thing that such a
/// run reports, so a report that holds one beside other diagnostics belongs to
/// a project that tsc did check, and the findings answer for it.
///
/// Returns `None` when the caller reports what the run found.
fn guard(diagnostics: &[Diagnostic]) -> Option<Outcome> {
    if let [only] = diagnostics
        && let Some(reason) = unexamined(only)
    {
        return Some(Outcome::Skipped { reason });
    }

    if !diagnostics.is_empty() {
        return None;
    }

    // checktypescript[impl check.passed]
    Some(Outcome::Passed { summary: None })
}

/// Returns why the action does not apply, for a diagnostic that says so
///
/// Returns `None` for a diagnostic about the code of the project, which is a
/// finding and not a reason to skip.
// checktypescript[impl skip.unconfigured]
// checktypescript[impl skip.uninhabited]
fn unexamined(diagnostic: &Diagnostic) -> Option<SkipReason> {
    let number = diagnostic.number().as_str();

    if UNCONFIGURED.contains(&number) || number == UNINHABITED {
        return Some(SkipReason::new(diagnostic.text().clone()));
    }

    None
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;
    use crate::diagnostic::{Category, Origin};

    /// Returns a diagnostic about the code of a file of the project
    fn about_the_code() -> Diagnostic {
        Diagnostic::builder()
            .origin(
                Origin::builder()
                    .path(path("src/index.ts"))
                    .line(2)
                    .column(14)
                    .build(),
            )
            .category(Category::Error)
            .number("TS2322")
            .text("Type 'string' is not assignable to type 'number'.")
            .build()
    }

    /// Returns a diagnostic about the run, which names no file
    fn about_the_run(number: &str, text: &str) -> Diagnostic {
        Diagnostic::builder()
            .category(Category::Error)
            .number(number)
            .text(text)
            .build()
    }

    /// The root that the diagnostics of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    // checktypescript[verify check.diagnostic]
    #[test]
    fn finding_of_a_diagnostic_carries_the_message_of_tsc() {
        let finding = finding(&about_the_code(), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "error TS2322: Type 'string' is not assignable to type 'number'."
        );
    }

    // checktypescript[verify check.diagnostic]
    #[test]
    fn finding_of_a_diagnostic_sits_at_the_position_of_tsc() {
        let finding = finding(&about_the_code(), &root()).unwrap();

        assert_eq!(
            finding.location(),
            &Location::Position {
                path: "src/index.ts".parse().unwrap(),
                position: Position::builder().line(2).column(14).build(),
            }
        );
    }

    // checktypescript[verify check.elaboration]
    #[test]
    fn finding_of_a_diagnostic_that_tsc_explained_carries_the_explanation() {
        let diagnostic = Diagnostic::builder()
            .origin(
                Origin::builder()
                    .path(path("src/index.ts"))
                    .line(2)
                    .column(14)
                    .build(),
            )
            .category(Category::Error)
            .number("TS2322")
            .text("Type '(a: number) => void' is not assignable to type 'F'. Types of parameters 'a' and 'a' are incompatible.")
            .build();

        let finding = finding(&diagnostic, &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "error TS2322: Type '(a: number) => void' is not assignable to type 'F'. Types of \
             parameters 'a' and 'a' are incompatible."
        );
    }

    // checktypescript[verify check.project]
    #[test]
    fn finding_of_a_diagnostic_without_a_file_belongs_to_the_project() {
        let diagnostic = about_the_run(
            "TS6379",
            "Composite projects may not disable incremental compilation.",
        );

        let finding = finding(&diagnostic, &root()).unwrap();

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn finding_of_a_path_outside_the_project_reports_the_path() {
        let diagnostic = Diagnostic::builder()
            .origin(
                Origin::builder()
                    .path(path("/elsewhere/index.ts"))
                    .line(1)
                    .column(1)
                    .build(),
            )
            .category(Category::Error)
            .number("TS2322")
            .text("Type 'string' is not assignable to type 'number'.")
            .build();

        let error = finding(&diagnostic, &root()).unwrap_err();

        assert!(matches!(error, CheckTypeScriptError::ForeignPath { .. }));
    }

    // checktypescript[verify check.passed]
    #[test]
    fn guard_of_a_run_that_reported_nothing_passes() {
        let outcome = guard(&[]);

        assert!(
            matches!(outcome, Some(Outcome::Passed { .. })),
            "expected the run to pass, got {outcome:?}"
        );
    }

    #[test]
    fn guard_of_a_run_that_reported_a_diagnostic_reports_nothing() {
        let outcome = guard(&[about_the_code()]);

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }

    // A project that tsc checked can break a rule about its own configuration
    // beside a rule of the language, and such a run examined the project.
    #[test]
    fn guard_of_a_run_that_reported_more_than_one_diagnostic_reports_nothing() {
        let diagnostics = [
            about_the_run("TS18003", "No inputs were found in config file."),
            about_the_code(),
        ];

        let outcome = guard(&diagnostics);

        assert!(outcome.is_none(), "expected no outcome, got {outcome:?}");
    }

    // checktypescript[verify skip.unconfigured]
    #[test]
    fn guard_of_a_run_without_a_configuration_file_skips() {
        let diagnostics = [about_the_run(
            "TS5081",
            "Cannot find a tsconfig.json file at the current directory: \
             /home/otter/project/tsconfig.json.",
        )];

        let outcome = guard(&diagnostics);

        let Some(Outcome::Skipped { reason }) = outcome else {
            panic!("expected the run to skip, got {outcome:?}");
        };
        assert_eq!(
            reason.get(),
            "Cannot find a tsconfig.json file at the current directory: \
             /home/otter/project/tsconfig.json."
        );
    }

    // checktypescript[verify skip.uninhabited]
    #[test]
    fn guard_of_a_run_whose_configuration_selected_nothing_skips() {
        let diagnostics = [about_the_run(
            "TS18003",
            "No inputs were found in config file '/home/otter/project/tsconfig.json'.",
        )];

        let outcome = guard(&diagnostics);

        assert!(
            matches!(outcome, Some(Outcome::Skipped { .. })),
            "expected the run to skip, got {outcome:?}"
        );
    }
}
