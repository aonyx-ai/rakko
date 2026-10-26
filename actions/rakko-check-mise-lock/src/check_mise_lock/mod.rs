//! The action that checks whether the lockfile of mise is complete
//!
//! This module holds the action, its arguments, and the error that stops a
//! run. The action reads the lockfiles of the project, runs `mise lock` in a
//! copy of the project or, with `fix`, in the project itself, and reads the
//! lockfiles again. Each part that the run added, changed, or removed becomes
//! a finding or a repair.

/// The arguments that a run reads
mod args;
/// The error that stops a run of the action
mod error;

use std::collections::{BTreeMap, BTreeSet};

use rakko_action::{
    Access, Action, Context, FilePath, Finding, Location, Name, Outcome, Summary, action_name,
};
use rakko_worktree::Worktree;

pub use self::args::CheckMiseLockArgs;
pub use self::error::CheckMiseLockError;
use crate::difference::Difference;
use crate::lockfile::Lockfile;
use crate::mise::{Lock, Mise, Trust};
use crate::part::Platform;

/// The action that checks whether the lockfile of mise is complete
///
/// A lockfile of [mise] records, for each tool that mise downloads, the
/// address and the checksum of the file for each platform. An install in
/// locked mode stops when the lockfile has no entry for the platform of the
/// machine, and it examines only that one platform, so a gap stays hidden
/// until a new machine installs the tool. Mise has no operation that only
/// checks, so the action runs `mise lock` and compares the lockfiles before
/// and after the run.
///
/// `mise lock` targets the platforms that the setting `lockfile_platforms` of
/// the project names, and the platform of the machine that runs it as well,
/// and in the default mode of mise it removes the entries of every other
/// platform. A project therefore names every platform on which something
/// locks it, that of a bot that moves its pins among them. Otherwise a run on
/// a machine of another platform reports the entries of that platform, as
/// added or as removed. The lockfile also depends on the version of mise, and
/// the version in CI decides when a contributor gets another answer.
///
/// A run reports by default, and it then runs `mise lock` in a disposable
/// copy of the project, so the checkout of a contributor stays as it was. The
/// copy holds the changes that are not committed, so a run answers for the
/// tree that the contributor works on. It holds no file that git ignores, so a
/// configuration such as an ignored `mise.local.toml` does not reach the run.
/// It lies in the temporary directory of the system, so a configuration of
/// mise in a directory above the project does not reach it either. No run of
/// mise inherits `MISE_YES`, with which an environment such as CI answers the
/// questions of mise in advance, so a project that nobody trusted stops the
/// run, and the action trusts nothing in the place of its owner. Mise still
/// reads every lockfile of the project when the run asks it for them there,
/// so an ignored lockfile that mise cannot read stops the run. With `fix`, the
/// run runs `mise lock` in the project, and its result stays there, also when
/// the run stops after `mise lock`. Such a run also compares and keeps what
/// `mise lock` writes to an ignored lockfile, such as `mise.local.lock`.
///
/// A run that reports declares a read of the project, because its copy
/// changes the project only inside `.git`, where git records the copy. A run
/// with `fix` declares a write.
///
/// Each tool, version, and platform that `mise lock` adds, changes, or removes
/// becomes one finding at its lockfile, or one repair with `fix`. A project
/// that configures no tool to lock skips visibly. A run stops with an error
/// when mise does not start or ends without success, when `mise lock` gives a
/// warning that can mean an unresolved tool, when a lockfile records a
/// platform that mise targeted without the address of its download after
/// `mise lock`, when a lockfile has a shape that the action cannot read, and
/// when the copy cannot be created. A warning of mise can be temporary, such
/// as one of the network, or last, such as a release without the attestations
/// that mise checks, and the words of mise in the error say which.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_mise_lock::CheckMiseLock;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckMiseLock)];
/// ```
///
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckMiseLock;

impl Action for CheckMiseLock {
    type Args = CheckMiseLockArgs;

    fn name(&self) -> Name {
        action_name!("check-mise-lock")
    }

