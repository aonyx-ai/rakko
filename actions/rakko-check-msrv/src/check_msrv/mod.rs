//! The action that checks a project against the Rust version it promises
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo as a subprocess: cargo reads the manifests, rustup selects
//! the toolchain that the manifests declare, and the compiler examines the
//! code. The action translates what cargo reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name};
use rakko_cargo::{Cargo, CargoReport, CargoRoot, Channel, RustVersion, Toolchain};
use rakko_tool::Execution;

pub use self::error::CheckMsrvError;

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The reason of a run that found no declaration
const NO_DECLARATION: &str =
    "no workspace of the project declares a rust-version in its Cargo.toml";

/// The arguments that ask the compiler to examine every target with every
/// feature
///
/// The check produces no binary, because the question is whether the older
/// toolchain accepts the code. The report arrives as JSON, because the
/// action reads it as data. The format selects the presentation of the
/// report and not the behavior of the tool: which lints apply, and at which
/// level, comes from the configuration of the project alone.
const CHECK: [&str; 4] = [
    "check",
    "--all-targets",
    "--all-features",
    "--message-format=json",
];

/// The action that checks a project against the Rust version it promises
///
/// A package writes the oldest toolchain that it compiles on as the
/// `rust-version` of its manifest, and whoever depends on the package reads
/// that as a fact. The action runs the compiler on the toolchain that the
/// promise names, so the compiler confirms the fact. The cargo that runs is
/// the one that [mise] installed for the project, on the toolchain that mise
/// installed for the declared version, and the action installs nothing.
///
/// A run only reports, and it takes no argument. It checks every workspace
/// of the project that declares a version, each on the toolchain that it
/// declares, because the harness of a project is a package of its own and
/// makes a promise of its own. Every diagnostic becomes a finding at the
/// range that the compiler named, and a run with a finding fails, whether
/// the compiler warned or refused the code.
///
/// The action applies to a project that holds a manifest of cargo and
/// declares a Rust version in it, and it skips visibly otherwise. A run
/// stops with an error when mise reports no cargo or no declared toolchain,
/// when the workspaces of the project cannot be discovered, when a
/// declaration cannot be read, and when cargo writes a report that the
/// action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_msrv::CheckMsrv;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckMsrv)];
/// ```
///
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckMsrv;

impl Action for CheckMsrv {
    // checkmsrv[impl args.none]
    type Args = ();

