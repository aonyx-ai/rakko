use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rakko_action::{Access, Action, Context, FileList, Files, Outcome};

/// The directories whose changes do not count as a write of the project
///
/// Git guards `.git` with locks of its own. Cargo locks `target` against
/// parallel use. Only tracey reads `.tracey`. The helper leaves out a
/// directory with one of these names at every depth, so it also leaves out
/// the `target` of a nested crate. It cannot know which tool wrote into a
/// `target`, so it leaves that directory out for every tool.
const UNCOUNTED_DIRECTORIES: [&str; 3] = [".git", ".tracey", "target"];

/// The end of the name of a file whose changes do not count as a write
///
/// Only the TypeScript compiler reads its `.tsbuildinfo` files, which it
/// writes during a check of an incremental project.
const UNCOUNTED_SUFFIX: &str = ".tsbuildinfo";

/// What a file of the project holds
#[derive(Clone, Eq, PartialEq, Debug)]
enum Entry {
    /// A regular file, with its content
    File(Vec<u8>),
    /// A symbolic link, with the path that it points to
    Link(PathBuf),
}

/// Runs an action and fails when the run changes a file that its declaration
/// does not cover
///
/// The helper asks the action how the run uses the project, with the given
/// context and arguments. Then it runs the action with the same context and
/// arguments, and it compares the files of the project before and after the
/// run. What the run may change depends on the declaration:
///
/// - A run that declares [`Access::Read`] may change no file.
/// - A run that declares [`Access::Write`] and writes the whole project may
///   change any file, so the helper does not compare the project.
/// - A run that declares [`Access::Write`] and writes the files of patterns
///   may change only the files of its expanded write list. The helper expands
///   the declaration before the run, as a caller that orders runs does. A
///   file that the run creates is therefore outside the list, unless a
///   pattern without a wildcard describes it.
///
/// The helper records the content of every file under the root of the
/// project before and after the run. A file that the run added, removed, or
/// changed counts as a change. The comparison reads contents and not
/// modification times. A tool that rewrites a file with the same bytes
/// therefore passes. The helper never asks the tool which files it examined,
/// because a pattern covers more files than a run examines.
///
/// Some changes do not count as a write of the project, so the helper leaves
/// them out at every depth:
///
/// - a change in a directory named `.git`, which Git guards with locks of
///   its own,
/// - a change in a directory named `target`, which cargo locks itself, and
///   the helper leaves it out for every tool,
/// - a change in a directory named `.tracey`, which only tracey reads, and
/// - a change to a file whose name ends in `.tsbuildinfo`, which only the
///   TypeScript compiler reads.
///
/// The helper returns the outcome of the run, so a test can check what the
/// action found as well.
///
/// # Panics
///
/// Panics when the run changed a file that the declaration does not let it
/// change. The message names each such file relative to the root. Panics as
/// well when a file or a directory of the project cannot be read, because
/// then the comparison has no answer.
///
/// # Examples
///
/// ```
/// # use std::pin::pin;
/// # use std::task::{Context as TaskContext, Poll, Waker};
/// use rakko_action::{Access, Action, Context, Name, Outcome, action_name};
/// use rakko_test_utils::run_checking_access;
///
/// struct LineCount;
///
/// impl Action for LineCount {
///     type Args = ();
///
///     fn name(&self) -> Name {
///         action_name!("line-count")
///     }
///
///     fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
///         Access::Read
///     }
///
///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
///         Outcome::Passed { summary: None }
///     }
/// }
///
/// let directory = tempfile::tempdir().unwrap();
/// let context = Context::builder().root(directory.path()).build();
///
/// let run = run_checking_access(&LineCount, &context, &());
/// # let mut run = pin!(run);
/// # let Poll::Ready(outcome) = run.as_mut().poll(&mut TaskContext::from_waker(Waker::noop())) else {
/// #     panic!("the action does not wait");
/// # };
/// # assert!(matches!(outcome, Outcome::Passed { .. }));
/// ```
pub async fn run_checking_access<A: Action>(
    action: &A,
    context: &Context,
    args: &A::Args,
) -> Outcome {
    let (permitted, complaint) = match action.access(context, args) {
        Access::Read => (
            FileList::default(),
            "declares a read, but its run changed these files of the project",
        ),
        Access::Write => {
            let files = action.files(context, args);
            let Files::Matching(_) = files.writes() else {
                return action.run(context, args).await;
            };

            let lists = files.expand(context.root()).unwrap_or_else(|error| {
                panic!(
                    "failed to expand the files that the action '{}' declares: {error}",
                    action.name()
                )
            });

            (
                lists.writes().clone(),
                "declares the files that it writes, but its run changed these files outside them",
            )
        }
    };

    let root = context.root().get();
    let before = snapshot(root);
    let outcome = action.run(context, args).await;
    let after = snapshot(root);

    let outside: Vec<PathBuf> = changes(&before, &after)
        .into_iter()
        .filter(|path| !permitted.contains(path))
        .collect();
    assert!(
        outside.is_empty(),
        "the action '{}' {complaint}: {}",
        action.name(),
        outside
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );

    outcome
}

