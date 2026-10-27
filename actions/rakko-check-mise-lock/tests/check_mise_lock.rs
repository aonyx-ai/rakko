//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it with the mise of the machine, so the test answers for the
//! lockfiles that a real `mise lock` writes.
//!
//! A project of a test is a git repository with one empty commit, and the
//! files of the project are untracked on top of it. That is the tree of a
//! contributor who has not committed yet, which is the tree that the copy has
//! to carry over.
//!
//! A project pins `jq`, a small tool that mise downloads, so its lockfile
//! records a download for each platform. Mise asks the network for those
//! downloads, so the tests need it. A project that a test needs with a
//! complete lockfile gets that lockfile from `mise lock` itself, so the
//! platform of the machine that runs the tests is always in it.
//!
//! The projects lock in the default mode of mise, as the projects that use
//! the action do. In that mode, `mise lock` rebuilds the lockfile for the
//! platforms that it targets and removes every other entry. The tests expect
//! the answers of the version of mise that CI pins, and a mise that locks in
//! another mode, because it is older or because its user chose the mode,
//! answers some of them differently.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::unwrap_used)]

use std::process::Command;

use rakko_action::{
    Action, Args, ArgsValues, ArgumentValue, Context, Finding, Location, Outcome, ProjectRoot,
    argument_name,
};
use rakko_check_mise_lock::{
    CheckMiseLock, CheckMiseLockArgs, CheckMiseLockError, Lock, Mise, Platform, Trust,
};
use rakko_test_utils::{path_text, run_checking_access};
use tempfile::TempDir;

/// The configuration of a project that pins one tool that mise downloads
///
/// Mise checks the provenance of every download when it locks, and it asks a
/// service over the network for each platform. A busy service makes mise skip
/// a platform, so a test whose project kept the checks would fail now and
/// then for a reason that is not the code under test. The configuration
/// therefore turns the checks off, and mise then locks from what it knows of
/// the tool.
const PINNED: &str = "[tools]\njq = \"1.8.1\"\n\n[settings]\nlockfile = true\nlockfile_platforms = [\"linux-x64\", \"macos-arm64\"]\naqua.github_attestations = false\naqua.slsa = false\naqua.cosign = false\naqua.minisign = false\n";

/// The configuration of a project that pins two tools that mise downloads
///
/// The configuration turns off the checks of provenance, as the one above
/// does.
const PINNED_TWICE: &str = "[tools]\njq = \"1.8.1\"\ntaplo = \"0.10.0\"\n\n[settings]\nlockfile = true\nlockfile_platforms = [\"linux-x64\", \"macos-arm64\"]\naqua.github_attestations = false\naqua.slsa = false\naqua.cosign = false\naqua.minisign = false\n";

/// The configuration of a project whose mise waits only one millisecond for
/// the network
///
/// The configuration keeps the checks of provenance, and every download that
/// they need times out, so `mise lock` resolves no platform of the tool and
/// ends with an error.
const IMPATIENT: &str = "[tools]\njq = \"1.8.1\"\n\n[settings]\nlockfile = true\nhttp_timeout = \"1ms\"\nlockfile_platforms = [\"linux-x64\", \"macos-arm64\"]\n";

/// The configuration that a contributor keeps for themselves
///
/// It pins a second tool, and git ignores it, so the copy of the project does
/// not hold it.
const LOCAL: &str = "[tools]\ntaplo = \"0.10.0\"\n";

/// The lockfile of the configuration that a contributor keeps for themselves
const LOCAL_LOCK: &str = "lockfile_version = 1\n\n[[tools.taplo]]\nversion = \"0.10.0\"\nbackend = \"aqua:tamasfe/taplo\"\nspecifiers = [\"0.10.0\"]\n";

/// The patterns that keep the configuration of a contributor out of git
const IGNORED_LOCAL: &str = "mise.local.toml\nmise.local.lock\n";

