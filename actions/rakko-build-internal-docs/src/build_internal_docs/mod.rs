//! The action that builds the internal documentation of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo as a subprocess: cargo reads the manifests, runs rustdoc over
//! every package of a workspace, and reports what rustdoc said, and the
//! action translates that report into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name};
use rakko_cargo::{Cargo, CargoReport, CargoRoot};
use rakko_tool::Execution;

pub use self::error::BuildInternalDocsError;

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The arguments that ask cargo to document every package of a workspace
///
/// The private items belong to the internal documentation, and a feature
/// that is off by default can carry documentation of its own, so the run
/// covers both. The dependencies stay out, because the documentation of a
/// dependency belongs to the project that publishes it.
///
/// The report arrives as JSON, because the action reads it as data. The
/// format selects the presentation of the report and not the behavior of the
/// tool: which lints apply, and at which level, comes from the configuration
/// of the project alone.
const DOC: [&str; 6] = [
    "doc",
    "--workspace",
    "--no-deps",
    "--document-private-items",
    "--all-features",
    "--message-format=json",
];

/// The action that builds the internal documentation of a project
///
/// The action wraps [rustdoc]: cargo reads the manifests of the project and
/// renders the documentation of every package, with the private items and
/// with every feature, so a run agrees with a contributor that runs
/// `cargo doc` bare. The cargo that runs is the one that [mise] installed for
/// the project, at the version that the project pinned, and the action
/// installs nothing.
///
/// Building the documentation is also the only examination that it gets.
/// Rustdoc resolves the links between items while it renders them, and a link
/// that names nothing is a warning that no other tool reports. Every
/// diagnostic becomes a finding at the range that the compiler named, with
/// the message of the compiler and the code of the lint, and a run with a
/// finding fails, whether the diagnostic is a warning or an error.
///
/// A run takes no argument. It documents every workspace of the project,
/// because the harness of a project is a package of its own. The
/// documentation goes where cargo builds, and the sources stay as they are.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// cargo, when the workspaces of the project cannot be discovered, and when
/// cargo writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_build_internal_docs::BuildInternalDocs;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(BuildInternalDocs)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [rustdoc]: https://doc.rust-lang.org/rustdoc/
#[derive(Copy, Clone, Debug, Default)]
pub struct BuildInternalDocs;

impl Action for BuildInternalDocs {
    // buildinternaldocs[impl args.none]
    type Args = ();

    // buildinternaldocs[impl name]
    fn name(&self) -> Name {
        action_name!("build-internal-docs")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // buildinternaldocs[impl roots.error]
            // buildinternaldocs[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo, discovers the workspaces,
/// and documents each of them. An error that this function returns stops the
/// run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, the discovery of the workspaces, a cargo run, or the reading
/// of a report.
async fn drive(context: &Context) -> Result<Outcome, BuildInternalDocsError> {
    // buildinternaldocs[impl skip.git]
    // buildinternaldocs[impl skip.links]
    // buildinternaldocs[impl skip.missing]
    // buildinternaldocs[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // buildinternaldocs[impl tool.cargo]
    // buildinternaldocs[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| BuildInternalDocsError::UnresolvedTool { source })?;

    // buildinternaldocs[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| BuildInternalDocsError::UndiscoveredRoots { source })?;

    let mut findings = Vec::new();

    // buildinternaldocs[impl roots.all]
    for root in &roots {
        findings.extend(document(&cargo, root, context).await?);
    }

    if findings.is_empty() {
        // buildinternaldocs[impl build.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(roots.len())),
        })
    } else {
        // buildinternaldocs[impl build.failed]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Documents one workspace of the project and returns the findings
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run, and
/// [`UnrecognizedReport`][unrecognized] when cargo writes a report that the
/// action cannot answer from.
///
/// [unavailable]: BuildInternalDocsError::CargoUnavailable
/// [unrecognized]: BuildInternalDocsError::UnrecognizedReport
// buildinternaldocs[impl build.diagnostic]
// buildinternaldocs[impl build.operation]
// buildinternaldocs[impl build.sources]
async fn document(
    cargo: &Cargo,
    root: &CargoRoot,
    context: &Context,
) -> Result<Vec<Finding>, BuildInternalDocsError> {
    let execution = cargo
        .invocation(root)
        .args(DOC)
        .run()
        .await
        .map_err(|source| BuildInternalDocsError::CargoUnavailable { source })?;

    let report = read(root, &execution.stdout().to_string_lossy())?;

    // buildinternaldocs[impl build.unrecognized]
    if !recognized(&report, &execution) {
        return Err(BuildInternalDocsError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: execution.stderr().to_string_lossy().into_owned(),
        });
    }

    Ok(report
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.finding(root, context.root()))
        .collect())
}

/// Returns whether the action can answer from a report
///
/// A run that ended without success and named no diagnostic, and a run that
/// ended with success without saying that the build finished, both wrote a
/// report that the action cannot read, and an answer built on such a report
/// would hide problems behind a green result.
// buildinternaldocs[impl build.unrecognized]
fn recognized(report: &CargoReport, execution: &Execution) -> bool {
    if execution.status().success() {
        report.finished() == Some(true)
    } else {
        !report.diagnostics().is_empty()
    }
}

/// Reads what cargo reported at a root
///
/// # Errors
///
/// Returns [`UnreadableReport`][unreadable] when the report holds a record
/// of cargo that the action cannot read. The shape of a record belongs to a
/// version of cargo, and an answer built on a report with such a record
/// would hide problems behind a green result.
///
/// [unreadable]: BuildInternalDocsError::UnreadableReport
// buildinternaldocs[impl build.unreadable]
fn read(root: &CargoRoot, stdout: &str) -> Result<CargoReport, BuildInternalDocsError> {
    CargoReport::read(stdout).map_err(|source| BuildInternalDocsError::UnreadableReport {
        root: root.directory().clone(),
        source,
    })
}

/// Returns the summary that tells how many workspaces the run documented
// buildinternaldocs[impl build.passed]
fn summary(roots: usize) -> Summary {
    if roots == 1 {
        Summary::new("documented 1 workspace")
    } else {
        Summary::new(format!("documented {roots} workspaces"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::{Path, PathBuf};

    use super::*;

    // buildinternaldocs[verify build.unreadable]
    #[test]
    fn read_a_record_in_a_shape_the_action_does_not_know_names_the_root() {
        let root = CargoRoot::new(PathBuf::from("/home/otter/project"));

        let report = read(
            &root,
            r#"{"reason":"compiler-message","message":{"level":5}}"#,
        );

        assert!(
            matches!(
                &report,
                Err(BuildInternalDocsError::UnreadableReport { root, .. })
                    if root == Path::new("/home/otter/project")
            ),
            "expected an unreadable report, got {report:?}"
        );
    }

    // buildinternaldocs[verify build.passed]
    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "documented 1 workspace");
    }

    // buildinternaldocs[verify build.passed]
    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "documented 2 workspaces");
    }
}
