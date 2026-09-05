//! The disposable copy of a project that a job runs in
//!
//! A copy is a detached checkout of the commit that the HEAD of the project
//! names, with the changed files of the checkout synced over it. It therefore
//! holds what the contributor holds, and a job that writes into it changes
//! nothing that the contributor can see.
//!
//! The value owns the copy. It lives for as long as the jobs that run in it,
//! and the drop takes the copy with it.

/// What the crate reports when it cannot stand up a copy of a project
mod error;
/// The commands of git that the crate starts
mod git;
/// The paths where the tree of a contributor differs from its commit
mod status;

use std::fs;
use std::fs::Metadata;
use std::io::ErrorKind;
use std::io::Result as IoResult;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

use getset::Getters;
use kawauso_process::Execution;
use rakko_action::ProjectRoot;
use tempfile::TempDir;

pub use self::error::CreateWorktreeError;

/// The subcommand of git that reports facts about a repository
const REV_PARSE: &str = "rev-parse";

/// The option of `rev-parse` that reports the top level of a repository
const SHOW_TOPLEVEL: &str = "--show-toplevel";

/// The subcommand of git that manages the working trees of a repository
const WORKTREE: &str = "worktree";

/// The subcommand of `worktree` that creates one
const ADD: &str = "add";

/// The option of `worktree add` that checks out a commit without a branch
const DETACH: &str = "--detach";

/// The revision that a copy checks out
const HEAD: &str = "HEAD";

/// The subcommand of `worktree` that takes one away
const REMOVE: &str = "remove";

/// The option of `worktree remove` that takes a working tree with changes
///
/// A job writes into the copy, so the copy has changes by the time it goes.
/// Without the option git would keep the work of the job, which is what the
/// caller asked the copy to hold.
const FORCE: &str = "--force";

/// The subcommand of git that reports what changed in a working tree
const STATUS: &str = "status";

/// The option of `status` that asks for the format that a program reads
const PORCELAIN: &str = "--porcelain=v1";

/// The option of `status` that separates the entries with a zero byte
///
/// A path can hold a space, a quotation mark, and a line break. Every other
/// format escapes such a path, and this one writes it as it is.
const SEPARATED: &str = "-z";

/// The option of `status` that names every untracked file
///
/// Without it git names the directory that holds untracked files, and the
/// sync needs the files.
const UNTRACKED: &str = "--untracked-files=all";

/// The option of `status` that reports a rename as a removal and an addition
///
/// The sync copies a path that the project holds and removes a path that it
/// does not, so the two entries of a rename need no rule of their own.
const NO_RENAMES: &str = "--no-renames";

/// The name of the checkout inside the directory that the crate creates
///
/// Git names the entry of a worktree after this directory, and it makes the
/// name unique when a repository already holds one, so two copies of one
/// project can live at the same time.
const CHECKOUT: &str = "worktree";

/// The details of a failure that git did not explain
const NO_DIAGNOSIS: &str = "git wrote nothing about it";

/// The details of a run of git that ended with success and named no top level
const NO_TOP_LEVEL: &str = "git named no top level";

/// A disposable copy of a project that a job runs in
///
/// The copy is a git worktree in a directory of the temporary directory of
/// the system. It holds the commit that the HEAD of the project names, and
/// the files that the contributor changed on top of it, so a job that runs
/// there reads the tree that the contributor is working on.
///
/// The value owns the copy, and the drop removes it. A caller therefore holds
/// the value for as long as the jobs run, and lets it go when the answers are
/// in.
///
/// The copy is a checkout and not a sandbox. A job that writes outside of it
/// writes into the project, and nothing here stops that.
#[derive(Debug, Getters)]
pub struct Worktree {
    /// The root of the project that the copy belongs to
    #[getset(get = "pub")]
    project: ProjectRoot,

    /// The root of the copy
    ///
    /// The copy holds the tree of the project at this directory, so it is a
    /// project root of its own: the file that marks the root of the project
    /// arrived with the checkout.
    #[getset(get = "pub")]
    root: ProjectRoot,

    /// The directory that holds the copy
    ///
    /// The drop removes the directory, which is what clears a copy whose
    /// worktree git could not take away.
    directory: TempDir,
}

