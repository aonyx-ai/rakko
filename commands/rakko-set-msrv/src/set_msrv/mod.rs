/// The arguments that a run of the command reads
mod args;
/// The errors that stop a run of the command
mod error;
/// The version in the root manifest
mod manifest;
/// The Rust pins in the configuration of mise
mod pins;
/// The steps of a run that mise does
mod step;

use std::fmt;
use std::path::Path;

use rakko_action::{ArgsValues, Context, ErasedAction, Name, Outcome, action_name};
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
use rakko_cli::{Command, Report};
use rakko_tool::{Execution, Invocation};
use toml_edit::{DocumentMut, Value};

pub use self::args::{Msrv, Reason, SetMsrvArgs};
pub use self::error::SetMsrvError;
pub use self::step::MiseStep;

/// The root manifest of a project
const MANIFEST: &str = "Cargo.toml";

/// The configuration of mise that pins the toolchains of a project
const CONFIGURATION: &str = "mise.toml";

/// The lock in which mise records what each pin resolved to
const LOCK: &str = "mise.lock";

/// The program that locks the pins of a project and installs the toolchain
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches the command reaches mise as well.
const MISE: &str = "mise";

/// The details of a run of mise that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// The command that sets the minimum supported Rust version of a project
///
/// Three places state the version, and all three must agree: the
/// `rust-version` of the root manifest, the Rust pin in `mise.toml` that
/// check-msrv runs the compiler on, and the entry of that pin in
/// `mise.lock`. A run writes the version and its reason to the manifest,
/// moves the pin that matched the old version, and asks mise to lock the
/// pins again. It then asks mise to install the toolchain at the new version,
/// and runs the check on it, so that one run tells whether the code compiles
/// on the new version.
///
/// A run changes only the version, its comment, the pin, and the lock, so a
/// reviewer reads a diff of a few lines. Mise locks every Rust pin again, so
/// in a project whose pins name exact versions, only the entry of the moved
/// pin changes in the lock.
pub struct SetMsrv {
    /// The action that runs the compiler on the toolchain at the new version
    check: Box<dyn ErasedAction>,
}

impl SetMsrv {
    /// Creates the command from the check that confirms a new version
    ///
    /// The check is the action that the project runs to compile its code on
    /// the toolchain that its manifest names, usually `CheckMsrv` of the
    /// `rakko-check-msrv` crate. The command runs it with no arguments, after
    /// it installed the toolchain. The check is independent of the mount, so a
    /// harness that wants a command for the check mounts it as well.
    pub fn new(check: Box<dyn ErasedAction>) -> Self {
        Self { check }
    }
}

/// Shows the name of the check, because an erased action shows nothing
impl fmt::Debug for SetMsrv {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SetMsrv")
            .field("check", &self.check.name())
            .finish()
    }
}

/// The characters that end a line in a file of the project
///
/// The TOML documents of a run end every line with a line feed, so a file
/// that ends its lines with a carriage return as well gets them back when
/// the run writes it.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum LineEnding {
    /// A line feed alone
    LineFeed,

    /// A carriage return and a line feed
    CarriageReturnLineFeed,
}

impl Command for SetMsrv {
    type Args = SetMsrvArgs;

    // setmsrv[impl name]
    fn name(&self) -> Name {
        action_name!("set-msrv")
    }

    /// Sets the version in the manifest, the pin, and the lock, installs the
    /// toolchain, and checks the code on it
    ///
    /// Each step starts only when the step before it succeeded. A step that
    /// fails after the first write leaves the files as the run wrote them.
    ///
    /// # Errors
    ///
    /// Returns an error when a file cannot be read, parsed, or written, when
    /// the manifest declares no version or declares it in an inline table,
    /// when no pin matches the version of the manifest, when mise does not
    /// lock the new pin or install its toolchain, when the check does not
    /// pass, and when the outcome of the check does not reach the reader.
    async fn run(
        &self,
        project: &Context,
        clawless: &ClawlessContext,
        args: &Self::Args,
    ) -> CommandResult {
        let root = project.root().get();

        set(root, args).await?;
        lock(root, args.msrv(), self.check.name()).await?;
        install(root, args.msrv(), self.check.name()).await?;
        check(self.check.as_ref(), args.msrv(), project, clawless).await?;

        Ok(())
    }
}

