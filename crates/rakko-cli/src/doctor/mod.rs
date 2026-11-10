//! The command that checks and repairs the tool pins of the project

mod check_error;
mod declaration;
mod generated;
mod precondition_error;
mod repair_error;
mod shadow;
mod toolchain;

use std::collections::BTreeSet;
use std::error::Error;

use clap::{Arg, ArgAction, ArgMatches};
use rakko::{Context, Finding, Name, Outcome};
use rakko_tool::{Execution, Invocation};

pub(crate) use self::check_error::CheckToolPinsError;
pub(crate) use self::declaration::Declaration;
use self::generated::Pins;
pub(crate) use self::precondition_error::CheckPreconditionError;
pub(crate) use self::repair_error::RepairToolPinsError;
use crate::report::Report;

/// The name of the command
///
/// The command line reserves the name, so no harness mounts an action, a
/// command, or a group with it.
pub(crate) const DOCTOR: &str = "doctor";

/// The name of the flag that asks for the repair
///
/// An action that repairs what it finds takes a flag with this name. So a
/// user who knows the flag of an action knows the flag of `doctor`.
const FIX: &str = "fix";

/// The program that the command asks about the project
const MISE: &str = "mise";

/// The subcommand of mise that locks the tools of the project
const LOCK: &str = "lock";

/// The details of a failure that mise did not explain
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// What a run of `doctor` does with the problems that it finds
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Mode {
    /// The run reports the problems and changes nothing
    Report,
    /// The run repairs what it can, and reports what remains
    Repair,
}

/// Returns the command of `doctor`, with its flag
///
/// The projection adds the command to every harness, so a harness writes no
/// code for it.
pub(crate) fn command() -> clap::Command {
    clap::Command::new(DOCTOR)
        .about("Reports whether the tool pins of the project match the mounted actions")
        .arg(Arg::new(FIX).long(FIX).action(ArgAction::SetTrue).help(
            "Write the generated file of the tool pins, remove each pin of the project \
                     that overrides a declared tool, and lock the tools with mise",
        ))
}

/// Returns what the arguments of the command ask the run to do
pub(crate) fn mode(matches: &ArgMatches) -> Mode {
    if matches.get_flag(FIX) {
        Mode::Repair
    } else {
        Mode::Report
    }
}

/// Returns the name of the command, as a report gives it
pub(crate) fn name() -> Name {
    rakko::action_name!(DOCTOR)
}

/// Checks the tool pins of the project, and repairs them in the repair mode
///
/// The check has three parts. The generated file must match the tools that
/// the mounted actions declare. No configuration file inside the project that
/// mise ranks above the generated file can pin a declared tool. Mise would run
/// that pin instead. Each toolchain that a mounted action requires must be
/// pinned by the project in the range of the action. A project whose actions
/// declare no tool needs no generated file. The run reports a generated file
/// that is left over.
///
/// The check reads the project and asks mise about it, and it needs no
/// network. Mise reads a pin with options, such as the platforms of a tool,
/// only from a configuration that the user trusts. So when the generated file
/// holds such a pin, the check stops with an error until the user trusts the
/// project, for example with `mise trust`. The repair writes the generated file and removes each pin that
/// overrides a declared tool. Then it runs `mise lock`, which needs the
/// network. It locks every tool of the project, so it can also change the
/// lockfile of the configuration of the project.
///
/// In a project whose actions declare no tool, the repair removes a generated
/// file that is left over. Mise keeps the entries of that file in its
/// lockfile, unless another configuration file shares the lockfile. So the
/// repair locks the tools when one does, and removes the lockfile when none
/// does. It locks nothing in such a project otherwise. A toolchain stays as
/// the project pins it, because its version is a decision of the project. Its
/// finding says which pin to change.
///
/// The outcome holds each problem as a finding, and each problem that the
/// repair fixed as a repair. A failure that stops the check or the repair
/// gives an outcome that stopped.
#[bon::builder]
pub(crate) async fn run(
    declarations: &BTreeSet<Declaration>,
    context: &Context,
    mode: Mode,
) -> Outcome {
    let result: Result<_, Box<dyn Error + Send + Sync>> = match mode {
        Mode::Report => report()
            .declarations(declarations)
            .context(context)
            .call()
            .await
            .map(|findings| (findings, Vec::new()))
            .map_err(Into::into),
        Mode::Repair => repair()
            .declarations(declarations)
            .context(context)
            .call()
            .await
            .map_err(Into::into),
    };

    match result {
        Ok((findings, repairs)) => outcome().findings(findings).repairs(repairs).call(),
        Err(source) => Outcome::Errored { source },
    }
}