impl Worktree {
    /// Creates a copy of the project and syncs the changed files into it
    ///
    /// The copy checks out the commit that the HEAD of the project names, and
    /// every path that git reports as changed or as untracked travels from the
    /// project into it: a path that the project holds is copied, and a path
    /// that the project holds nothing at is removed. Ignored files are never
    /// reported and never copied, so the build directory of the project stays
    /// where it is and the copy builds from nothing.
    ///
    /// Nothing in the project changes. The copy is a working tree of the
    /// repository, so it shows up in `git worktree list` until the value goes.
    ///
    /// The project must be a git repository, and its root must be the top
    /// level of that repository.
    ///
    /// # Errors
    ///
    /// Returns [`MissingRepository`][missing] when the project is in no
    /// repository and [`NestedProject`][nested] when its root is not the top
    /// level of one. Returns [`GitUnavailable`][git] when git does not run,
    /// [`TemporaryDirectoryUnavailable`][directory] when the system hands out
    /// no directory, and [`WorktreeUnavailable`][worktree] when git creates no
    /// worktree. Returns [`UnreadableStatus`][status] and
    /// [`UnsyncedPath`][path] when the sync cannot read what changed or cannot
    /// carry a path over. No error leaves a copy behind.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// git, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_worktree::Worktree;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let worktree = Worktree::create("/home/otter/project".into()).await?;
    ///
    /// println!("{}", worktree.root());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [directory]: CreateWorktreeError::TemporaryDirectoryUnavailable
    /// [git]: CreateWorktreeError::GitUnavailable
    /// [missing]: CreateWorktreeError::MissingRepository
    /// [nested]: CreateWorktreeError::NestedProject
    /// [path]: CreateWorktreeError::UnsyncedPath
    /// [status]: CreateWorktreeError::UnreadableStatus
    /// [worktree]: CreateWorktreeError::WorktreeUnavailable
    // worktree[impl worktree.detached]
    // worktree[impl worktree.temporary]
    // worktree[impl worktree.unavailable]
    // worktree[impl repository.missing]
    // worktree[impl repository.toplevel]
    // worktree[impl project.untouched]
    pub async fn create(project: ProjectRoot) -> Result<Self, CreateWorktreeError> {
        let top_level = top_level(&project).await?;

        if !names_the_same_directory(&top_level, &project) {
            return Err(CreateWorktreeError::NestedProject { project, top_level });
        }

        let directory = TempDir::new().map_err(|source| {
            CreateWorktreeError::TemporaryDirectoryUnavailable {
                project: project.clone(),
                source,
            }
        })?;

        let root = ProjectRoot::new(directory.path().join(CHECKOUT));

        add(&project, &root).await?;

        // From here on the value owns the worktree, so a failure of the sync
        // drops it and git takes the worktree away.
        let worktree = Self {
            project,
            root,
            directory,
        };

        worktree.sync().await?;

        Ok(worktree)
    }