/// The configuration of a project that pins a release that does not exist
///
/// `mise lock` writes a table for each platform that holds only the
/// provenance of the download, warns about nothing, and ends with success,
/// and a second run leaves that lockfile as it is.
const PHANTOM: &str = "[tools]\njq = \"1.99.0\"\n\n[settings]\nlockfile = true\nlockfile_platforms = [\"linux-x64\", \"macos-arm64\"]\n";

/// The impatient configuration, in a project that also asks mise to be quiet
const IMPATIENT_AND_QUIET: &str = "[tools]\njq = \"1.8.1\"\n\n[settings]\nlockfile = true\nquiet = true\nlog_level = \"error\"\nhttp_timeout = \"1ms\"\nlockfile_platforms = [\"linux-x64\", \"macos-arm64\"]\n";

/// The configuration of a project that pins no tool
const UNPINNED: &str = "[settings]\nlockfile = true\n";

/// The configuration of a project that no released mise accepts
///
/// Mise refuses every command for a configuration that asks for a newer
/// version of mise, whatever the machine trusts.
const UNSATISFIABLE: &str = "min_version = \"9999.0.0\"\n\n[tools]\njq = \"1.8.1\"\n";

/// The configuration of a project that pins a tool that no registry knows
///
/// Mise names the lockfiles of such a project, and it fails when it locks
/// the tool.
const UNKNOWN: &str = "[tools]\n\"aqua:rakko-no-such-owner/no-such-tool\" = \"1.0.0\"\n";

/// An entry of a tool that the configuration of a project does not pin
///
/// `mise lock` removes such an entry from the lockfile.
const STALE_ENTRY: &str = "\n[[tools.\"cargo:cargo-deny\"]]\nversion = \"0.20.2\"\nbackend = \"cargo:cargo-deny\"\nspecifiers = [\"0.20.2\"]\n";

/// An entry of jq for a platform that the projects do not target
///
/// It is the entry that `mise lock` writes for `macos-x64`, a platform on
/// which no job of CI runs the tests. A machine that locks on a platform
/// outside the list of a project writes such an entry, and `mise lock` on
/// another machine removes it.
const UNTARGETED_ENTRY: &str = "\n[tools.jq.\"platforms.macos-x64\"]\nchecksum = \"sha256:e80dbe0d2a2597e3c11c404f03337b981d74b4a8504b70586c354b7697a7c27f\"\nchecksum_source = \"mise-versions\"\nurl = \"https://github.com/jqlang/jq/releases/download/jq-1.8.1/jq-macos-amd64\"\nurl_api = \"https://api.github.com/repos/jqlang/jq/releases/assets/268981505\"\n";

/// A lockfile whose entry names no version
///
/// Mise accepts such an entry and replaces it when it locks, and the action
/// cannot tell which part of the lockfile the entry is.
const VERSIONLESS: &str = "lockfile_version = 1\n\n[[tools.jq]]\nbackend = \"aqua:jqlang/jq\"\n";

/// The header of a table that records a platform of jq
const JQ_PLATFORM: &str = "[tools.jq.\"platforms.";

/// The prefix of the names of the environment that git reads for itself
///
/// A test that runs inside a hook, a rebase, or a bisect of another repository
/// inherits variables that name that repository, and those beat the directory
/// that a command runs in. They therefore leave the environment of every git
/// command of a test.
const GIT_PREFIX: &[u8] = b"GIT_";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,

    /// The path of the configuration of mise, relative to the project
    configuration: &'static str,
}

impl Project {
    /// Creates a project with the given configuration, outside a repository
    ///
    /// The configuration is trusted right away, because mise ignores a
    /// configuration that nobody trusts.
    fn without_repository(configuration: &str) -> Self {
        let project = Self::untrusted(configuration);

        project.trust();

        project
    }

