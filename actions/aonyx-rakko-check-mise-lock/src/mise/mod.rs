//! The mise that a run asks to lock a project
//!
//! The tool of the action is mise itself. This module starts it in one
//! directory, the project or a copy of it. It asks two questions: which
//! lockfiles the directory keeps, and what `mise lock` does there. It turns
//! what mise wrote into data. It stops on a warning that is not one of the
//! harmless kinds, because such a warning can mean an unresolved tool.

mod lock;
mod trust;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rakko::{FilePath, ProjectRoot, SkipReason, ToolOutput};
use rakko_tool::{Execution, Invocation, Launch, ShownStreams};
use serde::Deserialize;

pub(crate) use self::lock::Lock;
pub(crate) use self::trust::Trust;
use crate::check_mise_lock::CheckMiseLockError;
use crate::lockfile::Lockfile;
use crate::part::Platform;

/// The program of the action
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches the action reaches mise as well.
const MISE: &str = "mise";

/// The subcommand of mise that writes the lockfiles of a project
const LOCK: &str = "lock";

/// The option of `mise lock` that lists the lockfiles instead of writing them
///
/// With it, mise names every lockfile of the root of the project that exists,
/// and it resolves, installs, and writes nothing.
const SIDECARS: &str = "--sidecars";

/// The option that asks mise for the list as JSON
const JSON: &str = "--json";

/// The settings of mise that make it show its warnings, as variables of the
/// environment, each with its value
///
/// The run reads a warning of mise to learn that it failed to resolve a tool,
/// so the run of `mise lock` sets these. A variable of the environment beats
/// the same setting in a configuration. So the warnings show whatever the
/// environment, the project, or the global configuration of the contributor
/// asks for. Each setting selects what mise shows, not what it locks. Each of
/// them silences mise on its own, so the run sets all three.
const VERBOSITY: [(&str, &str); 3] = [
    ("MISE_QUIET", "false"),
    ("MISE_SILENT", "false"),
    ("MISE_LOG_LEVEL", "warn"),
];

/// The variable with which the environment answers every question of mise
/// with yes
///
/// Mise asks whether to trust a configuration that nobody trusted, and with
/// this variable it trusts the configuration and records the trust for good.
/// Every run of mise takes it out of its environment. This way the action never
/// trusts a project in the place of whoever owns it, and a project that nobody
/// trusted stops the run.
const YES: &str = "MISE_YES";

/// The variables with which an environment says that it is a job of CI
///
/// Mise trusts a configuration that nobody trusted, without a question and
/// without a record, when either of them is set. Every run of mise takes them
/// out of its environment. This way a project that nobody trusted stops the run on
/// a job of CI as well. A project that a job checks stays trusted through the
/// directories that `MISE_TRUSTED_CONFIG_PATHS` names.
const CI: [&str; 2] = ["CI", "GITHUB_ACTIONS"];

/// The variable that names the directories that mise trusts for one run
const TRUSTED_CONFIG_PATHS: &str = "MISE_TRUSTED_CONFIG_PATHS";

/// The words with which mise starts the line that names the platforms that it
/// targets for a lockfile
const TARGETING: &str = "\u{2192} Targeting ";

/// The words that stand between the count of the platforms and the path of
/// the lockfile in that line
const TARGETING_FOR: &str = " for ";

/// The words that stand between the path of the lockfile and its platforms in
/// that line
const TARGETING_PLATFORMS: &str = ": ";

/// The mark that mise writes in place of the home directory at the start of a
/// path under it
const HOME: &str = "~";

/// The words with which mise reports a project that configures nothing to
/// lock
const UNCONFIGURED: &str = "No tools configured to lock";

/// The words with which mise reports a failure of its own
///
/// A warning with these words is never harmless, whatever else it says. One
/// example is the name of a deprecated backend in the chain of its causes.
const FAILED_TO: &str = "failed to ";

/// The mark with which mise starts a warning of its log
const WARNING: &str = "mise WARN";

/// The words of the warnings of mise that say nothing about what it locked
///
/// Mise warns that a lockfile keeps an older format, that the registry moved
/// a tool to another backend, and that a backend is deprecated. It also warns
/// that a request failed and is tried again or elsewhere, and that a newer
/// mise exists. None of these leaves an entry unresolved. Every other warning
/// of a run that ended with success can be the only sign that mise could not
/// resolve a tool. Examples are one that it failed to resolve and one that it
/// failed to fetch what it needs for. Another is a limit of the rate of an API.
const HARMLESS: [&str; 7] = [
    "uses lockfile format version",
    "is locked to ",
    "deprecated",
    "(transient)",
    "fallback=true",
    "mise version ",
    "self-update",
];