    /// Returns the path that a path of the project has inside the copy
    ///
    /// The copy holds the tree of the project at its own root, so the answer
    /// is the same path relative to the root. A caller that runs a job at a
    /// directory of the project asks for the directory inside the copy, and
    /// keeps reporting its findings at the path of the project.
    ///
    /// A relative path is read against the root of the project, and a path
    /// that climbs through `..` is resolved before the answer.
    ///
    /// Returns `None` when the project does not contain the path. A caller
    /// that reaches outside the project reaches a file that the copy does not
    /// hold, and the copy has no name for it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    ///
    /// use rakko_worktree::Worktree;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let worktree = Worktree::create("/home/otter/project".into()).await?;
    ///
    /// let inside = worktree.path_of(Path::new("crates/example"));
    ///
    /// assert_eq!(inside, Some(worktree.root().get().join("crates/example")));
    /// # Ok(())
    /// # }
    /// ```
    // worktree[impl path.inside]
    // worktree[impl path.foreign]
    // worktree[impl path.parent]
    pub fn path_of(&self, path: &Path) -> Option<PathBuf> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project.get().join(path)
        };

        let relative = strip(&normalize(&absolute), &self.project)?;

        Some(self.root.get().join(relative))
    }

    /// Returns the paths where the tree of the project differs from its commit
    ///
    /// Git names them, and it names no file that it ignores, so the sync never
    /// looks at a build directory.
    ///
    /// # Errors
    ///
    /// Returns an error when git does not run, when it reports no status, and
    /// when it reports a status that the crate cannot read.
    async fn changed(&self) -> Result<Vec<PathBuf>, CreateWorktreeError> {
        let execution = self::git::command(self.project.get())
            .arg(STATUS)
            .arg(PORCELAIN)
            .arg(SEPARATED)
            .arg(UNTRACKED)
            .arg(NO_RENAMES)
            .run()
            .await
            .map_err(|source| CreateWorktreeError::GitUnavailable {
                project: self.project.clone(),
                source,
            })?;

        if !execution.status().success() {
            return Err(CreateWorktreeError::UnreadableStatus {
                project: self.project.clone(),
                details: diagnosis(&execution),
            });
        }

        self::status::changed_paths(execution.stdout().get()).map_err(|details| {
            CreateWorktreeError::UnreadableStatus {
                project: self.project.clone(),
                details,
            }
        })
    }

    /// Carries every changed path of the project into the copy
    ///
    /// # Errors
    ///
    /// Returns an error when the crate cannot read what changed, and when a
    /// path does not reach the copy.
    // worktree[impl sync.changed]
    // worktree[impl sync.ignored]
    // worktree[impl sync.modified]
    async fn sync(&self) -> Result<(), CreateWorktreeError> {
        for path in self.changed().await? {
            self.sync_path(&path)
                .map_err(|source| CreateWorktreeError::UnsyncedPath { path, source })?;
        }

        Ok(())
    }

    /// Carries one changed path of the project into the copy
    ///
    /// A path that the project holds a file at is copied over the path in the
    /// copy, and a path that the project holds nothing at is removed from the
    /// copy. A path that names a directory in the project is a submodule that
    /// moved, and the copy keeps the revision that the commit names.
    ///
    /// # Errors
    ///
    /// Returns an error when the operating system refuses to read the path of
    /// the project, or to write the path of the copy.
    fn sync_path(&self, path: &Path) -> IoResult<()> {
        let source = self.project.get().join(path);
        let target = self.root.get().join(path);

        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return remove(&target),
            Err(error) => return Err(error),
        };

        if metadata.is_dir() {
            return Ok(());
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // The copy can hold a file where the project holds a link, and a
        // write through a link would reach the file that the link names, so
        // the path goes first and the write creates what the project has.
        remove(&target)?;

        write(&source, &target, &metadata)
    }
}

/// Takes the copy away when the value goes
///
/// The removal starts git and waits for it, which holds the thread for as long
/// as git takes. A drop cannot wait for a future, and a copy that outlived its
/// value would stay in `git worktree list` until somebody pruned it.
///
/// A failure is silent, because a drop has nobody to report to. The directory
/// still goes with the value, and `git worktree prune` forgets the entry that
/// it leaves.
impl Drop for Worktree {
    // worktree[impl remove.drop]
    fn drop(&mut self) {
        let _ = self::git::blocking_command(self.project.get())
            .arg(WORKTREE)
            .arg(REMOVE)
            .arg(FORCE)
            .arg(self.root.get())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        // Git takes the checkout away, and the directory that held it goes
        // here, with whatever a git that failed left behind.
        let _ = fs::remove_dir_all(self.directory.path());
    }
}

/// Creates the worktree of the project at the directory that the caller names
///
/// # Errors
///
/// Returns an error when git does not run, and when git runs and creates no
/// worktree.
async fn add(project: &ProjectRoot, root: &ProjectRoot) -> Result<(), CreateWorktreeError> {
    let execution = self::git::command(project.get())
        .arg(WORKTREE)
        .arg(ADD)
        .arg(DETACH)
        .arg(root.get())
        .arg(HEAD)
        .run()
        .await
        .map_err(|source| CreateWorktreeError::GitUnavailable {
            project: project.clone(),
            source,
        })?;

    if execution.status().success() {
        return Ok(());
    }

    Err(CreateWorktreeError::WorktreeUnavailable {
        project: project.clone(),
        details: diagnosis(&execution),
    })
}

/// Returns what git said about a run that ended without success
///
/// Git writes its diagnosis to the standard error stream, and that diagnosis
/// names the condition in the words of git. The text travels into the message
/// of the error, so whoever reads the failure reads the answer of git instead
/// of a sentence that Rakko wrote about it.
fn diagnosis(execution: &Execution) -> String {
    let stderr = execution.stderr().to_string_lossy();
    let text = stderr.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Returns whether the two paths name one directory
///
/// A project root can reach a directory through a symbolic link, and git
/// answers with the directory that it resolved, so the resolved forms decide
/// when the paths differ as text.
fn names_the_same_directory(top_level: &Path, project: &ProjectRoot) -> bool {
    if top_level == project.get() {
        return true;
    }

    match (top_level.canonicalize(), project.get().canonicalize()) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

/// Returns the path with every `.` dropped and every `..` resolved against the
/// component before it
///
/// A check that asks whether a directory contains a path that climbs through
/// its parents answers for the wrong directory, so the climb is resolved
/// first. The resolution is lexical: a component that is a symbolic link
/// resolves as a directory, which is how the caller wrote the path.
// worktree[impl path.parent]
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }

    normalized
}