/// Returns the paths whose entry differs between two snapshots
fn changes(before: &BTreeMap<PathBuf, Entry>, after: &BTreeMap<PathBuf, Entry>) -> Vec<PathBuf> {
    let paths: BTreeSet<&PathBuf> = before.keys().chain(after.keys()).collect();

    paths
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect()
}

/// Returns every file under the root, by its path relative to the root
///
/// # Panics
///
/// Panics when a directory or a file under the root cannot be read.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Entry> {
    let mut entries = BTreeMap::new();
    record(root, Path::new(""), &mut entries);

    entries
}

/// Records every file under one directory of the project
///
/// The directory is named relative to the root, and so is every file that
/// the function records.
///
/// # Panics
///
/// Panics when the directory or a file in it cannot be read.
fn record(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Entry>) {
    let absolute = root.join(directory);
    let listing = fs::read_dir(&absolute).unwrap_or_else(|error| {
        panic!(
            "failed to list the directory {}: {error}",
            absolute.display()
        )
    });

    for item in listing {
        let item = item.unwrap_or_else(|error| {
            panic!(
                "failed to list the directory {}: {error}",
                absolute.display()
            )
        });
        let path = item.path();
        let relative = directory.join(item.file_name());
        let kind = item.file_type().unwrap_or_else(|error| {
            panic!("failed to read the type of {}: {error}", path.display())
        });

        let name = item.file_name();

        if kind.is_dir() {
            if !UNCOUNTED_DIRECTORIES
                .iter()
                .any(|uncounted| name == *uncounted)
            {
                record(root, &relative, entries);
            }
            continue;
        }

        if name.to_string_lossy().ends_with(UNCOUNTED_SUFFIX) {
            continue;
        }

        let entry = if kind.is_symlink() {
            Entry::Link(fs::read_link(&path).unwrap_or_else(|error| {
                panic!("failed to read the link {}: {error}", path.display())
            }))
        } else {
            Entry::File(fs::read(&path).unwrap_or_else(|error| {
                panic!("failed to read the file {}: {error}", path.display())
            }))
        };

        entries.insert(relative, entry);
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context as TaskContext, Poll, Waker};

    use rakko_action::{FileAccess, Name, Pattern, action_name};
    use tempfile::TempDir;

    use super::*;

    /// An action that declares an access and changes the project as a test
    /// asks
    struct Scribe {
        /// Whether the action declares a read or a write
        access: Access,
        /// The files that the action declares
        files: FileAccess,
        /// What the run does to the root of the project
        effect: fn(&Path),
    }

    impl Action for Scribe {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("scribe")
        }

        fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
            self.access
        }

        fn files(&self, _context: &Context, _args: &Self::Args) -> FileAccess {
            self.files.clone()
        }

        async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
            (self.effect)(context.root().get());

            Outcome::Passed { summary: None }
        }
    }

    /// Drives a future to its result without a runtime
    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let mut task_context = TaskContext::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut task_context) {
                return output;
            }
        }
    }

    /// Returns a project that holds three files: `notes.txt`, `Cargo.toml`,
    /// and `README.md`
    fn project() -> TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("notes.txt"), "otters hold hands").unwrap();
        fs::write(directory.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(directory.path().join("README.md"), "# Otters").unwrap();

        directory
    }

    /// Runs a scribe that covers the whole project and returns the outcome
    fn run(access: Access, effect: fn(&Path)) -> Outcome {
        run_declaring(access, FileAccess::default(), effect)
    }

    /// Runs a scribe against a project and returns the outcome of the run
    fn run_declaring(access: Access, files: FileAccess, effect: fn(&Path)) -> Outcome {
        let directory = project();
        let context = Context::builder().root(directory.path()).build();
        let scribe = Scribe {
            access,
            files,
            effect,
        };

        block_on(run_checking_access(&scribe, &context, &()))
    }

    /// Returns a declaration that writes the files of the patterns
    fn writing<const N: usize>(patterns: [&'static str; N]) -> FileAccess {
        FileAccess::builder()
            .writes(Files::Matching(
                patterns.into_iter().map(Pattern::from_static).collect(),
            ))
            .build()
    }

    #[test]
    #[should_panic(expected = "failed to list the directory")]
    fn run_checking_access_fails_a_reader_in_a_project_that_cannot_be_read() {
        let directory = tempfile::tempdir().unwrap();
        let context = Context::builder()
            .root(directory.path().join("missing").as_path())
            .build();
        let scribe = Scribe {
            access: Access::Read,
            files: FileAccess::default(),
            effect: |_root| {},
        };

        block_on(run_checking_access(&scribe, &context, &()));
    }

    #[test]
    #[should_panic(expected = "notes.txt")]
    fn run_checking_access_fails_a_reader_that_changes_a_file() {
        run(Access::Read, |root| {
            fs::write(root.join("notes.txt"), "otters sleep afloat").unwrap();
        });
    }

    #[test]
    #[should_panic(
        expected = "declares a read, but its run changed these files of the project: Cargo.toml"
    )]
    fn run_checking_access_fails_a_reader_that_changes_a_file_of_its_write_side() {
        run_declaring(Access::Read, writing(["**/*.toml"]), |root| {
            fs::write(root.join("Cargo.toml"), "[workspace]").unwrap();
        });
    }

    #[cfg(unix)]
    #[test]
    #[should_panic(expected = "latest.txt")]
    fn run_checking_access_fails_a_reader_that_points_a_link_elsewhere() {
        let directory = project();
        fs::write(directory.path().join("copy.txt"), "otters hold hands").unwrap();
        std::os::unix::fs::symlink("notes.txt", directory.path().join("latest.txt")).unwrap();
        let context = Context::builder().root(directory.path()).build();
        let scribe = Scribe {
            access: Access::Read,
            files: FileAccess::default(),
            effect: |root| {
                fs::remove_file(root.join("latest.txt")).unwrap();
                std::os::unix::fs::symlink("copy.txt", root.join("latest.txt")).unwrap();
            },
        };

        block_on(run_checking_access(&scribe, &context, &()));
    }

    #[test]
    #[should_panic(expected = "notes.txt")]
    fn run_checking_access_fails_a_reader_that_removes_a_file() {
        run(Access::Read, |root| {
            fs::remove_file(root.join("notes.txt")).unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "draft.txt")]
    fn run_checking_access_fails_a_reader_that_writes_a_file() {
        run(Access::Read, |root| {
            fs::write(root.join("draft.txt"), "otters use tools").unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "draft.txt")]
    fn run_checking_access_fails_a_reader_that_writes_a_file_in_a_directory() {
        run(Access::Read, |root| {
            fs::create_dir(root.join("src")).unwrap();
            fs::write(root.join("src").join("draft.txt"), "otters use tools").unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "failed to expand the files")]
    fn run_checking_access_fails_a_writer_of_patterns_in_a_project_that_cannot_be_read() {
        let directory = tempfile::tempdir().unwrap();
        let context = Context::builder()
            .root(directory.path().join("missing").as_path())
            .build();
        let scribe = Scribe {
            access: Access::Write,
            files: writing(["**/*.toml"]),
            effect: |_root| {},
        };

        block_on(run_checking_access(&scribe, &context, &()));
    }

    #[test]
    #[should_panic(
        expected = "declares the files that it writes, but its run changed these files outside them: README.md"
    )]
    fn run_checking_access_fails_a_writer_of_patterns_that_changes_a_file_outside_them() {
        run_declaring(Access::Write, writing(["**/*.toml"]), |root| {
            fs::write(root.join("README.md"), "# Sea otters").unwrap();
        });
    }

    #[test]
    #[should_panic(expected = "Pipfile.toml")]
    fn run_checking_access_fails_a_writer_of_patterns_that_creates_a_file_that_a_wildcard_matches()
    {
        run_declaring(Access::Write, writing(["**/*.toml"]), |root| {
            fs::write(root.join("Pipfile.toml"), "[packages]").unwrap();
        });
    }

    #[test]
    fn run_checking_access_passes_a_reader_that_rewrites_a_file_unchanged() {
        let outcome = run(Access::Read, |root| {
            fs::write(root.join("notes.txt"), "otters hold hands").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_reader_that_writes_a_build_info_file() {
        let outcome = run(Access::Read, |root| {
            fs::write(root.join("tsconfig.tsbuildinfo"), "{}").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_reader_that_writes_into_git() {
        let outcome = run(Access::Read, |root| {
            fs::create_dir(root.join(".git")).unwrap();
            fs::write(root.join(".git").join("index"), "staged").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_reader_that_writes_into_target() {
        let outcome = run(Access::Read, |root| {
            fs::create_dir_all(root.join("tools").join("target")).unwrap();
            fs::write(root.join("tools").join("target").join("build"), "built").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_reader_that_writes_into_tracey() {
        let outcome = run(Access::Read, |root| {
            fs::create_dir(root.join(".tracey")).unwrap();
            fs::write(root.join(".tracey").join("state"), "traced").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_reader_that_writes_nothing() {
        let outcome = run(Access::Read, |_root| {});

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_writer_of_patterns_that_changes_a_file_of_them() {
        let outcome = run_declaring(Access::Write, writing(["**/*.toml"]), |root| {
            fs::write(root.join("Cargo.toml"), "[workspace]").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_writer_of_patterns_that_creates_the_file_of_a_pattern_without_a_wildcard()
     {
        let outcome = run_declaring(Access::Write, writing(["CHANGELOG.md"]), |root| {
            fs::write(root.join("CHANGELOG.md"), "# Changes").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_writer_of_patterns_that_writes_into_git() {
        let outcome = run_declaring(Access::Write, writing(["**/*.toml"]), |root| {
            fs::create_dir(root.join(".git")).unwrap();
            fs::write(root.join(".git").join("index"), "staged").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    #[test]
    fn run_checking_access_passes_a_writer_that_writes_a_file() {
        let outcome = run(Access::Write, |root| {
            fs::write(root.join("draft.txt"), "otters use tools").unwrap();
        });

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }
}