/// Checks the precondition on the tools before the run of an action, a
/// command that the harness wrote, or a group
///
/// The check is the check of a run of `doctor` without `--fix`. It reads the
/// project and asks mise about it, so it needs no network, and it changes
/// nothing. Like that check, it stops with an error in a project that mise does not
/// trust, when the generated file holds a pin with options. A project whose tool pins have no problem lets the run start.
///
/// # Errors
///
/// Returns [`CheckPreconditionError::UnmetPrecondition`] with the report of
/// `doctor` when the tool pins have a problem, and
/// [`CheckPreconditionError::UncheckedToolPins`] when the check gets no
/// answer.
#[bon::builder]
pub(crate) async fn require(
    declarations: &BTreeSet<Declaration>,
    context: &Context,
) -> Result<(), CheckPreconditionError> {
    let findings = report()
        .declarations(declarations)
        .context(context)
        .call()
        .await
        .map_err(|source| CheckPreconditionError::UncheckedToolPins { source })?;

    if findings.is_empty() {
        return Ok(());
    }

    Err(CheckPreconditionError::UnmetPrecondition {
        report: Report::builder()
            .action(name())
            .outcome(Outcome::Failed {
                findings,
                repairs: Vec::new(),
            })
            .build(),
    })
}

/// Writes the generated file of the tools that the declarations give, in the
/// project at the root
///
/// A test of the command line uses it to prepare a project whose tool pins
/// are current. The lock of `doctor --fix` needs the network, so the test
/// does not use the repair.
///
/// # Errors
///
/// Returns [`RepairToolPinsError::UnwritableFile`] when the file system
/// refuses the write.
#[cfg(test)]
#[bon::builder]
pub(crate) fn write_pins(
    declarations: &BTreeSet<Declaration>,
    root: &std::path::Path,
) -> Result<(), RepairToolPinsError> {
    self::generated::write()
        .pins(&pins(declarations))
        .root(&rakko::ProjectRoot::from(root))
        .call()
}

/// Returns every problem of the tool pins of the project
///
/// # Errors
///
/// Returns the errors that [`CheckToolPinsError`] describes.
#[bon::builder]
async fn report(
    declarations: &BTreeSet<Declaration>,
    context: &Context,
) -> Result<Vec<Finding>, CheckToolPinsError> {
    let pins = pins(declarations);

    let mut findings: Vec<Finding> = self::generated::check()
        .pins(&pins)
        .root(context.root())
        .call()?
        .into_iter()
        .collect();

    findings.extend(
        self::shadow::find()
            .pins(&pins)
            .context(context)
            .call()
            .await?
            .iter()
            .map(self::shadow::Shadow::finding),
    );
    findings.extend(
        self::toolchain::check()
            .declarations(declarations)
            .context(context)
            .call()
            .await?,
    );

    Ok(findings)
}