/// The marks with which mise starts a record of its output
///
/// A record of the log starts with the name of the program, and a line of
/// progress starts with an arrow or a check mark. A line without one of these
/// marks goes on from the record above it.
const RECORD_MARKS: [&str; 3] = ["mise ", "\u{2192}", "\u{2713}"];

/// The character that starts a sequence of the terminal, such as one that
/// sets a color
const ESCAPE: char = '\u{1b}';

/// The mark with which mise starts a line that warns
const WARNING_MARK: char = '!';

/// The details of a run of mise that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

/// One lockfile in the list that mise writes
///
/// Mise writes more fields for each lockfile, such as the dependency graphs
/// that it keeps next to it. The action compares the lockfile and reads only
/// its path.
#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
struct Listed {
    /// The path of the lockfile, relative to the directory of the run when it
    /// lies inside it
    lockfile: PathBuf,
}

/// The mise that locks one directory
///
/// The value holds the directory that every run of mise starts in, and who
/// trusts its configuration. An action creates one for the project, and one
/// for the copy of the project when it locks there.
#[derive(Clone, Eq, PartialEq, Debug, bon::Builder)]
pub(crate) struct Mise {
    /// The directory that every run of mise starts in
    root: ProjectRoot,

    /// Who trusts the configuration of the directory
    ///
    /// With [`Trust::Granted`], every run of mise trusts the directory, and
    /// mise then reads a configuration there that nobody trusted. A caller
    /// grants it only for a directory whose configuration somebody trusted in
    /// another place. An example is a copy of a project that mise already read.
    trust: Trust,

    /// The tool output that each run of mise sends what it writes to
    ///
    /// A value without one discards what mise writes.
    #[builder(default)]
    output: ToolOutput,
}

impl Mise {
    /// Runs `mise lock` in the directory
    ///
    /// The run passes no option, so the configuration of the project decides
    /// which tools and which platforms mise locks. The run writes the
    /// lockfiles of the directory.
    ///
    /// The answer names each lockfile relative to the directory, also one that
    /// mise names relative to the home directory. On a Unix-like system, the
    /// run reads the home directory as mise reads it. It reads from `HOME`, or
    /// from the account of the user when `HOME` is not set. On other systems,
    /// mise names no path relative to the home directory.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not start, and
    /// [`UnlockedProject`][unlocked] with what mise wrote when it ends without
    /// success. Returns [`UnexpectedWarnings`][unresolved] with each warning
    /// when mise ends with success and gives a warning that is not one of the
    /// harmless kinds. The reason is that such a warning can mean that mise
    /// left the entry of a platform as it was. A gap in it would then pass
    /// unseen. Returns
    /// [`ForeignLockfile`][foreign] when mise names a lockfile outside the
    /// directory.
    ///
    /// [foreign]: CheckMiseLockError::ForeignLockfile
    /// [unavailable]: CheckMiseLockError::MiseUnavailable
    /// [unlocked]: CheckMiseLockError::UnlockedProject
    /// [unresolved]: CheckMiseLockError::UnexpectedWarnings
    pub(crate) async fn lock(&self) -> Result<Lock, CheckMiseLockError> {
        let execution = self
            .lock_invocation()
            .launch_with(&self.output, ShownStreams::Both)
            .await
            .map_err(|source| CheckMiseLockError::MiseUnavailable { source })?;

        if !execution.status().success() {
            return Err(CheckMiseLockError::UnlockedProject {
                details: details(&execution),
            });
        }

        let output = plain(&format!(
            "{}\n{}",
            execution.stdout().to_string_lossy(),
            execution.stderr().to_string_lossy()
        ));

        if let Some(details) = unresolved(&output) {
            return Err(CheckMiseLockError::UnexpectedWarnings { details });
        }

        if let Some(reason) = unconfigured(&execution) {
            return Ok(Lock::Unconfigured { reason });
        }

        Ok(Lock::Written {
            targets: targets(&output, &self.root, std::env::home_dir().as_deref())?,
        })
    }