    /// Creates a project with the given configuration, in a repository
    ///
    /// The repository holds one empty commit, and every file of the project
    /// is untracked, which is the tree of a contributor who has not committed
    /// their work yet.
    fn new(configuration: &str) -> Self {
        let project = Self::without_repository(configuration);

        project.initialize();

        project
    }

    /// Creates a project in a repository, with the configuration at the given
    /// path
    fn at(path: &'static str, configuration: &str) -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self {
            directory,
            configuration: path,
        };

        project.write(path, configuration);
        project.trust();
        project.initialize();

        project
    }

    /// Creates a project with the given configuration that nobody trusted,
    /// in a repository
    fn untrusted(configuration: &str) -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self {
            directory,
            configuration: "mise.toml",
        };

        project.write("mise.toml", configuration);

        project
    }

    /// Creates a project whose lockfile `mise lock` wrote
    fn locked(configuration: &str) -> Self {
        let project = Self::new(configuration);

        project.lock();

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Runs git in the project
    ///
    /// The command forgets the variables that name a repository, so a test
    /// that runs inside a hook of another repository still works on the one
    /// that it created.
    fn git(&self, arguments: &[&str]) {
        let mut command = Command::new("git");
        command.current_dir(self.directory.path()).args(arguments);

        for (name, _) in std::env::vars_os() {
            if name.as_encoded_bytes().starts_with(GIT_PREFIX) {
                command.env_remove(name);
            }
        }

        let status = command.status().expect("the test starts git");

        assert!(status.success(), "expected git to run {arguments:?}");
    }

    /// Returns whether the project holds anything at the path
    fn holds(&self, path: &str) -> bool {
        self.directory.path().join(path).exists()
    }

    /// Makes the project a repository with one empty commit
    fn initialize(&self) {
        self.git(&["init", "--quiet", "--initial-branch=main", "."]);
        self.git(&["config", "user.name", "Rakko"]);
        self.git(&["config", "user.email", "rakko@example.com"]);
        self.git(&["config", "commit.gpgsign", "false"]);
        self.git(&["commit", "--quiet", "--allow-empty", "--message", "Start"]);
    }

    /// Runs `mise lock` in the project, as a contributor does
    fn lock(&self) {
        let status = Command::new("mise")
            .arg("lock")
            .arg("--quiet")
            .current_dir(self.directory.path())
            .status()
            .expect("the test starts mise to lock the project");

        assert!(status.success(), "expected mise to lock the project");
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file of the project")
    }

    /// Runs the action against this project, and reports
    async fn run(&self) -> Outcome {
        CheckMiseLock
            .run(&self.context(), &CheckMiseLockArgs::default())
            .await
    }

    /// Runs the action against this project, and fixes
    async fn fix(&self) -> Outcome {
        fix(&self.context()).await
    }

    /// Removes the tables of every platform of jq from the lockfile
    ///
    /// Returns the platforms that the lockfile recorded for jq, which are the
    /// entries that a run of `mise lock` adds again.
    fn strip(&self) -> Vec<String> {
        let mut platforms = Vec::new();
        let mut kept = Vec::new();
        let mut skipping = false;

        let lockfile = self.read("mise.lock");

        for line in lockfile.lines() {
            if let Some(rest) = line.strip_prefix(JQ_PLATFORM) {
                platforms.push(rest.trim_end_matches("\"]").to_owned());
                skipping = true;
            } else if skipping && line.starts_with('[') {
                skipping = false;
            }

            if !skipping {
                kept.push(line);
            }
        }

        self.write("mise.lock", &(kept.join("\n") + "\n"));

        platforms
    }

    /// Trusts the configuration of the project, so that mise reads it
    fn trust(&self) {
        let trusted = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg(self.directory.path().join(self.configuration))
            .status()
            .expect("the test starts mise to trust a configuration");

        assert!(
            trusted.success(),
            "expected mise to trust the configuration"
        );
    }

    /// Writes a file of the project, with the directories that lead to it
    fn write(&self, path: &str, content: &str) {
        let path = self.directory.path().join(path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the test creates the directories of a file");
        }

        std::fs::write(&path, content).expect("the test writes a file of the project");
    }
}