/// Removes the path from the copy, if the copy holds anything there
///
/// A path that is already gone is the state that the caller asked for, so it
/// is no failure. A path that names a directory belongs to a submodule, which
/// the sync leaves alone.
///
/// # Errors
///
/// Returns an error when the operating system refuses to read the path, or to
/// take the file away.
fn remove(target: &Path) -> IoResult<()> {
    match fs::symlink_metadata(target) {
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => fs::remove_file(target),
    }
}

/// Returns the path without the project root that prefixes it
///
/// The root of a project can name the same directory through a symbolic link,
/// and a caller can hold the directory that the link resolves to, which is why
/// the canonical root is tried as well.
// worktree[impl path.inside]
// worktree[impl path.foreign]
fn strip(path: &Path, project: &ProjectRoot) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(project.get()) {
        return Some(stripped.to_path_buf());
    }

    let canonical = project.get().canonicalize().ok()?;

    path.strip_prefix(canonical).ok().map(Path::to_path_buf)
}

/// Writes what the project holds at the source to the target
///
/// A symbolic link becomes a link to the same target, so the copy holds what
/// the project holds and not the file that the link names.
///
/// # Errors
///
/// Returns an error when the operating system refuses to read the source, or
/// to write the target.
#[cfg(unix)]
fn write(source: &Path, target: &Path, metadata: &Metadata) -> IoResult<()> {
    if metadata.is_symlink() {
        return std::os::unix::fs::symlink(fs::read_link(source)?, target);
    }

    fs::copy(source, target).map(|_| ())
}

/// Writes what the project holds at the source to the target
///
/// A symbolic link becomes the file that it names, because this platform
/// creates a link only for a process that holds a privilege for it.
///
/// # Errors
///
/// Returns an error when the operating system refuses to read the source, or
/// to write the target.
#[cfg(not(unix))]
fn write(source: &Path, target: &Path, _metadata: &Metadata) -> IoResult<()> {
    fs::copy(source, target).map(|_| ())
}