    /// Returns each lockfile of the directory, read
    ///
    /// Mise names the lockfiles that the root of the directory keeps, and the
    /// action reads each of them. The answer maps the path of a lockfile,
    /// relative to the directory, to its entries. A lockfile that does not
    /// exist is not in the answer.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`list`][list], and
    /// [`InaccessibleLockfile`][inaccessible] and
    /// [`UnreadableLockfile`][unreadable] when a lockfile cannot be read.
    ///
    /// [inaccessible]: CheckMiseLockError::InaccessibleLockfile
    /// [list]: Mise::list
    /// [unreadable]: CheckMiseLockError::UnreadableLockfile
    pub(crate) async fn lockfiles(
        &self,
    ) -> Result<BTreeMap<FilePath, Lockfile>, CheckMiseLockError> {
        let mut lockfiles = BTreeMap::new();

        for path in self.list().await? {
            let lockfile = self.read(&path).await?;
            lockfiles.insert(path, lockfile);
        }

        Ok(lockfiles)
    }

    /// Returns the paths of the lockfiles of the directory, without reading
    /// them
    ///
    /// Mise names the lockfiles that the root of the directory keeps, relative
    /// to the directory. Mise refuses the question for a configuration that
    /// needs trust and that nobody trusted. So an answer also says that mise
    /// reads the configuration of the directory.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not start.
    /// Returns [`UnlistedLockfiles`][unlisted] with what mise wrote when it
    /// ends without success, which includes a configuration that needs trust
    /// and that nobody trusted. Returns [`UnreadableListing`][listing] when the
    /// action cannot read the list, and [`ForeignLockfile`][foreign] when mise
    /// names a lockfile outside the directory.
    ///
    /// [foreign]: CheckMiseLockError::ForeignLockfile
    /// [listing]: CheckMiseLockError::UnreadableListing
    /// [unavailable]: CheckMiseLockError::MiseUnavailable
    /// [unlisted]: CheckMiseLockError::UnlistedLockfiles
    pub(crate) async fn list(&self) -> Result<Vec<FilePath>, CheckMiseLockError> {
        let execution = self
            .invocation()
            .arg(LOCK)
            .arg(SIDECARS)
            .arg(JSON)
            .launch_with(&self.output, ShownStreams::StandardError)
            .await
            .map_err(|source| CheckMiseLockError::MiseUnavailable { source })?;

        if !execution.status().success() {
            return Err(CheckMiseLockError::UnlistedLockfiles {
                details: details(&execution),
            });
        }

        listed(&execution.stdout().to_string_lossy(), &self.root)
    }

    /// Returns the command that starts mise in the directory
    ///
    /// A directory that the run trusts gets the variable that tells mise so,
    /// for this one run of mise. The command takes the variable that answers
    /// the questions of mise out of its environment. Mise then asks no question
    /// that the environment answers in the place of the contributor. The command
    /// also removes the variables that tell mise it runs on a job of CI, which
    /// allow it to trust every configuration.
    fn invocation(&self) -> Invocation {
        let invocation = CI.into_iter().fold(
            Invocation::new(MISE)
                .in_directory(self.root.get())
                .env_remove(YES),
            Invocation::env_remove,
        );

        match self.trust {
            Trust::Inherited => invocation,
            Trust::Granted => invocation.env(TRUSTED_CONFIG_PATHS, self.root.get()),
        }
    }

    /// Returns the command that runs `mise lock` in the directory
    ///
    /// The command sets the settings that make mise show its warnings in its
    /// environment. The reason is that the run reads a warning to learn that mise failed to
    /// resolve a tool.
    fn lock_invocation(&self) -> Invocation {
        VERBOSITY
            .into_iter()
            .fold(self.invocation().arg(LOCK), |invocation, (name, value)| {
                invocation.env(name, value)
            })
    }

    /// Returns the entries of one lockfile of the directory
    ///
    /// # Errors
    ///
    /// Returns [`InaccessibleLockfile`][inaccessible] when the file cannot be
    /// read, and [`UnreadableLockfile`][unreadable] when the action cannot
    /// read its entries.
    ///
    /// [inaccessible]: CheckMiseLockError::InaccessibleLockfile
    /// [unreadable]: CheckMiseLockError::UnreadableLockfile
    async fn read(&self, path: &FilePath) -> Result<Lockfile, CheckMiseLockError> {
        let text = tokio::fs::read_to_string(self.root.get().join(path.get()))
            .await
            .map_err(|source| CheckMiseLockError::InaccessibleLockfile {
                path: path.get().to_path_buf(),
                source,
            })?;

        Lockfile::parse(&text).map_err(|source| CheckMiseLockError::UnreadableLockfile {
            path: path.get().to_path_buf(),
            source,
        })
    }
}

