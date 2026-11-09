//! The disposable copy of a project that a job runs in
//!
//! A copy is a detached checkout of the commit that the HEAD of the project
//! names. The changed files of the checkout are synced over it. The copy
//! therefore holds what the contributor holds , and a job that writes into it
//! changes nothing that the contributor can see.
//!
//! The value owns the copy. It lives for as long as the jobs that run in it,
//! and the drop takes the copy with it.

mod error;
mod git;
mod status;

use std::fs;
use std::fs::Metadata;
use std::io::ErrorKind;
use std::io::Result as IoResult;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;

use getset::Getters;
use kawauso_process::Execution;
use rakko::ProjectRoot;
use rakko::path::canonical;
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
/// does not. So the two entries of a rename need no rule of their own.
const NO_RENAMES: &str = "--no-renames";

/// The name of the checkout inside the directory that the crate creates
///
/// Git names the entry of a worktree after this directory. It makes the name
/// unique when a repository already holds one. So two copies of one project can
/// live at the same time.
const CHECKOUT: &str = "worktree";

/// The details of a failure that git did not explain
const NO_DIAGNOSIS: &str = "git wrote nothing about it";

/// The details of a run of git that ended with success and named no top level
const NO_TOP_LEVEL: &str = "git named no top level";

/// A disposable copy of a project that a job runs in
///
/// The copy is a git worktree in a directory of the temporary directory of the
/// system. It holds the commit that the HEAD of the project names, and the
/// files that the contributor changed on top of it. So a job that runs there
/// reads the tree that the contributor is working on.
///
/// The value owns the copy, and the drop removes it. A caller therefore holds
/// the value for as long as the jobs run, and lets it go when the answers are
/// in.
///
/// The copy is a checkout and not a sandbox. A job that writes outside of it
/// writes into the project, and nothing here stops that.
///
/// # Examples
///
/// ```no_run
/// use rakko::ProjectRoot;
/// use rakko_worktree::Worktree;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let project = ProjectRoot::new(std::env::temp_dir());
/// let worktree = Worktree::create(project.clone()).await?;
///
/// assert_ne!(worktree.root(), &project);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Getters)]
pub struct Worktree {
    /// The root of the project that the copy belongs to
    project: ProjectRoot,