/// Repairs the tool pins of the project, and returns the problems that remain
/// and the problems that the repair fixed
///
/// # Errors
///
/// Returns the errors that [`RepairToolPinsError`] describes.
#[bon::builder]
async fn repair(
    declarations: &BTreeSet<Declaration>,
    context: &Context,
) -> Result<(Vec<Finding>, Vec<Finding>), RepairToolPinsError> {
    let root = context.root();
    let pins = pins(declarations);

    let mut findings = Vec::new();
    let mut repairs = Vec::new();

    let mut locks = !pins.is_empty();

    if let Some(problem) = self::generated::check().pins(&pins).root(root).call()? {
        self::generated::write().pins(&pins).root(root).call()?;

        if pins.is_empty() {
            locks = self::generated::retire_lockfile(root)?;
        }

        repairs.push(problem);
    }

    for shadow in self::shadow::find()
        .pins(&pins)
        .context(context)
        .call()
        .await?
    {
        if self::shadow::remove().shadow(&shadow).root(root).call()? {
            repairs.push(shadow.finding());
        } else {
            findings.push(shadow.finding());
        }
    }

    if locks {
        lock(context).await?;
    }

    findings.extend(
        self::toolchain::check()
            .declarations(declarations)
            .context(context)
            .call()
            .await?,
    );

    Ok((findings, repairs))
}

/// Locks the tools of the project with mise
///
/// Mise writes the lockfile of the generated file, and it refreshes the
/// lockfile of each other configuration file of the project.
///
/// # Errors
///
/// Returns [`RepairToolPinsError::MiseUnavailable`] when mise does not run,
/// and [`RepairToolPinsError::UnlockedTools`] when it ends without success,
/// for example without the network.
async fn lock(context: &Context) -> Result<(), RepairToolPinsError> {
    let execution = Invocation::new(MISE)
        .arg(LOCK)
        .in_directory(context.root().get())
        .run()
        .await
        .map_err(|source| RepairToolPinsError::MiseUnavailable { source })?;

    if !execution.status().success() {
        return Err(RepairToolPinsError::UnlockedTools {
            details: diagnosis(&execution),
        });
    }

    Ok(())
}

/// Returns the pins of the tools that the declarations give
fn pins(declarations: &BTreeSet<Declaration>) -> Pins {
    Pins::of(declarations.iter().map(Declaration::requirement))
}

/// Returns the outcome of the problems that remain and of the repairs
///
/// A run without a problem passed. A run that repaired every problem changed
/// the project, and a run with a problem that remains failed, also when it
/// repaired others.
#[bon::builder]
fn outcome(findings: Vec<Finding>, repairs: Vec<Finding>) -> Outcome {
    match (findings.is_empty(), repairs.is_empty()) {
        (true, true) => Outcome::Passed { summary: None },
        (true, false) => Outcome::Changed { repairs },
        (false, _) => Outcome::Failed { findings, repairs },
    }
}