    // checkmsrv[impl name]
    fn name(&self) -> Name {
        action_name!("check-msrv")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checkmsrv[impl roots.error]
            // checkmsrv[impl tool.missing]
            // checkmsrv[impl tool.unpinned]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo, discovers the workspaces,
/// reads the declaration of each of them, and checks the ones that declare a
/// version. An error that this function returns stops the run, and the
/// caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool or of a toolchain, the discovery of the workspaces, the reading
/// of a declaration, a cargo run, or the reading of a report.
async fn drive(context: &Context) -> Result<Outcome, CheckMsrvError> {
    // checkmsrv[impl skip.git]
    // checkmsrv[impl skip.links]
    // checkmsrv[impl skip.missing]
    // checkmsrv[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // checkmsrv[impl tool.cargo]
    // checkmsrv[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| CheckMsrvError::UnresolvedTool { source })?;

    // checkmsrv[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckMsrvError::UndiscoveredRoots { source })?;

    let declared = declarations(&cargo, &roots).await?;

    // checkmsrv[impl skip.undeclared]
    if declared.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_DECLARATION),
        });
    }

    let mut findings = Vec::new();

    // checkmsrv[impl roots.declared]
    for (root, version) in &declared {
        findings.extend(check(&cargo, root, version, context).await?);
    }

    if findings.is_empty() {
        // checkmsrv[impl check.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(declared.len())),
        })
    } else {
        // checkmsrv[impl check.failed]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Checks one workspace of the project and returns the findings
///
/// The run selects the toolchain of the declaration, so the compiler that
/// answers is the one that the workspace promises to compile on.
///
/// # Errors
///
/// Returns [`UnresolvedToolchain`][toolchain] when mise reports no toolchain
/// for the declared version, [`CargoUnavailable`][unavailable] when cargo
/// does not run, and [`UnrecognizedReport`][unrecognized] when cargo writes a
/// report that the action cannot answer from.
///
/// [toolchain]: CheckMsrvError::UnresolvedToolchain
/// [unavailable]: CheckMsrvError::CargoUnavailable
/// [unrecognized]: CheckMsrvError::UnrecognizedReport
// checkmsrv[impl check.diagnostic]
// checkmsrv[impl check.operation]
// checkmsrv[impl check.read]
async fn check(
    cargo: &Cargo,
    root: &CargoRoot,
    version: &RustVersion,
    context: &Context,
) -> Result<Vec<Finding>, CheckMsrvError> {
    // checkmsrv[impl tool.toolchain]
    // checkmsrv[impl tool.unpinned]
    let toolchain = Toolchain::resolve(Channel::new(version.get()), context.root())
        .await
        .map_err(|source| CheckMsrvError::UnresolvedToolchain {
            version: version.clone(),
            source,
        })?;

    let execution = cargo
        .invocation_with_toolchain(root, &toolchain)
        .args(CHECK)
        .run()
        .await
        .map_err(|source| CheckMsrvError::CargoUnavailable { source })?;

    let report = read(root, &execution.stdout().to_string_lossy())?;

    // checkmsrv[impl check.unrecognized]
    if !recognized(&report, &execution) {
        return Err(CheckMsrvError::UnrecognizedReport {
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

/// Returns the roots that declare a Rust version, with the version that each
/// of them declares
///
/// A root that declares none is passed over, because it promises nothing
/// that a compiler can confirm.
///
/// # Errors
///
/// Returns [`UnreadableDeclaration`][declaration] when cargo gives no answer
/// about a root that the action can use.
///
/// [declaration]: CheckMsrvError::UnreadableDeclaration
// checkmsrv[impl roots.declared]
// checkmsrv[impl skip.undeclared]
async fn declarations(
    cargo: &Cargo,
    roots: &[CargoRoot],
) -> Result<Vec<(CargoRoot, RustVersion)>, CheckMsrvError> {
    let mut declared = Vec::new();

    for root in roots {
        // checkmsrv[impl roots.error]
        let version = cargo.rust_version(root).await.map_err(|source| {
            CheckMsrvError::UnreadableDeclaration {
                root: root.directory().clone(),
                source,
            }
        })?;

        if let Some(version) = version {
            declared.push((root.clone(), version));
        }
    }

    Ok(declared)
}

/// Returns whether the action can answer from a report
///
/// A run that ended without success and named no diagnostic, and a run that
/// ended with success without saying that the build finished, both wrote a
/// report that the action cannot read, and an answer built on such a report
/// would hide problems behind a green result.
// checkmsrv[impl check.unrecognized]
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
/// [unreadable]: CheckMsrvError::UnreadableReport
// checkmsrv[impl check.unreadable]
fn read(root: &CargoRoot, stdout: &str) -> Result<CargoReport, CheckMsrvError> {
    CargoReport::read(stdout).map_err(|source| CheckMsrvError::UnreadableReport {
        root: root.directory().clone(),
        source,
    })
}

/// Returns the summary that tells how many workspaces the run checked
// checkmsrv[impl check.passed]
fn summary(roots: usize) -> Summary {
    if roots == 1 {
        Summary::new("checked 1 workspace")
    } else {
        Summary::new(format!("checked {roots} workspaces"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::{Path, PathBuf};

    use super::*;

    // checkmsrv[verify check.unreadable]
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
                Err(CheckMsrvError::UnreadableReport { root, .. })
                    if root == Path::new("/home/otter/project")
            ),
            "expected an unreadable report, got {report:?}"
        );
    }

    // checkmsrv[verify check.passed]
    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 workspace");
    }

    // checkmsrv[verify check.passed]
    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "checked 2 workspaces");
    }
}