impl Drop for Project {
    /// Withdraws the trust that the project received
    ///
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
    /// pile up there forever. A failure stays quiet, because the test already
    /// reported what it was about.
    fn drop(&mut self) {
        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(self.directory.path().join(self.configuration))
            .status();
    }
}

/// Runs the action against the project of the context, and fixes
async fn fix(context: &Context) -> Outcome {
    let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);
    let args = CheckMiseLockArgs::from_values(&values).expect("the test passes a valid value");

    CheckMiseLock.run(context, &args).await
}

/// Returns the paths that the findings of an outcome name
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } => path.to_string(),
            other => panic!("expected a finding at a file, got {other:?}"),
        })
        .collect()
}

/// Returns the messages of the given findings
fn messages(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.message().get().to_owned())
        .collect()
}

/// Returns the messages that report the given platforms of jq with a verb
fn reports(verb: &str, platforms: &[String]) -> Vec<String> {
    reports_of("jq 1.8.1", verb, platforms)
}

/// Returns the messages that report the given platforms of a tool with a
/// verb, in the order of the platforms
fn reports_of(tool: &str, verb: &str, platforms: &[String]) -> Vec<String> {
    let mut expected: Vec<String> = platforms
        .iter()
        .map(|platform| format!("mise lock {verb} the entry of {tool} for {platform}"))
        .collect();
    expected.sort();

    expected
}

/// Returns the platforms that a lockfile records for jq
fn platforms(lockfile: &str) -> Vec<String> {
    lockfile
        .lines()
        .filter_map(|line| line.strip_prefix(JQ_PLATFORM))
        .map(|rest| rest.trim_end_matches("\"]").to_owned())
        .collect()
}

#[test]
fn action_identifies_itself_as_check_mise_lock() {
    let name = CheckMiseLock.name();

    assert_eq!(name.get(), "check-mise-lock");
}

// The platforms come from the report that mise writes, so this test reads the
// report of the mise that the tests run.
#[tokio::test]
async fn lock_of_a_pinned_project_names_the_platforms_that_it_targets() {
    let project = Project::locked(PINNED);
    let mise = Mise::new(project.context().root().clone(), Trust::Inherited);

    let Lock::Written { targets } = mise.lock().await.expect("mise locks the project") else {
        panic!("expected mise to lock the project");
    };

    let lockfile = "mise.lock".parse().expect("the test names a relative path");
    let wanted = [Platform::new("linux-x64"), Platform::new("macos-arm64")];
    assert!(
        targets
            .get(&lockfile)
            .is_some_and(|targeted| wanted.iter().all(|platform| targeted.contains(platform))),
        "expected the targets of mise.lock to hold {wanted:?}, got {targets:?}"
    );
}

// A lockfile with a stale entry is one that `mise lock` rewrites, also in a
// run that then stops, so the lockfile shows where the run locked. The
// release that the project pins has no downloads, so `mise lock` ends with
// success after it removes the stale entry, and the run stops after it. A
// project whose downloads fail would not do: mise ends with an error before
// it writes anything, and the lockfile stays as it was wherever the run
// locked.
#[tokio::test]
async fn run_that_stops_after_the_lock_leaves_the_lockfile_as_it_was() {
    let project = Project::new(PHANTOM);
    project.lock();
    let stale = project.read("mise.lock") + STALE_ENTRY;
    project.write("mise.lock", &stale);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    let Some(
        CheckMiseLockError::UnaddressedPlatforms { .. }
        | CheckMiseLockError::UnexpectedWarnings { .. },
    ) = source.downcast_ref::<CheckMiseLockError>()
    else {
        panic!("expected the run to stop after the lock, got {source:?}");
    };
    assert_eq!(project.read("mise.lock"), stale);
}