/// Sets the version in the manifest and the pin in the root directory of the
/// project
///
/// The run finds everything that it changes before it writes the first file,
/// so a project that the run refuses stays as it was.
///
/// # Errors
///
/// Returns an error when a file cannot be read, parsed, or written, when the
/// manifest declares no version or declares it in an inline table, and when
/// no pin matches the version of the manifest.
// setmsrv[impl files.unreadable]
// setmsrv[impl pin.unmatched]
async fn set(root: &Path, args: &SetMsrvArgs) -> Result<(), SetMsrvError> {
    let manifest_path = root.join(MANIFEST);
    let configuration_path = root.join(CONFIGURATION);
    let (mut manifest, manifest_ending) = read(&manifest_path).await?;
    let (mut configuration, configuration_ending) = read(&configuration_path).await?;

    let current = self::manifest::set(&mut manifest, &manifest_path, args.msrv(), args.reason())?;
    if !self::pins::set(&mut configuration, &current, args.msrv()) {
        return Err(SetMsrvError::UnmatchedPin { version: current });
    }

    write(&manifest_path, &manifest, manifest_ending).await?;
    write(&configuration_path, &configuration, configuration_ending).await
}

/// Has mise lock the Rust pins of the project again
///
/// Mise generates the lock, so it writes the entry of the new pin in its own
/// format and removes the entry of the old one. It locks every Rust pin of
/// the project, so a pin that names a channel or a partial version, such as
/// `nightly` or `1.94`, can move to the newest version that it resolves to.
///
/// A project that keeps no lock is left without one. The version and the
/// check name the steps that a failed lock leaves for the user.
///
/// # Errors
///
/// Returns [`SetMsrvError::MiseUnavailable`] when mise does not start, and
/// [`SetMsrvError::UnlockedPin`] with what mise reported when it ends
/// without success.
// setmsrv[impl lock.update]
// setmsrv[impl lock.absent]
// setmsrv[impl lock.failed]
async fn lock(root: &Path, msrv: &Msrv, check: Name) -> Result<(), SetMsrvError> {
    // A project that keeps no lock has chosen not to, and mise would create
    // one. A lock whose presence is unknown is locked, because a stale lock
    // makes `mise install --locked` fail.
    if matches!(tokio::fs::try_exists(root.join(LOCK)).await, Ok(false)) {
        return Ok(());
    }

    let execution = start(root, MiseStep::Lock).await?;
    if execution.status().success() {
        return Ok(());
    }

    Err(SetMsrvError::UnlockedPin {
        msrv: msrv.clone(),
        check,
        details: diagnosis(&execution),
    })
}

/// Has mise install the toolchain at the new version
///
/// The check needs this toolchain and no other tool, so the run asks mise for
/// nothing else, and a tool that fails to install cannot stop the run. A hook
/// that the project gives mise runs after the install as it always does, and
/// it can do more.
///
/// # Errors
///
/// Returns [`SetMsrvError::MiseUnavailable`] when mise does not start, and
/// [`SetMsrvError::UninstalledToolchain`] with what mise reported when it
/// ends without success.
// setmsrv[impl install.run]
// setmsrv[impl install.failed]
async fn install(root: &Path, msrv: &Msrv, check: Name) -> Result<(), SetMsrvError> {
    let execution = start(root, MiseStep::Install { msrv: msrv.clone() }).await?;
    if execution.status().success() {
        return Ok(());
    }

    Err(SetMsrvError::UninstalledToolchain {
        msrv: msrv.clone(),
        check,
        details: diagnosis(&execution),
    })
}