/// Returns the top level of the repository that holds the project
///
/// # Errors
///
/// Returns an error when git does not run, and when git reports that the
/// project is in no repository.
async fn top_level(project: &ProjectRoot) -> Result<PathBuf, CreateWorktreeError> {
    let execution = self::git::command(project.get())
        .arg(REV_PARSE)
        .arg(SHOW_TOPLEVEL)
        .run()
        .await
        .map_err(|source| CreateWorktreeError::GitUnavailable {
            project: project.clone(),
            source,
        })?;

    if !execution.status().success() {
        return Err(CreateWorktreeError::MissingRepository {
            project: project.clone(),
            details: diagnosis(&execution),
        });
    }

    let toplevel = execution.stdout().to_string_lossy();

    toplevel
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| CreateWorktreeError::MissingRepository {
            project: project.clone(),
            details: NO_TOP_LEVEL.to_owned(),
        })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::env;
    use std::process::Output;

    use super::*;

    /// A repository that one test works on
    ///
    /// The tests need a project of their own, because they read and write the
    /// tree of the project and remove it afterwards. A repository of the
    /// machine would carry the configuration of the contributor into the test,
    /// so each repository is a fresh one in a directory that goes with the
    /// value.
    struct Repository {
        /// The directory that holds the repository
        directory: TempDir,
    }

    impl Repository {
        /// Creates a repository with one commit that holds one file
        fn new() -> Self {
            let repository = Self::empty();

            repository.write("tracked.txt", "one");
            repository.write("crates/example/lib.rs", "// example");
            repository.git(&["add", "--all"]);
            repository.git(&["commit", "--message", "The first commit"]);

            repository
        }

        /// Creates a repository without a commit
        fn empty() -> Self {
            let directory = tempfile::tempdir().expect("the test creates a temporary directory");
            let repository = Self { directory };

            repository.git(&["init", "--quiet", "--initial-branch=main", "."]);
            repository.git(&["config", "user.name", "Rakko"]);
            repository.git(&["config", "user.email", "rakko@example.com"]);
            repository.git(&["config", "commit.gpgsign", "false"]);

            repository
        }

        /// Removes a path from the working tree of the repository
        fn delete(&self, path: &str) {
            fs::remove_file(self.root().get().join(path)).expect("the test removes a file");
        }

        /// Runs git in the repository and returns what it produced
        ///
        /// The command forgets the variables that name a repository, so a
        /// test that runs inside a hook of another repository still works on
        /// the one that it created.
        fn git(&self, arguments: &[&str]) -> Output {
            self::git::blocking_command(self.directory.path())
                .args(arguments)
                .output()
                .expect("the test runs git")
        }

        /// Returns whether git reports the tree at the directory as modified
        fn is_modified(directory: &Path) -> bool {
            let output = self::git::blocking_command(directory)
                .arg(STATUS)
                .arg(PORCELAIN)
                .output()
                .expect("the test runs git");

            !output.stdout.is_empty()
        }

        /// Returns what git wrote to its standard output, without the line end
        fn read(&self, arguments: &[&str]) -> String {
            let output = self.git(arguments);

            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }

        /// Returns the root of the repository as a project root
        fn root(&self) -> ProjectRoot {
            ProjectRoot::from(self.directory.path())
        }

        /// Returns the state of the repository that a run must not change
        fn state(&self) -> (String, String, String) {
            (
                self.read(&["rev-parse", HEAD]),
                self.read(&[STATUS, PORCELAIN, UNTRACKED]),
                self.read(&["stash", "list"]),
            )
        }

        /// Writes a file of the working tree, and its parent directories
        fn write(&self, path: &str, content: &str) {
            let file = self.root().get().join(path);

            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent).expect("the test creates a directory");
            }

            fs::write(file, content).expect("the test writes a file");
        }
    }

    /// Returns whether the copy holds the path, and what it holds there
    fn content(worktree: &Worktree, path: &str) -> Option<String> {
        fs::read_to_string(worktree.root().get().join(path)).ok()
    }

    // worktree[verify repository.toplevel]
    #[tokio::test]
    async fn create_below_the_top_level_names_both_directories() {
        let repository = Repository::new();
        let nested = ProjectRoot::from(repository.root().get().join("crates/example").as_path());

        let Err(error) = Worktree::create(nested).await else {
            panic!("expected a project below the top level to stop the run");
        };

        assert!(error.to_string().contains("is not the top level"));
    }

    // worktree[verify worktree.detached]
    #[tokio::test]
    async fn create_checks_out_a_detached_head() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(
            !self::git::blocking_command(worktree.root().get())
                .args(["symbolic-ref", "--quiet", HEAD])
                .output()
                .expect("the test runs git")
                .status
                .success()
        );
    }

    // worktree[verify worktree.detached]
    #[tokio::test]
    async fn create_checks_out_the_commit_of_the_project() {
        let repository = Repository::new();
        let commit = repository.read(&["rev-parse", HEAD]);

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(
            String::from_utf8_lossy(
                &self::git::blocking_command(worktree.root().get())
                    .args(["rev-parse", HEAD])
                    .output()
                    .expect("the test runs git")
                    .stdout
            )
            .trim(),
            commit
        );
    }

    // worktree[verify sync.changed]
    #[tokio::test]
    async fn create_copies_a_changed_file() {
        let repository = Repository::new();
        repository.write("tracked.txt", "two");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "tracked.txt").as_deref(), Some("two"));
    }

    // The copy holds what the project holds, and a link is not the file that
    // it names. A write through the link would reach that file instead, which
    // is the failure that this test rules out.
    #[cfg(unix)]
    #[tokio::test]
    async fn create_copies_a_symbolic_link_as_a_link() {
        let repository = Repository::new();
        std::os::unix::fs::symlink("tracked.txt", repository.root().get().join("link.txt"))
            .expect("the test creates a symbolic link");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(worktree.root().get().join("link.txt").is_symlink());
    }

    // worktree[verify sync.changed]
    #[tokio::test]
    async fn create_copies_a_staged_file() {
        let repository = Repository::new();
        repository.write("staged.txt", "staged");
        repository.git(&["add", "staged.txt"]);

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "staged.txt").as_deref(), Some("staged"));
    }

    // worktree[verify sync.changed]
    #[tokio::test]
    async fn create_copies_an_untracked_file_in_a_new_directory() {
        let repository = Repository::new();
        repository.write("fresh/deep/new.txt", "new");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(
            content(&worktree, "fresh/deep/new.txt").as_deref(),
            Some("new")
        );
    }

    // worktree[verify sync.ignored]
    #[tokio::test]
    async fn create_ignores_a_file_that_git_ignores() {
        let repository = Repository::new();
        repository.write(".gitignore", "target/\n");
        repository.git(&["add", ".gitignore"]);
        repository.git(&["commit", "--message", "Ignore the build directory"]);
        repository.write("target/artifact.bin", "a gigabyte, in spirit");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "target/artifact.bin"), None);
    }

    // worktree[verify sync.modified]
    #[tokio::test]
    async fn create_in_a_clean_project_reports_the_copy_as_unmodified() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(!Repository::is_modified(worktree.root().get()));
    }

    // worktree[verify sync.modified]
    #[tokio::test]
    async fn create_in_a_modified_project_reports_the_copy_as_modified() {
        let repository = Repository::new();
        repository.write("tracked.txt", "two");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(Repository::is_modified(worktree.root().get()));
    }

    // worktree[verify project.untouched]
    #[tokio::test]
    async fn create_leaves_the_project_as_it_was() {
        let repository = Repository::new();
        repository.write("tracked.txt", "two");
        repository.write("untracked.txt", "new");
        repository.git(&["add", "tracked.txt"]);
        let before = repository.state();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");
        drop(worktree);

        assert_eq!(repository.state(), before);
    }

    // worktree[verify worktree.temporary]
    #[tokio::test]
    async fn create_puts_the_copy_outside_the_project() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(!worktree.root().get().starts_with(repository.root().get()));
    }

    // worktree[verify worktree.temporary]
    #[tokio::test]
    async fn create_puts_the_copy_under_the_temporary_directory() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(worktree.root().get().starts_with(env::temp_dir()));
    }

    // worktree[verify sync.changed]
    #[tokio::test]
    async fn create_removes_a_file_that_the_project_deleted() {
        let repository = Repository::new();
        repository.delete("tracked.txt");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "tracked.txt"), None);
    }

    // worktree[verify worktree.unavailable]
    #[tokio::test]
    async fn create_without_a_commit_holds_what_git_wrote() {
        let repository = Repository::empty();

        let Err(error) = Worktree::create(repository.root()).await else {
            panic!("expected a repository without a commit to stop the run");
        };

        assert!(error.to_string().contains("HEAD"));
    }

    // worktree[verify repository.missing]
    #[tokio::test]
    async fn create_without_a_repository_names_the_project() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = ProjectRoot::from(directory.path());

        let Err(error) = Worktree::create(project).await else {
            panic!("expected a project without a repository to stop the run");
        };

        assert!(error.to_string().contains("is not in a git repository"));
    }

    // worktree[verify remove.drop]
    #[tokio::test]
    async fn drop_removes_the_directory_of_the_copy() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");
        let root = worktree.root().get().to_path_buf();

        drop(worktree);

        assert!(!root.exists());
    }

    // worktree[verify remove.drop]
    #[tokio::test]
    async fn drop_removes_the_worktree_of_the_repository() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");
        let root = worktree.root().get().to_path_buf();

        drop(worktree);

        assert!(
            !repository
                .read(&[WORKTREE, "list"])
                .contains(&*root.to_string_lossy())
        );
    }

    // worktree[verify path.foreign]
    #[tokio::test]
    async fn path_of_a_path_outside_the_project_names_nothing() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("/home/otter/elsewhere"));

        assert_eq!(inside, None);
    }

    // worktree[verify path.parent]
    #[tokio::test]
    async fn path_of_a_path_that_climbs_out_of_the_project_names_nothing() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("../elsewhere"));

        assert_eq!(inside, None);
    }

    // worktree[verify path.parent]
    #[tokio::test]
    async fn path_of_a_path_that_climbs_within_the_project_resolves_the_climb() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("crates/example/../other"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/other")));
    }

    // worktree[verify path.inside]
    #[tokio::test]
    async fn path_of_an_absolute_path_of_the_project_names_the_same_path() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(&repository.root().get().join("crates/example"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/example")));
    }

    // worktree[verify path.inside]
    #[tokio::test]
    async fn path_of_a_relative_path_of_the_project_names_the_same_path() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("crates/example"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/example")));
    }

    // A scheduler runs actions in parallel and can move a run to a different
    // thread, so a copy that an action holds across a wait travels with it.
    // This test holds the copy to the auto traits that make this possible,
    // because a field of a later version could take them away without a word
    // from the compiler.
    #[test]
    fn worktree_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<Worktree>();
    }
}