// A field that the entry shares changes every part of the entry, the old
// table of an untargeted platform among them. Whether `mise lock` keeps that
// table or removes it, the table stops nothing.
#[tokio::test]
async fn run_with_a_changed_shared_field_next_to_an_untargeted_platform_fails() {
    let project = Project::locked(PINNED);
    let lockfile: Vec<String> = project
        .read("mise.lock")
        .lines()
        .filter(|line| !line.starts_with("specifiers"))
        .map(str::to_owned)
        .collect();
    project.write(
        "mise.lock",
        &format!(
            "{}\n\n{JQ_PLATFORM}linux-x64-musl\"]\nprovenance = \"github-attestations\"\n",
            lockfile.join("\n")
        ),
    );

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail with findings, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_complete_lockfile_passes() {
    let project = Project::locked(PINNED);

    let outcome = project.run().await;

    assert!(
        matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "compared 1 lockfile"),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_configuration_in_a_directory_reports_its_lockfile() {
    let project = Project::at(".config/mise.toml", PINNED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    project.lock();
    let expected: Vec<(String, String)> =
        reports("adds", &platforms(&project.read(".config/mise.lock")))
            .into_iter()
            .map(|message| (path_text(".config/mise.lock"), message))
            .collect();
    let reported: Vec<(String, String)> = locations(&findings)
        .into_iter()
        .zip(messages(&findings))
        .collect();
    assert_eq!(reported, expected);
}

#[tokio::test]
async fn run_with_a_mise_that_fails_stops_with_what_it_wrote() {
    let project = Project::new(UNSATISFIABLE);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("9999.0.0"),
        "expected what mise wrote, got {source}"
    );
}

// `mise lock` fills in the checksum that an entry of a targeted platform
// lacks, in either mode.
#[tokio::test]
async fn run_with_a_missing_checksum_reports_the_change() {
    let project = Project::locked(PINNED);
    let lockfile = project.read("mise.lock");
    let platform = lockfile
        .lines()
        .find_map(|line| line.strip_prefix(JQ_PLATFORM))
        .map(|rest| rest.trim_end_matches("\"]").to_owned())
        .expect("the lockfile records a platform of jq");
    let checksum = lockfile
        .lines()
        .find(|line| line.starts_with("checksum = "))
        .expect("the lockfile records a checksum of jq");
    let missing = lockfile.replacen(&format!("{checksum}\n"), "", 1);
    project.write("mise.lock", &missing);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        messages(&findings),
        [format!(
            "mise lock changes the entry of jq 1.8.1 for {platform}"
        )]
    );
}

// The lockfile holds the version of jq, so mise needs the network only for
// the platforms, and only those fail. Mise then ends with an error.
#[tokio::test]
async fn run_with_a_platform_that_mise_fails_to_resolve_stops_with_what_it_wrote() {
    let project = Project::locked(PINNED);
    project.strip();
    project.write("mise.toml", IMPATIENT);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::UnlockedProject { details }) if details.contains("failed to resolve jq")
        ),
        "expected what mise wrote about jq, got {source:?}"
    );
}

// A project that asks mise to be quiet still gets the words of mise in the
// error.
#[tokio::test]
async fn run_with_a_quiet_project_that_mise_fails_to_resolve_stops() {
    let project = Project::locked(PINNED);
    project.strip();
    project.write("mise.toml", IMPATIENT_AND_QUIET);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::UnlockedProject { details }) if details.contains("failed to resolve jq")
        ),
        "expected what mise wrote about jq, got {source:?}"
    );
}

#[tokio::test]
async fn run_with_a_stale_entry_reports_its_removal() {
    let project = Project::locked(PINNED);
    let lockfile = project.read("mise.lock");
    project.write("mise.lock", &(lockfile + STALE_ENTRY));

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        messages(&findings),
        ["mise lock removes the entry of cargo:cargo-deny 0.20.2"]
    );
}