    fn access(&self, _context: &Context, args: &Self::Args) -> Access {
        if args.fix() {
            Access::Write
        } else {
            Access::Read
        }
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run asks mise for the lockfiles of the project first. Mise refuses
/// that question for a project that nobody trusted, so such a project stops
/// before anything else happens. The run then locks the project, in a copy
/// without `fix` and in the project with it, and reads the lockfiles where it
/// locks, before and after. A run without `fix` only lists the lockfiles of
/// the project, because the copy holds no file that git ignores, and a
/// lockfile that only the checkout holds gives no difference.
///
/// The copy lives until this function returns, and the drop takes it with it,
/// so a run that ends with an answer and a run that stops with an error clear
/// it alike.
///
/// # Errors
///
/// Returns the error of the step that could not finish: a run of mise, the
/// reading of a lockfile, or the creation of the copy. Returns
/// [`UnaddressedPlatforms`][unaddressed] when a lockfile records a platform
/// that mise targeted without the address of its download after
/// `mise lock`.
///
/// [unaddressed]: CheckMiseLockError::UnaddressedPlatforms
async fn drive(context: &Context, args: &CheckMiseLockArgs) -> Result<Outcome, CheckMiseLockError> {
    let project = Mise::new(context.root().clone(), Trust::Inherited);

    let (mise, before, _worktree) = if args.fix() {
        let before = project.lockfiles().await?;

        (project, before, None)
    } else {
        project.list().await?;

        let worktree = Worktree::create(context.root().clone())
            .await
            .map_err(|source| CheckMiseLockError::CopyUnavailable { source })?;

        let copy = Mise::new(worktree.root().clone(), Trust::Granted);

        let before = copy.lockfiles().await?;

        (copy, before, Some(worktree))
    };

    let targets = match mise.lock().await? {
        Lock::Written { targets } => targets,
        Lock::Unconfigured { reason } => return Ok(Outcome::Skipped { reason }),
    };

    let after = mise.lockfiles().await?;
    let differences = differences(&before, &after);

    if let Some(path) = untargeted(&after, &targets) {
        return Err(CheckMiseLockError::UntargetedLockfile {
            path: path.get().to_path_buf(),
        });
    }

    if let Some(parts) = unaddressed(&after, &targets) {
        return Err(CheckMiseLockError::UnaddressedPlatforms { parts });
    }

    if differences.is_empty() {
        let compared = before.keys().chain(after.keys()).collect::<BTreeSet<_>>();

        return Ok(Outcome::Passed {
            summary: Some(summary(compared.len())),
        });
    }

    if args.fix() {
        Ok(Outcome::Changed {
            repairs: findings(&differences, Difference::repair),
        })
    } else {
        Ok(Outcome::Failed {
            findings: findings(&differences, Difference::finding),
            repairs: Vec::new(),
        })
    }
}

/// Returns the differences of every lockfile, by the path of the lockfile
///
/// A lockfile that only one side holds counts as a lockfile without parts on
/// the other side, so a lockfile that `mise lock` created gives a difference
/// for each of its parts.
fn differences(
    before: &BTreeMap<FilePath, Lockfile>,
    after: &BTreeMap<FilePath, Lockfile>,
) -> Vec<(FilePath, Difference)> {
    let empty = Lockfile::default();
    let paths: BTreeSet<&FilePath> = before.keys().chain(after.keys()).collect();

    paths
        .into_iter()
        .flat_map(|path| {
            let earlier = before.get(path).unwrap_or(&empty);
            let later = after.get(path).unwrap_or(&empty);

            earlier
                .differences(later)
                .into_iter()
                .map(|difference| (path.clone(), difference))
        })
        .collect()
}

/// Returns the first lockfile that records a platform while mise named no
/// platform that it targeted for it, if there is one
///
/// Mise names the platforms that it targets for each lockfile that it locks.
/// Without them, the run cannot tell an entry of a targeted platform from an
/// old one, so a lockfile with any platform and without targets stops the
/// run rather than pass it.
fn untargeted<'a>(
    after: &'a BTreeMap<FilePath, Lockfile>,
    targets: &BTreeMap<FilePath, BTreeSet<Platform>>,
) -> Option<&'a FilePath> {
    after
        .iter()
        .find(|(path, lockfile)| lockfile.records_platforms() && !targets.contains_key(*path))
        .map(|(path, _)| path)
}