/// Runs the check on the new version and reports its outcome
///
/// The report says what the check found, so the error of a check that did not
/// pass only names it.
///
/// # Errors
///
/// Returns [`SetMsrvError::UnreportedOutcome`] when the report does not reach
/// the reader, and [`SetMsrvError::FailedCheck`] when the check found
/// problems, stopped, or skipped.
// setmsrv[impl check.given]
// setmsrv[impl check.report]
// setmsrv[impl check.failed]
// setmsrv[impl check.passed]
async fn check(
    check: &dyn ErasedAction,
    msrv: &Msrv,
    project: &Context,
    clawless: &ClawlessContext,
) -> Result<(), SetMsrvError> {
    let name = check.name();
    let outcome = check.run(project, &ArgsValues::empty()).await;

    // A check that skipped did not compile the code on the new version, so it
    // cannot confirm the version.
    let confirmed = match outcome {
        Outcome::Passed { .. } | Outcome::Changed { .. } => true,
        Outcome::Failed { .. } | Outcome::Errored { .. } | Outcome::Skipped { .. } => false,
    };

    clawless
        .output()
        .artifact(Report::new(name.clone(), outcome))
        .await
        .map_err(|source| SetMsrvError::UnreportedOutcome {
            action: name.clone(),
            source,
        })?;

    if confirmed {
        return Ok(());
    }

    Err(SetMsrvError::FailedCheck {
        check: name,
        msrv: msrv.clone(),
    })
}

/// Returns the document in a file of the project and the end of its lines
///
/// # Errors
///
/// Returns [`SetMsrvError::UnreadableFile`] when the file cannot be read,
/// and [`SetMsrvError::MalformedFile`] when it is not valid TOML.
async fn read(path: &Path) -> Result<(DocumentMut, LineEnding), SetMsrvError> {
    let text =
        tokio::fs::read_to_string(path)
            .await
            .map_err(|source| SetMsrvError::UnreadableFile {
                path: path.to_path_buf(),
                source,
            })?;

    let ending = if text.contains("\r\n") {
        LineEnding::CarriageReturnLineFeed
    } else {
        LineEnding::LineFeed
    };
    let document = text.parse().map_err(|source| SetMsrvError::MalformedFile {
        path: path.to_path_buf(),
        source,
    })?;

    Ok((document, ending))
}

/// Writes a document to a file of the project
///
/// The document keeps the comments, the order, and the whitespace of the file
/// that it was read from, so the file changes only where the run changed the
/// document. The lines end as they ended in that file.
///
/// # Errors
///
/// Returns [`SetMsrvError::UnwritableFile`] when the file cannot be written.
// setmsrv[impl files.layout]
async fn write(
    path: &Path,
    document: &DocumentMut,
    ending: LineEnding,
) -> Result<(), SetMsrvError> {
    let text = match ending {
        LineEnding::LineFeed => document.to_string(),
        LineEnding::CarriageReturnLineFeed => document
            .to_string()
            .replace("\r\n", "\n")
            .replace('\n', "\r\n"),
    };

    tokio::fs::write(path, text)
        .await
        .map_err(|source| SetMsrvError::UnwritableFile {
            path: path.to_path_buf(),
            source,
        })
}

/// Has mise do a step in the root directory of the project, and returns how
/// mise ended
///
/// # Errors
///
/// Returns [`SetMsrvError::MiseUnavailable`] when mise does not start.
// setmsrv[impl lock.failed]
// setmsrv[impl install.failed]
async fn start(root: &Path, step: MiseStep) -> Result<Execution, SetMsrvError> {
    Invocation::new(MISE)
        .args(step.arguments())
        .in_directory(root)
        .run()
        .await
        .map_err(|source| SetMsrvError::MiseUnavailable { step, source })
}

/// Returns what mise reported about a step that ended without success
fn diagnosis(execution: &Execution) -> String {
    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Replaces a version and keeps the whitespace and the comment around it
// setmsrv[impl files.layout]
fn replace(value: &mut Value, msrv: &Msrv) {
    let decor = value.decor().clone();
    *value = Value::from(msrv.get());
    *value.decor_mut() = decor;
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::Action;

    use super::*;

    /// An action that the command holds as its check
    struct Stub;

    impl Action for Stub {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("check-msrv")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    // setmsrv[verify name]
    #[test]
    fn name_is_set_msrv() {
        let command = SetMsrv::new(Box::new(Stub));

        assert_eq!(command.name().get(), "set-msrv");
    }

    // A directory that does not exist keeps any program from starting in it.
    // setmsrv[verify install.failed]
    #[tokio::test]
    async fn start_where_mise_cannot_start_names_the_step() {
        let root = Path::new("/rakko/a directory that does not exist");

        let error = start(
            root,
            MiseStep::Install {
                msrv: Msrv::new("1.89.0"),
            },
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "failed to start `mise install rust@1.89.0`"
        );
    }
}