#[tokio::test]
async fn run_with_a_tool_that_mise_cannot_lock_stops_with_what_it_wrote() {
    let project = Project::new(UNKNOWN);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::UnlockedProject { details }) if details.contains("rakko-no-such-owner")
        ),
        "expected what mise wrote about the lock, got {source:?}"
    );
}

#[tokio::test]
async fn run_with_an_ignored_local_configuration_passes() {
    let project = Project::locked(PINNED);
    project.write(".gitignore", IGNORED_LOCAL);
    project.write("mise.local.toml", LOCAL);
    project.write("mise.local.lock", LOCAL_LOCK);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// The question for the lockfiles in the project makes mise read a lockfile
// that only the checkout holds, and mise refuses one that is not TOML.
#[tokio::test]
async fn run_with_an_ignored_lockfile_that_mise_cannot_read_stops() {
    let project = Project::locked(PINNED);
    project.write(".gitignore", IGNORED_LOCAL);
    project.write("mise.local.toml", LOCAL);
    project.write("mise.local.lock", "this is not toml [[[\n");

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::UnlistedLockfiles { .. })
        ),
        "expected mise to refuse the listing, got {source:?}"
    );
}

#[tokio::test]
async fn run_with_an_ignored_lockfile_that_the_action_cannot_read_passes() {
    let project = Project::locked(PINNED);
    project.write(".gitignore", IGNORED_LOCAL);
    project.write("mise.local.toml", LOCAL);
    project.write("mise.local.lock", VERSIONLESS);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_uncommitted_tool_reports_its_entries() {
    let project = Project::locked(PINNED);
    project.git(&["add", "--all"]);
    project.git(&["commit", "--quiet", "--message", "Pin jq"]);
    project.write("mise.toml", PINNED_TWICE);
    let targeted = platforms(&project.read("mise.lock"));

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        messages(&findings),
        reports_of("taplo 0.10.0", "adds", &targeted)
    );
}

#[tokio::test]
async fn run_with_an_unreadable_lockfile_stops() {
    let project = Project::new(PINNED);
    project.write("mise.lock", VERSIONLESS);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::UnreadableLockfile { .. })
        ),
        "expected an unreadable lockfile, got {source:?}"
    );
}

// A machine that locks on a platform outside the list of a project, such as
// the one of a bot that moves the pins, writes entries that `mise lock` on
// every other machine removes.
#[tokio::test]
async fn run_with_an_untargeted_platform_reports_its_removal() {
    let project = Project::locked(PINNED);
    let lockfile = project.read("mise.lock");
    project.write("mise.lock", &(lockfile + UNTARGETED_ENTRY));

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        messages(&findings),
        ["mise lock removes the entry of jq 1.8.1 for macos-x64"]
    );
}