/// Returns what mise wrote to its standard error stream, or a sentence that
/// says that it wrote nothing
fn diagnosis(execution: &Execution) -> String {
    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rakko::{FilePath, Location, RequiredVersion, ToolRequirement, ToolVersion, VersionRange};
    use rakko_tool::MiseFile;
    use tempfile::TempDir;

    use super::*;

    /// The toolchain that the tests require of a project
    ///
    /// The test needs a tool that this repository installs, so that the
    /// resolution of the toolchain reaches the comparison with the range.
    const TOOLCHAIN: &str = "jq";

    /// The tool that the tests declare at an exact version
    const TOOL: &str = "taplo";

    /// The lockfile of the generated file, relative to the root
    const LOCKFILE: &str = ".config/mise/mise.lock";

    /// Returns the root of this repository
    fn repository() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository")
            .to_path_buf()
    }

    /// Returns the version that a mise file of this repository pins for a
    /// tool
    #[bon::builder]
    fn pinned_in(file: &str, tool: &str) -> String {
        let pins = std::fs::read_to_string(repository().join(file))
            .expect("the test reads a mise file of the repository");

        MiseFile::new(&pins).pinned_version(tool).to_owned()
    }

    /// Returns the declaration of the tool at the version that the crate of
    /// the tool pins
    fn tool() -> Declaration {
        let version = pinned_in()
            .file("crates/rakko-taplo/mise.rakko.toml")
            .tool(TOOL)
            .call();

        Declaration::builder()
            .action("format-toml".parse().expect("the test names an action"))
            .requirement(
                ToolRequirement::builder()
                    .tool(TOOL)
                    .executable(TOOL)
                    .version(RequiredVersion::Exact(ToolVersion::new(version)))
                    .build(),
            )
            .build()
    }

    /// Returns the declaration of the toolchain in a range
    fn toolchain(range: &str) -> Declaration {
        Declaration::builder()
            .action("format-json".parse().expect("the test names an action"))
            .requirement(
                ToolRequirement::builder()
                    .tool(TOOLCHAIN)
                    .executable(TOOLCHAIN)
                    .version(RequiredVersion::Range(VersionRange::new(range)))
                    .build(),
            )
            .build()
    }

    /// Returns the declarations of the tool and of a toolchain that no
    /// version accepts
    fn declarations() -> BTreeSet<Declaration> {
        BTreeSet::from([tool(), toolchain("<1")])
    }

    /// A project in a temporary directory with a configuration file of mise
    struct Project {
        /// The directory that holds the project
        directory: TempDir,
    }

    impl Project {
        /// Creates a project whose `mise.toml` holds the text
        ///
        /// Mise ignores a configuration that nobody trusts, so the file is
        /// trusted right away.
        fn with(text: &str) -> Self {
            let directory = TempDir::new().expect("the test creates a temporary directory");
            let pins = directory.path().join("mise.toml");
            std::fs::write(&pins, text).expect("the test writes the mise.toml of the project");

            let trusted = Command::new(MISE)
                .arg("trust")
                .arg("--quiet")
                .arg(&pins)
                .status()
                .expect("the test runs mise");
            assert!(trusted.success(), "expected mise to trust the project");

            Self { directory }
        }

        /// Creates a project that pins the toolchain at the version of this
        /// repository, and the tool at a version that no action declares
        fn shadowing() -> Self {
            let toolchain = pinned_in().file("mise.toml").tool(TOOLCHAIN).call();

            Self::with(&format!(
                "[tools]\n{TOOLCHAIN} = \"{toolchain}\"\n{TOOL} = \"0.9.0\"\n"
            ))
        }

        /// Returns the context of a run in the project
        fn context(&self) -> Context {
            Context::builder().root(self.directory.path()).build()
        }

        /// Writes a generated file that pins the tool, and the lockfile beside it
        ///
        /// A project holds both after it unmounted the last action that
        /// declares a tool.
        fn left_over(&self, lockfile: &str) {
            generated::write()
                .pins(&pins(&BTreeSet::from([tool()])))
                .root(self.context().root())
                .call()
                .expect("the test writes the generated file");
            std::fs::write(self.directory.path().join(LOCKFILE), lockfile)
                .expect("the test writes the lockfile");
        }

        /// Returns whether the project holds the file
        fn holds(&self, path: &str) -> bool {
            self.directory.path().join(path).exists()
        }
    }

    impl Drop for Project {
        /// Withdraws the trust that the project received
        ///
        /// A failure stays quiet, because the test already reported what it
        /// was about.
        fn drop(&mut self) {
            let _ = Command::new(MISE)
                .arg("trust")
                .arg("--quiet")
                .arg("--untrust")
                .arg(self.directory.path().join("mise.toml"))
                .status();
        }
    }

    /// Returns the location of each finding of a failed outcome, and of each
    /// repair
    fn locations(outcome: &Outcome) -> (Vec<Location>, Vec<Location>) {
        let of = |findings: &[Finding]| findings.iter().map(|f| f.location().clone()).collect();

        match outcome {
            Outcome::Failed { findings, repairs } => (of(findings), of(repairs)),
            Outcome::Changed { repairs } => (Vec::new(), of(repairs)),
            other => panic!("expected findings or repairs, got {other:?}"),
        }
    }

    /// Returns the location of a file of the project
    fn file(path: &str) -> Location {
        Location::File {
            path: FilePath::try_from(path).expect("the test names a relative file"),
        }
    }

    /// Returns a finding at the location
    fn finding_at(location: Location) -> Finding {
        Finding::builder()
            .message("the test finds a problem")
            .location(location)
            .build()
    }

    #[test]
    fn mode_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Mode>();
    }

    #[test]
    fn mode_with_the_flag_asks_for_the_repair() {
        let matches = command().get_matches_from([DOCTOR, "--fix"]);

        assert_eq!(mode(&matches), Mode::Repair);
    }

    #[test]
    fn mode_without_the_flag_asks_for_the_report() {
        let matches = command().get_matches_from([DOCTOR]);

        assert_eq!(mode(&matches), Mode::Report);
    }

    #[test]
    fn outcome_with_findings_and_repairs_fails() {
        let outcome = outcome()
            .findings(vec![finding_at(Location::Project)])
            .repairs(vec![finding_at(file("mise.toml"))])
            .call();

        assert!(matches!(outcome, Outcome::Failed { .. }), "got {outcome:?}");
    }

    #[test]
    fn outcome_with_nothing_passes() {
        let outcome = outcome().findings(Vec::new()).repairs(Vec::new()).call();

        assert!(matches!(outcome, Outcome::Passed { .. }), "got {outcome:?}");
    }

    #[test]
    fn outcome_with_only_repairs_changed_the_project() {
        let outcome = outcome()
            .findings(Vec::new())
            .repairs(vec![finding_at(file("mise.toml"))])
            .call();

        assert!(
            matches!(outcome, Outcome::Changed { .. }),
            "got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn require_with_a_configuration_that_mise_refuses_stops_the_run() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::write(
            directory.path().join("mise.toml"),
            "min_version = \"9999.0.0\"\n",
        )
        .expect("the test writes the mise.toml of the project");
        let context = Context::builder().root(directory.path()).build();

        let result = require()
            .declarations(&BTreeSet::from([tool()]))
            .context(&context)
            .call()
            .await;

        assert!(
            matches!(
                result,
                Err(CheckPreconditionError::UncheckedToolPins { .. })
            ),
            "got {result:?}"
        );
    }

    #[tokio::test]
    async fn require_with_a_current_file_lets_the_run_start() {
        let project = Project::with("[tools]\n");
        let declarations = BTreeSet::from([tool()]);
        generated::write()
            .pins(&pins(&declarations))
            .root(project.context().root())
            .call()
            .expect("the test writes the generated file");

        let result = require()
            .declarations(&declarations)
            .context(&project.context())
            .call()
            .await;

        assert!(result.is_ok(), "got {result:?}");
    }

    #[tokio::test]
    async fn require_with_a_stale_file_gives_the_problem_and_the_repair() {
        let project = Project::with("[tools]\n");
        std::fs::create_dir_all(project.directory.path().join(".config/mise/conf.d"))
            .expect("the test creates the directory of the generated file");
        std::fs::write(
            project.directory.path().join(generated::PATH),
            "[tools]\ntaplo = \"0.9.0\"\n",
        )
        .expect("the test writes a stale generated file");

        let Err(error) = require()
            .declarations(&BTreeSet::from([tool()]))
            .context(&project.context())
            .call()
            .await
        else {
            panic!("expected the stale file to stop the run");
        };

        let message = error.to_string();
        assert!(
            message.contains("mise run rakko -- doctor --fix")
                && message.contains(&format!("{}: ", generated::PATH)),
            "got {message}"
        );
    }

    #[tokio::test]
    async fn require_without_a_declared_tool_lets_the_run_start() {
        let project = Project::with("[tools]\n");

        let result = require()
            .declarations(&BTreeSet::new())
            .context(&project.context())
            .call()
            .await;

        assert!(result.is_ok(), "got {result:?}");
    }

    #[tokio::test]
    async fn run_in_repair_mode_keeps_the_pin_of_the_toolchain() {
        let project = Project::shadowing();
        let toolchain = pinned_in().file("mise.toml").tool(TOOLCHAIN).call();

        let _outcome = run()
            .declarations(&declarations())
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        assert_eq!(
            std::fs::read_to_string(project.directory.path().join("mise.toml"))
                .expect("the test reads the mise.toml of the project"),
            format!("[tools]\n{TOOLCHAIN} = \"{toolchain}\"\n")
        );
    }

    #[tokio::test]
    async fn run_in_repair_mode_leaves_a_pin_that_mise_ranks_below_the_file() {
        let project = Project::with("[tools]\n");
        let versions = project.directory.path().join(".tool-versions");
        std::fs::write(&versions, format!("{TOOL} 0.9.0\n")).expect("the test writes the file");

        let outcome = run()
            .declarations(&BTreeSet::from([tool()]))
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        let kept = std::fs::read_to_string(&versions).expect("the test reads the file");
        assert_eq!(
            (locations(&outcome), kept),
            (
                (Vec::new(), vec![file(generated::PATH)]),
                format!("{TOOL} 0.9.0\n")
            )
        );
    }

    #[tokio::test]
    async fn run_in_repair_mode_repairs_the_file_and_the_pin_and_keeps_the_toolchain() {
        let project = Project::shadowing();

        let outcome = run()
            .declarations(&declarations())
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        assert_eq!(
            locations(&outcome),
            (
                vec![Location::Project],
                vec![file(generated::PATH), file("mise.toml")]
            )
        );
    }

    #[tokio::test]
    async fn run_in_repair_mode_reports_a_lock_that_mise_refuses() {
        let project = Project::with("[settings]\nlockfile_platforms = [\"bogus\"]\n");

        let outcome = run()
            .declarations(&BTreeSet::from([tool()]))
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        assert!(
            matches!(&outcome, Outcome::Errored { source } if source.to_string().contains("did not lock")),
            "expected a lock that mise refused, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn run_in_repair_mode_with_a_left_over_file_beside_a_configuration_prunes_the_lockfile() {
        let project = Project::with("[tools]\n");
        let toolchain = pinned_in().file("mise.toml").tool(TOOLCHAIN).call();
        project.left_over(&format!(
            "lockfile_version = 3\n\n[[tools.{TOOL}]]\nversion = \"0.10.0\"\nbackend = \"aqua:tamasfe/taplo\"\n"
        ));
        std::fs::write(
            project.directory.path().join(".config/mise/config.toml"),
            format!("[tools]\n{TOOLCHAIN} = \"{toolchain}\"\n"),
        )
        .expect("the test writes the configuration");

        let _outcome = run()
            .declarations(&BTreeSet::new())
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        let lockfile = std::fs::read_to_string(project.directory.path().join(LOCKFILE))
            .expect("the test reads the lockfile");
        assert!(!lockfile.contains(TOOL), "got {lockfile}");
    }

    #[tokio::test]
    async fn run_in_repair_mode_with_a_left_over_file_removes_it_and_its_lockfile() {
        let project = Project::with("[tools]\n");
        project.left_over("lockfile_version = 3\n");

        let outcome = run()
            .declarations(&BTreeSet::new())
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        assert_eq!(
            (
                locations(&outcome),
                project.holds(generated::PATH),
                project.holds(LOCKFILE)
            ),
            ((Vec::new(), vec![file(generated::PATH)]), false, false)
        );
    }

    #[tokio::test]
    async fn run_in_repair_mode_without_a_declared_tool_locks_nothing() {
        let toolchain = pinned_in().file("mise.toml").tool(TOOLCHAIN).call();
        let project = Project::with(&format!("[tools]\n{TOOLCHAIN} = \"{toolchain}\"\n"));

        let _outcome = run()
            .declarations(&BTreeSet::new())
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        assert!(!project.holds("mise.lock"));
    }

    #[tokio::test]
    async fn run_in_report_mode_after_the_repair_reports_only_the_toolchain() {
        let project = Project::shadowing();
        let _repaired = run()
            .declarations(&declarations())
            .context(&project.context())
            .mode(Mode::Repair)
            .call()
            .await;

        let outcome = run()
            .declarations(&declarations())
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        assert_eq!(locations(&outcome), (vec![Location::Project], Vec::new()));
    }

    #[tokio::test]
    async fn run_in_report_mode_changes_nothing() {
        let project = Project::shadowing();

        let pins = std::fs::read_to_string(project.directory.path().join("mise.toml"))
            .expect("the test reads the mise.toml of the project");

        let _outcome = run()
            .declarations(&declarations())
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        let after = std::fs::read_to_string(project.directory.path().join("mise.toml"))
            .expect("the test reads the mise.toml of the project");
        assert_eq!((project.holds(generated::PATH), after), (false, pins));
    }

    #[tokio::test]
    async fn run_in_report_mode_leaves_out_a_pin_that_mise_ranks_below_the_file() {
        let project = Project::with("[tools]\n");
        let declarations = BTreeSet::from([tool()]);
        generated::write()
            .pins(&pins(&declarations))
            .root(project.context().root())
            .call()
            .expect("the test writes the generated file");
        std::fs::write(
            project.directory.path().join(".tool-versions"),
            format!("{TOOL} 0.9.0\n"),
        )
        .expect("the test writes the file");

        let outcome = run()
            .declarations(&declarations)
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        assert!(matches!(outcome, Outcome::Passed { .. }), "got {outcome:?}");
    }

    #[tokio::test]
    async fn run_in_report_mode_reports_the_file_the_pin_and_the_toolchain() {
        let project = Project::shadowing();

        let outcome = run()
            .declarations(&declarations())
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        assert_eq!(
            locations(&outcome),
            (
                vec![file(generated::PATH), file("mise.toml"), Location::Project],
                Vec::new()
            )
        );
    }

    #[tokio::test]
    async fn run_with_a_configuration_that_mise_refuses_stops() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::write(
            directory.path().join("mise.toml"),
            "min_version = \"9999.0.0\"\n",
        )
        .expect("the test writes the mise.toml of the project");
        let context = Context::builder().root(directory.path()).build();

        let outcome = run()
            .declarations(&BTreeSet::from([tool()]))
            .context(&context)
            .mode(Mode::Report)
            .call()
            .await;

        assert!(
            matches!(&outcome, Outcome::Errored { source } if source.to_string().contains("listed no configuration")),
            "expected a configuration that mise refused, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn run_with_a_toolchain_in_its_range_passes() {
        let project = Project::shadowing();

        let outcome = run()
            .declarations(&BTreeSet::from([toolchain(">=1")]))
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        assert!(matches!(outcome, Outcome::Passed { .. }), "got {outcome:?}");
    }

    #[tokio::test]
    async fn run_with_an_unreadable_generated_file_stops() {
        let project = Project::with("[tools]\n");
        std::fs::create_dir_all(project.directory.path().join(generated::PATH))
            .expect("the test creates a directory in place of the file");

        let outcome = run()
            .declarations(&BTreeSet::from([tool()]))
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        assert!(
            matches!(outcome, Outcome::Errored { .. }),
            "got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn run_without_a_declared_tool_asks_mise_nothing() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::write(
            directory.path().join("mise.toml"),
            "min_version = \"9999.0.0\"\n",
        )
        .expect("the test writes the mise.toml of the project");
        let context = Context::builder().root(directory.path()).build();

        let outcome = run()
            .declarations(&BTreeSet::new())
            .context(&context)
            .mode(Mode::Report)
            .call()
            .await;

        assert!(matches!(outcome, Outcome::Passed { .. }), "got {outcome:?}");
    }

    #[tokio::test]
    async fn run_without_a_declared_tool_passes() {
        let project = Project::with("[tools]\n");

        let outcome = run()
            .declarations(&BTreeSet::new())
            .context(&project.context())
            .mode(Mode::Report)
            .call()
            .await;

        assert!(matches!(outcome, Outcome::Passed { .. }), "got {outcome:?}");
    }
}