    /// The root of the copy
    ///
    /// The copy holds the tree of the project at this directory. So it is a
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
    /// The copy checks out the commit that the HEAD of the project names.
    /// Every path that git reports as changed or as untracked travels from the
    /// project into it. The crate copies a path that the project holds. It
    /// removes a path that the project holds nothing at. It also removes each
    /// directory that such a removal leaves empty and that the project no
    /// longer holds. Ignored files are never reported and never copied, so
    /// the build directory of the project stays where it is and the copy
    /// builds from nothing.
    ///
    /// Nothing in the project changes. The copy is a working tree of the
    /// repository, so it shows up in `git worktree list` until the value goes.
    ///
    /// The project must be a git repository, and its root must be the top
    /// level of that repository.
    ///
    /// Once the worktree exists, the value owns it. A failure of the sync then
    /// drops the value, and git takes the worktree away.
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
    /// The copy holds the tree of the project at its own root, so the answer is
    /// the same path relative to the root. A caller that runs a job at a
    /// directory of the project asks for the directory inside the copy. The
    /// caller keeps reporting its findings at the path of the project.
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
    /// copy. A path that the project holds nothing at is removed from the copy.
    /// A path that names a directory in the project is a submodule that moved.
    /// The copy keeps the revision that the commit names.
    ///
    /// Git reports the files under a removed directory, and never the
    /// directory itself. A removal can therefore leave a directory in the copy
    /// that the project no longer holds. So the removal also removes each
    /// parent directory that it left empty. It stops at the first directory
    /// that the project still holds or that still holds an entry in the copy.
    /// It stops at the first directory that the copy does not hold.
    ///
    /// # Errors
    ///
    /// Returns an error when the operating system refuses to read the path of
    /// the project, or to write the path of the copy. A removal also fails
    /// when the operating system refuses to read a parent directory of the
    /// path, in the project or in the copy. It fails as well when the
    /// operating system refuses to remove a parent directory from the copy.
    fn sync_path(&self, path: &Path) -> IoResult<()> {
        let source = self.project.get().join(path);
        let target = self.root.get().join(path);

        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                remove(&target)?;

                return self.prune(path);
            }
            Err(error) => return Err(error),
        };

        if metadata.is_dir() {
            return Ok(());
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        remove(&target)?;

        write(&source, &target, &metadata)
    }

    /// Removes the parent directories of a removed path that the project no
    /// longer holds
    ///
    /// The walk climbs from the parent of the path, one directory at a time.
    /// It stops at the first directory that the project still holds or that
    /// still holds an entry in the copy. It stops at the first directory that
    /// the copy does not hold. The project always holds its root, so the copy
    /// keeps its own.
    ///
    /// A directory that the project still holds stays even when it is empty,
    /// because the copy holds what the project holds. A directory that still
    /// holds an entry in the copy stays as well. If the project no longer
    /// holds that entry, the sync removes it later, and the walk starts again.
    ///
    /// # Errors
    ///
    /// Returns an error when the operating system refuses to read a directory
    /// of the project or of the copy. It fails when the operating system
    /// refuses to remove a directory of the copy.
    fn prune(&self, path: &Path) -> IoResult<()> {
        for parent in path.ancestors().skip(1) {
            match fs::symlink_metadata(self.project.get().join(parent)) {
                Ok(_) => return Ok(()),
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }

            let directory = self.root.get().join(parent);

            let mut entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            };

            if entries.next().is_some() {
                return Ok(());
            }

            fs::remove_dir(&directory)?;
        }

        Ok(())
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
    /// Takes the checkout and then its directory away
    ///
    /// Git takes the checkout away, and the method removes the directory that
    /// held it, with whatever a git that failed left behind.
    fn drop(&mut self) {
        let _ = self::git::blocking_command(self.project.get())
            .arg(WORKTREE)
            .arg(REMOVE)
            .arg(FORCE)
            .arg(self.root.get())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

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
/// of the error. So whoever reads the failure reads the answer of git instead
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
/// A project root can reach a directory through a symbolic link. Git answers
/// with the directory that it resolved. So the resolved forms decide when the
/// paths differ as text.
fn names_the_same_directory(top_level: &Path, project: &ProjectRoot) -> bool {
    if top_level == project.get() {
        return true;
    }

    match (canonical(top_level), canonical(project.get())) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

/// Returns the path with every `.` dropped and every `..` resolved against the
/// component before it
///
/// A check that asks whether a directory contains a path that climbs through
/// its parents answers for the wrong directory. So the climb is resolved first.
/// The resolution is lexical: a component that is a symbolic link resolves as a
/// directory, which is how the caller wrote the path.
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
/// The root of a project can name the same directory through a symbolic link. A
/// caller can hold the directory that the link resolves to. So the canonical
/// root is tried as well.
fn strip(path: &Path, project: &ProjectRoot) -> Option<PathBuf> {
    if let Ok(stripped) = path.strip_prefix(project.get()) {
        return Some(stripped.to_path_buf());
    }

    let resolved = canonical(project.get()).ok()?;

    path.strip_prefix(resolved).ok().map(Path::to_path_buf)
}

/// Writes what the project holds at the source to the target
///
/// A symbolic link becomes a link to the same target. So the copy holds what
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::env;
    use std::process::Output;

    use super::*;

    /// A repository that one test works on
    ///
    /// The tests need a project of their own, because they read and write the
    /// tree of the project and remove it afterwards. A repository of the
    /// machine would carry the configuration of the contributor into the test.
    /// So each repository is a fresh one in a directory that goes with the
    /// value.
    struct Repository {
        /// The directory that holds the repository
        directory: TempDir,
    }

    #[bon::bon]
    impl Repository {
        /// Creates a repository with one commit that holds one file
        fn new() -> Self {
            let repository = Self::empty();

            repository.write().path("tracked.txt").content("one").call();
            repository
                .write()
                .path("crates/example/lib.rs")
                .content("// example")
                .call();
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

        /// Removes a directory and everything in it from the working tree
        fn delete_directory(&self, path: &str) {
            fs::remove_dir_all(self.root().get().join(path)).expect("the test removes a directory");
        }

        /// Runs git in the repository and returns what it produced
        ///
        /// The command forgets the variables that name a repository. So a test
        /// that runs inside a hook of another repository still works on the one
        /// that it created.
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
        #[builder]
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

    #[tokio::test]
    async fn create_below_the_top_level_names_both_directories() {
        let repository = Repository::new();
        let nested = ProjectRoot::from(repository.root().get().join("crates/example").as_path());

        let Err(error) = Worktree::create(nested).await else {
            panic!("expected a project below the top level to stop the run");
        };

        assert!(error.to_string().contains("is not the top level"));
    }

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

    #[tokio::test]
    async fn create_copies_a_changed_file() {
        let repository = Repository::new();
        repository.write().path("tracked.txt").content("two").call();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "tracked.txt").as_deref(), Some("two"));
    }

    #[tokio::test]
    async fn create_copies_a_staged_file() {
        let repository = Repository::new();
        repository
            .write()
            .path("staged.txt")
            .content("staged")
            .call();
        repository.git(&["add", "staged.txt"]);

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "staged.txt").as_deref(), Some("staged"));
    }

    /// Preserves symbolic links when copying a worktree
    ///
    /// The copy holds what the project holds, and a link is not the file that
    /// it names. A write through the link would reach that file instead, which
    /// is the failure that this test rules out.
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

    #[tokio::test]
    async fn create_copies_an_untracked_file_in_a_new_directory() {
        let repository = Repository::new();
        repository
            .write()
            .path("fresh/deep/new.txt")
            .content("new")
            .call();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(
            content(&worktree, "fresh/deep/new.txt").as_deref(),
            Some("new")
        );
    }

    #[tokio::test]
    async fn create_ignores_a_file_that_git_ignores() {
        let repository = Repository::new();
        repository
            .write()
            .path(".gitignore")
            .content("target/\n")
            .call();
        repository.git(&["add", ".gitignore"]);
        repository.git(&["commit", "--message", "Ignore the build directory"]);
        repository
            .write()
            .path("target/artifact.bin")
            .content("a gigabyte, in spirit")
            .call();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "target/artifact.bin"), None);
    }

    #[tokio::test]
    async fn create_in_a_clean_project_reports_the_copy_as_unmodified() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(!Repository::is_modified(worktree.root().get()));
    }

    #[tokio::test]
    async fn create_in_a_modified_project_reports_the_copy_as_modified() {
        let repository = Repository::new();
        repository.write().path("tracked.txt").content("two").call();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(Repository::is_modified(worktree.root().get()));
    }

    /// Shows that the copy keeps a directory the project still holds
    ///
    /// The checkout still holds the directory, empty, and the copy holds what
    /// the checkout holds. A workspace glob that matches the directory then
    /// fails in the copy as it fails in the checkout.
    #[tokio::test]
    async fn create_keeps_a_directory_that_the_project_still_holds() {
        let repository = Repository::new();
        repository.delete("crates/example/lib.rs");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(worktree.root().get().join("crates/example").is_dir());
    }

    #[tokio::test]
    async fn create_leaves_the_project_as_it_was() {
        let repository = Repository::new();
        repository.write().path("tracked.txt").content("two").call();
        repository
            .write()
            .path("untracked.txt")
            .content("new")
            .call();
        repository.git(&["add", "tracked.txt"]);
        let before = repository.state();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");
        drop(worktree);

        assert_eq!(repository.state(), before);
    }

    #[tokio::test]
    async fn create_puts_the_copy_outside_the_project() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(!worktree.root().get().starts_with(repository.root().get()));
    }

    #[tokio::test]
    async fn create_puts_the_copy_under_the_temporary_directory() {
        let repository = Repository::new();

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(worktree.root().get().starts_with(env::temp_dir()));
    }

    /// Shows that the copy removes a directory the project deleted
    ///
    /// A workspace glob such as `crates/*` matches every directory. So a member
    /// that the contributor removed must not stay in the copy as an empty
    /// directory without its manifest. The directory holds two files, so the
    /// walk after the first removal meets a directory that is not empty yet.
    /// The removal must not fail on it.
    #[tokio::test]
    async fn create_removes_a_directory_that_the_project_deleted() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/example/Cargo.toml")
            .content("[package]")
            .call();
        repository.git(&["add", "--all"]);
        repository.git(&["commit", "--message", "Add a manifest"]);
        repository.delete_directory("crates/example");

        let exists = Worktree::create(repository.root())
            .await
            .map(|worktree| worktree.root().get().join("crates/example").exists());

        assert_eq!(exists.ok(), Some(false));
    }

    #[tokio::test]
    async fn create_removes_a_file_that_the_project_deleted() {
        let repository = Repository::new();
        repository.delete("tracked.txt");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert_eq!(content(&worktree, "tracked.txt"), None);
    }

    /// Shows that the copy removes a staged file whose directory is gone
    ///
    /// A staged file in a new directory that the contributor then deleted is
    /// reported as removed, and the copy never held its directory. The walk
    /// must stop there without an error.
    #[tokio::test]
    async fn create_removes_a_staged_file_whose_directory_the_project_deleted() {
        let repository = Repository::new();
        repository
            .write()
            .path("fresh/deep/new.txt")
            .content("new")
            .call();
        repository.git(&["add", "fresh/deep/new.txt"]);
        repository.delete_directory("fresh");

        let exists = Worktree::create(repository.root())
            .await
            .map(|worktree| worktree.root().get().join("fresh").exists());

        assert_eq!(exists.ok(), Some(false));
    }

    #[tokio::test]
    async fn create_removes_the_parents_that_the_project_deleted() {
        let repository = Repository::new();
        repository.delete_directory("crates");

        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        assert!(!worktree.root().get().join("crates").exists());
    }

    #[tokio::test]
    async fn create_without_a_commit_holds_what_git_wrote() {
        let repository = Repository::empty();

        let Err(error) = Worktree::create(repository.root()).await else {
            panic!("expected a repository without a commit to stop the run");
        };

        assert!(error.to_string().contains("HEAD"));
    }

    #[tokio::test]
    async fn create_without_a_repository_names_the_project() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = ProjectRoot::from(directory.path());

        let Err(error) = Worktree::create(project).await else {
            panic!("expected a project without a repository to stop the run");
        };

        assert!(error.to_string().contains("is not in a git repository"));
    }

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

    #[tokio::test]
    async fn path_of_a_path_outside_the_project_names_nothing() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("/home/otter/elsewhere"));

        assert_eq!(inside, None);
    }

    #[tokio::test]
    async fn path_of_a_path_that_climbs_out_of_the_project_names_nothing() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("../elsewhere"));

        assert_eq!(inside, None);
    }

    #[tokio::test]
    async fn path_of_a_path_that_climbs_within_the_project_resolves_the_climb() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("crates/example/../other"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/other")));
    }

    #[tokio::test]
    async fn path_of_a_relative_path_of_the_project_names_the_same_path() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(Path::new("crates/example"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/example")));
    }

    #[tokio::test]
    async fn path_of_an_absolute_path_of_the_project_names_the_same_path() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");

        let inside = worktree.path_of(&repository.root().get().join("crates/example"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/example")));
    }

    /// Returns the same path when resolved on different platforms
    ///
    /// A caller can hold the directory that the file system resolved. On some
    /// platforms the temporary directory differs from it as text. On macOS the
    /// difference is a symbolic link. On Windows the difference is a short
    /// name. The resolved root must then carry the spelling that the caller
    /// holds, or no path gets a name inside the copy.
    #[tokio::test]
    async fn path_of_an_absolute_path_under_the_resolved_root_names_the_same_path() {
        let repository = Repository::new();
        let worktree = Worktree::create(repository.root())
            .await
            .expect("expected git to create the copy");
        let resolved = canonical(repository.root().get()).expect("the directory exists");

        let inside = worktree.path_of(&resolved.join("crates/example"));

        assert_eq!(inside, Some(worktree.root().get().join("crates/example")));
    }

    /// Shows that a `Worktree` holds auto traits
    ///
    /// A scheduler runs actions in parallel. It can move a run to a different
    /// thread, so a copy that an action holds across a wait travels with it.
    /// This test holds the copy to the auto traits that make this possible. It
    /// does so because a field of a later version could take them away without
    /// a word from the compiler.
    #[test]
    fn worktree_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<Worktree>();
    }

    #[test]
    fn worktree_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Worktree>();
    }
}