#[tokio::test]
async fn run_with_fix_and_a_complete_lockfile_passes() {
    let project = Project::locked(PINNED);

    let outcome = project.fix().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_fix_and_missing_platforms_reports_a_repair_for_each() {
    let project = Project::locked(PINNED);
    let platforms = project.strip();

    let outcome = project.fix().await;

    let Outcome::Changed { repairs } = outcome else {
        panic!("expected the run to change the project, got {outcome:?}");
    };
    assert_eq!(messages(&repairs), reports("added", &platforms));
}

#[tokio::test]
async fn run_with_fix_and_missing_platforms_writes_them() {
    let project = Project::locked(PINNED);
    let complete = project.read("mise.lock");
    project.strip();

    project.fix().await;

    assert_eq!(project.read("mise.lock"), complete);
}

#[tokio::test]
async fn run_with_missing_platforms_leaves_the_lockfile_as_it_was() {
    let project = Project::locked(PINNED);
    project.strip();
    let stripped = project.read("mise.lock");

    project.run().await;

    assert_eq!(project.read("mise.lock"), stripped);
}

#[tokio::test]
async fn run_with_missing_platforms_leaves_the_project_unchanged() {
    let project = Project::locked(PINNED);
    project.strip();

    // The helper asserts that the run left every file of the project as it
    // was.
    run_checking_access(
        &CheckMiseLock,
        &project.context(),
        &CheckMiseLockArgs::default(),
    )
    .await;
}

#[tokio::test]
async fn run_with_missing_platforms_reports_a_finding_for_each() {
    let project = Project::locked(PINNED);
    let platforms = project.strip();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(messages(&findings), reports("adds", &platforms));
}

// The second gap that the parent of this action, the issue "Catch gaps in mise
// lockfiles in CI", names: the tables of the platforms held no address and no
// checksum. In that gap the tables still named the provenance of the
// download. The project turns the checks of provenance off, and mise can
// refuse a lockfile that names a provenance whose check is off, so the tables
// here hold nothing at all.
#[tokio::test]
async fn run_with_platforms_without_an_address_reports_the_change_of_each() {
    let project = Project::locked(PINNED);
    let lockfile = project.read("mise.lock");
    let platforms = platforms(&lockfile);
    let bare: Vec<&str> = lockfile
        .lines()
        .filter(|line| {
            !line.starts_with("url")
                && !line.starts_with("checksum")
                && !line.starts_with("provenance")
        })
        .collect();
    project.write("mise.lock", &(bare.join("\n") + "\n"));

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(messages(&findings), reports("changes", &platforms));
}

// The lockfile already holds the tables without an address that `mise lock`
// writes for the release, so the run leaves it as it is. Mise asks the API of
// GitHub about the release as well, and when that API limits the rate of the
// machine, mise warns, and the warning stops the run first. Either stop is
// the answer, and the unit tests of the two checks tell them apart.
#[tokio::test]
async fn run_with_targeted_platforms_without_an_address_stops() {
    let project = Project::new(PHANTOM);
    project.lock();

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(
                CheckMiseLockError::UnaddressedPlatforms { .. }
                    | CheckMiseLockError::UnexpectedWarnings { .. }
            )
        ),
        "expected the run to stop over the platforms without an address, got {source:?}"
    );
}

#[tokio::test]
async fn run_without_a_lockfile_creates_none() {
    let project = Project::new(PINNED);

    project.run().await;

    assert!(!project.holds("mise.lock"));
}

#[tokio::test]
async fn run_without_a_lockfile_reports_a_finding_for_each_part() {
    let project = Project::new(PINNED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    project.lock();
    assert_eq!(
        messages(&findings),
        reports("adds", &platforms(&project.read("mise.lock")))
    );
}

#[tokio::test]
async fn run_without_a_repository_stops() {
    let project = Project::without_repository(PINNED);

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::CopyUnavailable { .. })
        ),
        "expected the copy to be unavailable, got {source:?}"
    );
}

#[tokio::test]
async fn run_without_a_tool_skips_with_what_mise_wrote() {
    let project = Project::new(UNPINNED);

    let outcome = project.run().await;

    assert!(
        matches!(&outcome, Outcome::Skipped { reason } if reason.get().starts_with("No tools configured to lock")),
        "expected the run to skip, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_without_trust_stops_before_the_copy() {
    let project = Project::untrusted(PINNED);
    project.initialize();

    let outcome = project.run().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        matches!(
            source.downcast_ref::<CheckMiseLockError>(),
            Some(CheckMiseLockError::UnlistedLockfiles { .. })
        ),
        "expected mise to refuse the project, got {source:?}"
    );
}

/// The tests of a project under the home directory
///
/// Mise writes `~` in place of the home directory at the start of a path
/// under it. It does so only on Unix-like systems, so these tests run only
/// there. No test puts a project in the real home directory. Instead, a test
/// moves the home directory into a temporary directory for a second run of
/// this binary. That second run starts the action in the moved home
/// directory, and the action starts mise there.
#[cfg(unix)]
mod home {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rakko_action::{Context, Outcome, ProjectRoot};