/// Returns what mise wrote about a run that ended without success
///
/// Mise writes its diagnosis to the standard error stream. So whoever reads the
/// error reads the answer of mise instead of a sentence that Rakko wrote
/// about it.
fn details(execution: &Execution) -> String {
    let diagnosis = plain(&execution.stderr().to_string_lossy());
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Returns the text without the sequences that a terminal reads as colors
///
/// Mise colors its output when the environment asks it to. The codes of a
/// color would otherwise reach the reason of a skip and the words of an
/// error. A sequence starts with the escape character and a bracket, and it
/// ends with its first letter.
fn plain(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();

    while let Some(character) = characters.next() {
        if character == ESCAPE && characters.peek() == Some(&'[') {
            for code in characters.by_ref() {
                if code.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }

    output
}

/// Returns the platforms that mise targeted for each lockfile, by the path of
/// the lockfile
///
/// Mise writes one line for each lockfile that it locks, such as
/// `→ Targeting 2 platform(s) for /srv/project/mise.lock: linux-x64,
/// macos-arm64`. A line in another shape names nothing. For a lockfile under
/// the home directory, mise writes `~` in place of the home directory, such
/// as `~/project/mise.lock`. The given home directory replaces the `~` again.
/// It is the home directory that mise read, because mise inherits the
/// environment of the action.
///
/// # Errors
///
/// Returns [`ForeignLockfile`][foreign] when a line names a lockfile outside
/// the directory. Mise writes such a lockfile when the configuration of the
/// tools lies in a directory above. A run that went on would compare no
/// lockfile of the project and could write outside it. A line can name a
/// lockfile under the home directory when no home directory is given. That
/// line gives the same error, because nothing places its lockfile inside the
/// directory.
///
/// [foreign]: CheckMiseLockError::ForeignLockfile
fn targets(
    output: &str,
    root: &ProjectRoot,
    home: Option<&Path>,
) -> Result<BTreeMap<FilePath, BTreeSet<Platform>>, CheckMiseLockError> {
    let mut targets = BTreeMap::new();

    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix(TARGETING) else {
            continue;
        };
        let Some((_, rest)) = rest.split_once(TARGETING_FOR) else {
            continue;
        };
        let Some((lockfile, platforms)) = rest.rsplit_once(TARGETING_PLATFORMS) else {
            continue;
        };

        let path = named(lockfile, home)
            .and_then(|named| FilePath::within().path(&named).root(root).call())
            .ok_or_else(|| CheckMiseLockError::ForeignLockfile {
                path: PathBuf::from(lockfile),
            })?;
        let platforms = platforms
            .split(',')
            .map(str::trim)
            .filter(|platform| !platform.is_empty())
            .map(Platform::new)
            .collect();

        targets.insert(path, platforms);
    }

    Ok(targets)
}

/// Returns the path that mise names with the given words, if it can be known
///
/// On a Unix-like system, mise writes `~` in place of the home directory at
/// the start of a path under it. The answer puts the home directory back in
/// place of the `~`. It joins the two as text and not as paths. Thus it
/// undoes exactly what mise did, also for a home directory that ends with a
/// separator. Without a home directory, a path that starts with `~` cannot be
/// known, and the answer is nothing.
fn named(words: &str, home: Option<&Path>) -> Option<PathBuf> {
    let Some(rest) = words.strip_prefix(HOME) else {
        return Some(PathBuf::from(words));
    };

    let mut path = home?.as_os_str().to_owned();
    path.push(rest);

    Some(PathBuf::from(path))
}

/// Returns the paths of the lockfiles that a list of mise names
///
/// A path is relative to the directory of the run when the lockfile lies
/// inside it, and absolute otherwise.
///
/// # Errors
///
/// Returns [`UnreadableListing`][listing] when the list is not the JSON that
/// the action reads, and [`ForeignLockfile`][foreign] when a path does not
/// lie inside the directory.
///
/// [foreign]: CheckMiseLockError::ForeignLockfile
/// [listing]: CheckMiseLockError::UnreadableListing
fn listed(listing: &str, root: &ProjectRoot) -> Result<Vec<FilePath>, CheckMiseLockError> {
    let entries: Vec<Listed> =
        serde_json::from_str(listing).map_err(|source| CheckMiseLockError::UnreadableListing {
            listing: listing.to_owned(),
            source,
        })?;

    entries
        .into_iter()
        .map(|entry| {
            FilePath::within()
                .path(&entry.lockfile)
                .root(root)
                .call()
                .ok_or(CheckMiseLockError::ForeignLockfile {
                    path: entry.lockfile,
                })
        })
        .collect()
}

/// Returns the warnings of mise that can mean an unresolved tool, if it wrote
/// any
///
/// Mise can end with success when it could not resolve a tool, and a warning
/// is then the only sign of it. Each warning that is not one of the harmless
/// kinds therefore counts. A warning starts on a line with the mark of a
/// warning. It can go on over the lines that follow, which say how to resolve
/// it. The answer holds each such warning as a whole, as mise wrote it, and
/// once, also when mise gave it for several requests. A warning ends where
/// the next record of mise starts.
fn unresolved(output: &str) -> Option<String> {
    let mut warnings: Vec<Vec<&str>> = Vec::new();
    let mut inside = false;

    for line in output.lines() {
        if line.starts_with(WARNING) {
            inside = line.contains(FAILED_TO) || !HARMLESS.iter().any(|words| line.contains(words));

            if inside {
                warnings.push(Vec::new());
            }
        } else if RECORD_MARKS.iter().any(|mark| line.starts_with(mark)) {
            inside = false;
        }

        if let (true, Some(warning)) = (inside, warnings.last_mut()) {
            warning.push(line.trim());
        }
    }

    let mut distinct: Vec<String> = Vec::new();

    for warning in warnings {
        let text = warning.join("\n");

        if !distinct.contains(&text) {
            distinct.push(text);
        }
    }

    if distinct.is_empty() {
        None
    } else {
        Some(distinct.join("\n"))
    }
}

/// Returns the reason of a run that found nothing to lock, if it found
/// nothing
///
/// Mise writes a warning when the project configures no tool to lock, and it
/// ends with success. The reason is the line of that warning, without the
/// mark that starts it, as mise wrote it.
fn unconfigured(execution: &Execution) -> Option<SkipReason> {
    let stdout = plain(&execution.stdout().to_string_lossy());
    let stderr = plain(&execution.stderr().to_string_lossy());

    stdout
        .lines()
        .chain(stderr.lines())
        .find(|line| line.contains(UNCONFIGURED))
        .map(|line| SkipReason::new(line.trim().trim_start_matches(WARNING_MARK).trim_start()))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::MAIN_SEPARATOR;

    use rakko_test_utils::path;

    use super::*;

    /// The root that the paths of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    /// The home directory that holds the root of a test
    fn home() -> PathBuf {
        path("/home/otter")
    }

    /// Returns the line with which mise names the platform `linux-x64` for a
    /// lockfile, after the `~` that stands for the home directory
    fn targeting_from_home(rest: &str) -> String {
        format!("\u{2192} Targeting 1 platform(s) for ~{rest}: linux-x64\n")
    }

    /// Returns the targets that name the platform `linux-x64` for
    /// `mise.lock`
    fn targeted_lockfile() -> BTreeMap<FilePath, BTreeSet<Platform>> {
        BTreeMap::from([(
            "mise.lock".parse::<FilePath>().unwrap(),
            BTreeSet::from([Platform::new("linux-x64")]),
        )])
    }

    /// Returns the given relative path, after the separator of the platform
    fn separated(literal: &str) -> String {
        format!("{MAIN_SEPARATOR}{}", path(literal).display())
    }

    /// Returns the value of the variable that names the trusted directories,
    /// in the command of the given mise
    fn trusted(mise: &Mise) -> Option<std::ffi::OsString> {
        mise.invocation()
            .environment()
            .iter()
            .find(|variable| variable.name().get() == TRUSTED_CONFIG_PATHS)
            .map(|variable| variable.value().get().to_owned())
    }

    /// The configuration of a project that pins one tool that mise downloads
    ///
    /// Mise checks the provenance of every download when it locks, and it asks
    /// a service over the network for each platform. A busy service makes mise
    /// skip a platform, so the configuration turns the checks off. Mise then
    /// locks from what it knows of the tool.
    const PINNED: &str = "[tools]\njq = \"1.8.1\"\n\n[settings]\nlockfile = true\nlockfile_platforms = [\"linux-x64\", \"macos-arm64\"]\naqua.github_attestations = false\naqua.slsa = false\naqua.cosign = false\naqua.minisign = false\n";

    /// A project in a temporary directory whose lockfile `mise lock` wrote
    ///
    /// The project trusts its configuration until it drops, because mise
    /// ignores a configuration that nobody trusts.
    struct Locked {
        /// The directory that holds the project
        directory: tempfile::TempDir,
    }

    impl Locked {
        /// Creates a project with the given configuration, and runs
        /// `mise lock` in it, as a contributor does
        fn new(configuration: &str) -> Self {
            let directory = tempfile::tempdir().unwrap();
            std::fs::write(directory.path().join("mise.toml"), configuration).unwrap();
            let project = Self { directory };

            project.mise(&["trust", "--quiet", "mise.toml"]);
            project.mise(&["lock", "--quiet"]);

            project
        }

        /// Returns the canonical root of the project
        ///
        /// The root is canonical, so the paths that mise reports do not depend
        /// on the symbolic links of the temporary directory.
        fn root(&self) -> ProjectRoot {
            ProjectRoot::canonical(self.directory.path()).unwrap()
        }

        /// Runs mise in the project with the given arguments
        fn mise(&self, arguments: &[&str]) {
            let status = std::process::Command::new(MISE)
                .args(arguments)
                .current_dir(self.directory.path())
                .status()
                .unwrap();

            assert!(status.success(), "expected mise to run {arguments:?}");
        }
    }

    impl Drop for Locked {
        /// Withdraws the trust that the project received
        ///
        /// Mise records the trust of a configuration in the state of the user.
        /// The temporary directory of a test never comes back, so the record
        /// would pile up there forever. A failure stays quiet, because the test
        /// already reported what it was about.
        fn drop(&mut self) {
            let _ = std::process::Command::new(MISE)
                .args(["trust", "--quiet", "--untrust", "mise.toml"])
                .current_dir(self.directory.path())
                .status();
        }
    }

    #[test]
    fn invocation_of_a_granted_directory_trusts_the_directory() {
        let mise = Mise::builder().root(root()).trust(Trust::Granted).build();

        assert_eq!(trusted(&mise), Some(root().get().as_os_str().to_owned()));
    }

    #[test]
    fn invocation_of_an_inherited_directory_trusts_nothing_of_its_own() {
        let mise = Mise::builder().root(root()).trust(Trust::Inherited).build();

        assert_eq!(trusted(&mise), None);
    }

    #[test]
    fn invocation_takes_the_answer_of_the_environment_out() {
        let mise = Mise::builder().root(root()).trust(Trust::Granted).build();

        let removed: Vec<String> = mise
            .invocation()
            .removals()
            .iter()
            .map(|name| name.get().to_string_lossy().into_owned())
            .collect();

        assert_eq!(removed[..1], [YES]);
    }

    #[test]
    fn invocation_takes_the_signs_of_ci_out() {
        let mise = Mise::builder().root(root()).trust(Trust::Granted).build();

        let removed: Vec<String> = mise
            .invocation()
            .removals()
            .iter()
            .map(|name| name.get().to_string_lossy().into_owned())
            .collect();

        assert_eq!(removed[1..], CI);
    }

    #[test]
    fn listed_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Listed>();
    }

    #[test]
    fn listed_with_a_list_that_is_not_json_reports_the_list() {
        let error = listed("mise.lock", &root()).unwrap_err();

        assert!(
            matches!(&error, CheckMiseLockError::UnreadableListing { listing, .. } if listing == "mise.lock"),
            "expected the list to be named, got {error:?}"
        );
    }

    #[test]
    fn listed_with_a_lockfile_outside_the_directory_reports_it() {
        let lockfile = serde_json::to_string(&path("/elsewhere/mise.lock")).unwrap();
        let listing = format!("[{{\"lockfile\": {lockfile}}}]");

        let error = listed(&listing, &root()).unwrap_err();

        assert!(
            matches!(error, CheckMiseLockError::ForeignLockfile { .. }),
            "expected a lockfile outside the project, got {error:?}"
        );
    }

    #[test]
    fn listed_with_several_lockfiles_names_each_of_them() {
        let listing = r#"[
            {"lockfile": "mise.lock", "root": ".mise/locks", "sidecars": []},
            {"lockfile": ".config/mise.lock", "root": ".config/mise/locks", "sidecars": []}
        ]"#;

        let paths = listed(listing, &root()).unwrap();

        assert_eq!(
            paths,
            [
                "mise.lock".parse::<FilePath>().unwrap(),
                ".config/mise.lock".parse::<FilePath>().unwrap(),
            ]
        );
    }

    #[test]
    fn lock_invocation_makes_mise_show_its_warnings() {
        let mise = Mise::builder().root(root()).trust(Trust::Inherited).build();

        let set: Vec<(String, String)> = mise
            .lock_invocation()
            .environment()
            .iter()
            .map(|variable| {
                (
                    variable.name().get().to_string_lossy().into_owned(),
                    variable.value().get().to_string_lossy().into_owned(),
                )
            })
            .collect();

        assert_eq!(
            set,
            VERBOSITY.map(|(name, value)| (name.to_owned(), value.to_owned()))
        );
    }

    #[test]
    fn lock_invocation_passes_no_option() {
        let mise = Mise::builder().root(root()).trust(Trust::Inherited).build();

        let arguments: Vec<String> = mise
            .lock_invocation()
            .arguments()
            .iter()
            .map(|argument| argument.get().to_string_lossy().into_owned())
            .collect();

        assert_eq!(arguments, [LOCK]);
    }

    /// Shows that the lock of a pinned project names its platforms
    ///
    /// The platforms come from the report that mise writes, so this test reads
    /// the report of the mise that the tests run.
    #[tokio::test]
    async fn lock_of_a_pinned_project_names_the_platforms_that_it_targets() {
        let project = Locked::new(PINNED);
        let mise = Mise::builder()
            .root(project.root())
            .trust(Trust::Inherited)
            .build();

        let Lock::Written { targets } = mise.lock().await.unwrap() else {
            panic!("expected mise to lock the project");
        };

        let lockfile = "mise.lock".parse::<FilePath>().unwrap();
        let wanted = [Platform::new("linux-x64"), Platform::new("macos-arm64")];
        assert!(
            targets
                .get(&lockfile)
                .is_some_and(|targeted| wanted.iter().all(|platform| targeted.contains(platform))),
            "expected the targets of mise.lock to hold {wanted:?}, got {targets:?}"
        );
    }

    #[test]
    fn mise_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Mise>();
    }

    #[test]
    fn plain_with_colors_drops_them() {
        let text = "\u{1b}[33m!\u{1b}[0m No tools configured to lock in this project";

        assert_eq!(plain(text), "! No tools configured to lock in this project");
    }

    /// Shows that targets with a home directory that ends with a separator name
    /// its lockfile
    ///
    /// Mise replaces the text of the home directory with `~`, so a home
    /// directory that ends with a separator leaves no separator after the `~`.
    #[test]
    fn targets_with_a_home_directory_that_ends_with_a_separator_name_its_lockfile() {
        let home = PathBuf::from(format!("{}{MAIN_SEPARATOR}", home().display()));
        let output = targeting_from_home(&path("project/mise.lock").display().to_string());

        let targets = targets(&output, &root(), Some(&home)).unwrap();

        assert_eq!(targets, targeted_lockfile());
    }

    #[test]
    fn targets_with_a_line_for_a_lockfile_name_its_platforms() {
        let lockfile = root().get().join("mise.lock");
        let output = format!(
            "\u{2192} Targeting 2 platform(s) for {}: linux-x64, macos-arm64\n\u{2192} Processing 1 tool(s): jq@1.8.1\n",
            lockfile.display()
        );

        let targets = targets(&output, &root(), None).unwrap();

        assert_eq!(
            targets,
            BTreeMap::from([(
                "mise.lock".parse::<FilePath>().unwrap(),
                BTreeSet::from([Platform::new("linux-x64"), Platform::new("macos-arm64")]),
            )])
        );
    }

    #[test]
    fn targets_with_a_line_under_the_home_directory_name_its_lockfile() {
        let output = targeting_from_home(&separated("project/mise.lock"));

        let targets = targets(&output, &root(), Some(&home())).unwrap();

        assert_eq!(targets, targeted_lockfile());
    }

    #[test]
    fn targets_with_a_lockfile_outside_the_directory_report_it() {
        let lockfile = path("/elsewhere/mise.lock");
        let output = format!(
            "\u{2192} Targeting 1 platform(s) for {}: linux-x64\n",
            lockfile.display()
        );

        let error = targets(&output, &root(), Some(&home())).unwrap_err();

        assert!(
            matches!(error, CheckMiseLockError::ForeignLockfile { .. }),
            "expected a lockfile outside the directory, got {error:?}"
        );
    }

    #[test]
    fn targets_with_a_lockfile_under_the_home_directory_outside_the_directory_report_it() {
        let output = targeting_from_home(&separated("elsewhere/mise.lock"));

        let targets = targets(&output, &root(), Some(&home()));

        assert!(
            matches!(targets, Err(CheckMiseLockError::ForeignLockfile { .. })),
            "expected a lockfile outside the directory, got {targets:?}"
        );
    }

    /// Shows that targets without a home directory report a lockfile under it
    ///
    /// Without a home directory, the action cannot tell where the lockfile
    /// lies, so it cannot place the lockfile inside the directory.
    #[test]
    fn targets_without_a_home_directory_report_a_lockfile_under_it() {
        let output = targeting_from_home(&separated("project/mise.lock"));

        let targets = targets(&output, &root(), None);

        assert!(
            matches!(targets, Err(CheckMiseLockError::ForeignLockfile { .. })),
            "expected a lockfile that the action cannot place, got {targets:?}"
        );
    }

    #[test]
    fn unresolved_with_a_failed_query_names_it() {
        let output = "mise WARN  [tamasfe/taplo] failed to fetch version tags for lockfile, URL may be incorrect: error sending request\n";

        let details = unresolved(output);

        assert_eq!(
            details.as_deref(),
            Some(
                "mise WARN  [tamasfe/taplo] failed to fetch version tags for lockfile, URL may be incorrect: error sending request"
            )
        );
    }

    #[test]
    fn unresolved_with_a_failed_resolution_names_it() {
        let output = "→ Processing 1 tool(s): jq@1.8.1\nmise WARN  failed to resolve jq for linux-x64: lock-time provenance verification failed\n";

        let details = unresolved(output);

        assert_eq!(
            details.as_deref(),
            Some(
                "mise WARN  failed to resolve jq for linux-x64: lock-time provenance verification failed"
            )
        );
    }

    #[test]
    fn unresolved_with_a_failure_that_names_a_harmless_word_names_it() {
        let output =
            "mise WARN  failed to resolve tool for linux-x64: the ubi backend is deprecated\n";

        let details = unresolved(output);

        assert_eq!(
            details.as_deref(),
            Some("mise WARN  failed to resolve tool for linux-x64: the ubi backend is deprecated")
        );
    }

    #[test]
    fn unresolved_with_a_limit_of_the_rate_names_it() {
        let output = "mise WARN  GitHub rate limit exceeded. Resets at 2026-10-17 13:28:37 +02:00\nmise WARN  HTTP GET https://api.github.com/repos/jqlang/jq/releases/tags/v1.8.1 attempt 1 failed after 11.8ms (transient): HTTP status client error\n\u{2713} Updated 0 platform entries (2 skipped)\n";

        let details = unresolved(output);

        assert_eq!(
            details.as_deref(),
            Some("mise WARN  GitHub rate limit exceeded. Resets at 2026-10-17 13:28:37 +02:00")
        );
    }

    #[test]
    fn unresolved_with_a_repeated_warning_names_it_once() {
        let output =
            "mise WARN  GitHub rate limit exceeded.\nmise WARN  GitHub rate limit exceeded.\n";

        let details = unresolved(output);

        assert_eq!(
            details.as_deref(),
            Some("mise WARN  GitHub rate limit exceeded.")
        );
    }

    #[test]
    fn unresolved_with_a_warning_over_several_lines_names_all_of_them() {
        let output = "mise WARN  failed to resolve python for macos-arm64: No GitHub artifact attestations found for python@3.13.1\nTo disable attestation verification, set MISE_PYTHON_GITHUB_ATTESTATIONS=false\nor add python.github_attestations = false under [settings] (version 3.13.1, and 1 more platform(s))\nmise lock                              \u{2713} 0 platform entries\n\u{2713} Updated 0 platform entries (2 skipped)\n";

        let details = unresolved(output);

        assert_eq!(
            details.as_deref(),
            Some(
                "mise WARN  failed to resolve python for macos-arm64: No GitHub artifact attestations found for python@3.13.1\nTo disable attestation verification, set MISE_PYTHON_GITHUB_ATTESTATIONS=false\nor add python.github_attestations = false under [settings] (version 3.13.1, and 1 more platform(s))"
            )
        );
    }

    #[test]
    fn unresolved_with_an_older_lockfile_format_names_nothing() {
        let output = "mise WARN  mise.lock uses lockfile format version 1; run `mise lock --upgrade` to enable the latest lockfile features\n✓ Updated 21 platform entries (42 skipped)\n";

        let details = unresolved(output);

        assert_eq!(details, None);
    }

    #[test]
    fn unresolved_with_harmless_warnings_names_nothing() {
        let output = "mise WARN  yamllint is locked to pipx:yamllint, but the registry now installs it from pypi:yamllint.\nmise WARN  deprecated [ubi]: The ubi backend is deprecated.\nmise WARN  HTTP GET https://example.com attempt 1 failed after 11.8ms (transient): timed out\nmise WARN  mise-versions endpoint=github_release repo=jqlang/jq outcome=failed status=0 fallback=true\nmise WARN  mise version 2026.10.12 available\nmise WARN  self-update is disabled for this install\n";

        let details = unresolved(output);

        assert_eq!(details, None);
    }
}