/// Returns the parts of a targeted platform that the lockfiles hold without
/// the address of their download after the run, each with its lockfile, if
/// they hold any
///
/// A part counts when mise targeted its platform for its lockfile, whether or
/// not the run changed it, because locked mode refuses it either way. An
/// entry of a platform that mise does not target says nothing about the run,
/// whether `mise lock` removes it, as it does in its default mode, or keeps
/// it.
fn unaddressed(
    after: &BTreeMap<FilePath, Lockfile>,
    targets: &BTreeMap<FilePath, BTreeSet<Platform>>,
) -> Option<String> {
    let parts: Vec<String> = after
        .iter()
        .flat_map(|(path, lockfile)| {
            lockfile
                .unaddressed()
                .into_iter()
                .filter(|part| {
                    let targeted = targets.get(path);

                    part.platform()
                        .as_ref()
                        .is_some_and(|platform| targeted.is_some_and(|set| set.contains(platform)))
                })
                .map(move |part| format!("{part} in {path}"))
        })
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

/// Returns a finding at its lockfile for each difference
///
/// The message comes from the difference, in the words of a finding or of a
/// repair.
fn findings(
    differences: &[(FilePath, Difference)],
    message: fn(&Difference) -> String,
) -> Vec<Finding> {
    differences
        .iter()
        .map(|(path, difference)| {
            Finding::builder()
                .message(message(difference))
                .location(Location::File { path: path.clone() })
                .build()
        })
        .collect()
}

/// Returns the summary of a run that compared the given number of lockfiles
fn summary(compared: usize) -> Summary {
    if compared == 1 {
        Summary::new("compared 1 lockfile")
    } else {
        Summary::new(format!("compared {compared} lockfiles"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{Args, ArgsValues, ArgumentValue, argument_name};

    use super::*;

    /// A lockfile with one tool on one platform
    const LOCKFILE: &str = "[[tools.jq]]\nversion = \"1.8.1\"\n\n[tools.jq.\"platforms.linux-x64\"]\nurl = \"https://example.com/jq\"\n";

    /// The old table of a platform that mise does not target, without an
    /// address
    const STALE: &str =
        "\n[tools.jq.\"platforms.linux-x64-musl\"]\nprovenance = \"github-attestations\"\n";

    /// Returns the targets of a run that targeted the given platforms for
    /// `mise.lock`
    fn targets(platforms: &[&str]) -> BTreeMap<FilePath, BTreeSet<Platform>> {
        BTreeMap::from([(
            "mise.lock".parse().unwrap(),
            platforms.iter().copied().map(Platform::new).collect(),
        )])
    }

    /// Returns the given lockfiles, by the paths that their literals name on
    /// this platform
    fn lockfiles(entries: &[(&str, &str)]) -> BTreeMap<FilePath, Lockfile> {
        entries
            .iter()
            .map(|(literal, text)| {
                let path = FilePath::try_from(rakko_test_utils::path(literal)).unwrap();

                (path, Lockfile::parse(text).unwrap())
            })
            .collect()
    }

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns the arguments of a run that asks for a repair
    fn fixing() -> CheckMiseLockArgs {
        let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);

        CheckMiseLockArgs::from_values(&values).unwrap()
    }

    #[test]
    fn access_with_fix_is_write() {
        let args = fixing();

        let access = CheckMiseLock.access(&context(), &args);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn access_without_fix_is_read() {
        let args = CheckMiseLockArgs::default();

        let access = CheckMiseLock.access(&context(), &args);

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn differences_with_a_lockfile_that_only_the_project_held_name_its_parts() {
        let before = lockfiles(&[("mise.lock", LOCKFILE)]);
        let after = lockfiles(&[]);

        let differences = differences(&before, &after);

        assert_eq!(differences.len(), 1);
    }

    #[test]
    fn differences_with_a_lockfile_that_only_the_run_wrote_name_its_parts() {
        let before = lockfiles(&[]);
        let after = lockfiles(&[("mise.lock", LOCKFILE)]);

        let differences = differences(&before, &after);

        assert_eq!(differences.len(), 1);
    }

    #[test]
    fn differences_with_several_lockfiles_name_the_lockfile_of_each() {
        let before = lockfiles(&[("mise.lock", ""), (".config/mise.lock", "")]);
        let after = lockfiles(&[("mise.lock", LOCKFILE), (".config/mise.lock", LOCKFILE)]);

        let paths: Vec<String> = differences(&before, &after)
            .iter()
            .map(|(path, _)| path.to_string())
            .collect();

        assert_eq!(
            paths,
            [
                rakko_test_utils::path_text(".config/mise.lock"),
                "mise.lock".to_owned(),
            ]
        );
    }

    #[test]
    fn summary_of_one_lockfile_names_one_lockfile() {
        let summary = summary(1);

        assert_eq!(summary.get(), "compared 1 lockfile");
    }

    #[test]
    fn summary_of_several_lockfiles_names_their_number() {
        let summary = summary(2);

        assert_eq!(summary.get(), "compared 2 lockfiles");
    }

    #[test]
    fn unaddressed_with_a_targeted_platform_without_an_address_names_it() {
        let text = LOCKFILE.replace("url = ", "provenance = ");
        let after = lockfiles(&[("mise.lock", &text)]);

        assert_eq!(
            unaddressed(&after, &targets(&["linux-x64"])).as_deref(),
            Some("jq 1.8.1 for linux-x64 in mise.lock")
        );
    }

    #[test]
    fn unaddressed_with_an_untargeted_platform_without_an_address_names_nothing() {
        let after = lockfiles(&[("mise.lock", &format!("{LOCKFILE}{STALE}"))]);

        assert_eq!(unaddressed(&after, &targets(&["linux-x64"])), None);
    }

    #[test]
    fn unaddressed_with_complete_targeted_platforms_names_nothing() {
        let after = lockfiles(&[("mise.lock", LOCKFILE)]);

        assert_eq!(unaddressed(&after, &targets(&["linux-x64"])), None);
    }

    #[test]
    fn untargeted_with_a_lockfile_without_platforms_names_nothing() {
        let after = lockfiles(&[("mise.lock", "[[tools.rust]]\nversion = \"1.99.1\"\n")]);

        assert_eq!(untargeted(&after, &BTreeMap::new()), None);
    }

    #[test]
    fn untargeted_with_a_lockfile_without_targets_names_it() {
        let after = lockfiles(&[("mise.lock", LOCKFILE)]);

        assert_eq!(
            untargeted(&after, &BTreeMap::new()).map(ToString::to_string),
            Some("mise.lock".to_owned())
        );
    }

    #[test]
    fn untargeted_with_targets_names_nothing() {
        let after = lockfiles(&[("mise.lock", LOCKFILE)]);

        assert_eq!(untargeted(&after, &targets(&["linux-x64"])), None);
    }
}