    use super::{PINNED, Project, fix};

    /// The variable that names the project of the run in the moved home
    /// directory
    const MOVED_PROJECT: &str = "RAKKO_TEST_MOVED_PROJECT";

    /// The test that runs the action in the moved home directory, by the name
    /// that the test harness gives it
    const MOVED_TEST: &str = "home::run_with_fix_in_a_moved_home_directory_passes";

    /// The words with which the test harness reports that one test passed
    ///
    /// A filter that matches no test passes as well, so the test that starts
    /// the run checks that the run ran one.
    const ONE_PASSED: &str = "test result: ok. 1 passed";

    /// The variables that name the directories of mise, each with the field
    /// that names the same directory in the report of `mise doctor --json`
    const DIRECTORIES: [(&str, &str); 4] = [
        ("MISE_CACHE_DIR", "cache"),
        ("MISE_CONFIG_DIR", "config"),
        ("MISE_DATA_DIR", "data"),
        ("MISE_STATE_DIR", "state"),
    ];

    impl Project {
        /// Creates a project whose lockfile `mise lock` wrote, outside a
        /// repository, in a new directory inside the given one
        fn locked_in(parent: &Path, configuration: &str) -> Self {
            let directory =
                tempfile::tempdir_in(parent).expect("the test creates a temporary directory");
            let project = Self {
                directory,
                configuration: "mise.toml",
            };

            project.write("mise.toml", configuration);
            project.trust();
            project.lock();

            project
        }
    }

    /// Returns each variable that names a directory of mise, with the
    /// directory that mise uses now
    ///
    /// Unless its own variable names it, mise derives each of these
    /// directories from a variable of XDG, such as `XDG_DATA_HOME`, or else
    /// from the home directory. The run in the moved home directory sets all
    /// of them. Its mise then reads the trust, the configuration, and the
    /// cache of the machine.
    fn directories() -> Vec<(&'static str, PathBuf)> {
        let output = Command::new("mise")
            .arg("doctor")
            .arg("--json")
            .output()
            .expect("the test starts mise to name its directories");
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("mise writes its report as JSON");

        DIRECTORIES
            .iter()
            .map(|(variable, field)| {
                let directory = report["dirs"][field]
                    .as_str()
                    .expect("mise names the directory");

                (*variable, PathBuf::from(directory))
            })
            .collect()
    }

    // This test runs only in the run that the test below starts, because it
    // reads the project from the variable that the test below sets.
    #[tokio::test]
    #[ignore = "the test of a project under the home directory runs it"]
    async fn run_with_fix_in_a_moved_home_directory_passes() {
        let root = std::env::var_os(MOVED_PROJECT)
            .expect("the test of a project under the home directory names the project");
        let root = ProjectRoot::canonical(Path::new(&root))
            .expect("the test names a directory that exists");

        let outcome = fix(&Context::builder().root(root).build()).await;

        assert!(
            matches!(outcome, Outcome::Passed { .. }),
            "expected the run to pass, got {outcome:?}"
        );
    }

    // Mise names the lockfile by its canonical path. It writes `~` only where
    // that path starts with the home directory, so the moved home directory
    // is canonical too.
    #[test]
    fn run_with_fix_in_a_project_under_the_home_directory_passes() {
        let home = tempfile::tempdir().expect("the test creates a temporary directory");
        let moved = home
            .path()
            .canonicalize()
            .expect("the test names a directory that exists");
        let project = Project::locked_in(&moved, PINNED);

        let output = Command::new(std::env::current_exe().expect("the test finds its binary"))
            .arg(MOVED_TEST)
            .arg("--exact")
            .arg("--ignored")
            .env("HOME", &moved)
            .envs(directories())
            .env(MOVED_PROJECT, project.directory.path())
            .output()
            .expect("the test runs its binary");

        let report = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && report.contains(ONE_PASSED),
            "expected the run in the moved home directory to pass, got:\n{report}{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
