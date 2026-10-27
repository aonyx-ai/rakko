/// The error type for the expansion of a declaration
mod error;
/// The files that a declaration covers, as lists of paths
mod file_lists;
/// The files that one side of a declaration covers
mod files;
/// A glob that describes files of a project
mod pattern;

use std::fs;
use std::path::{Path, PathBuf};

use bon::bon;
use getset::Getters;

pub use self::error::ExpandFilesError;
pub use self::file_lists::{FileList, FileLists};
pub use self::files::Files;
pub use self::pattern::{ParsePatternError, Pattern};
use crate::context::ProjectRoot;

/// The files that a run of an action reads and the files that it writes
///
/// An action declares this next to its [`Access`], after its arguments are
/// bound and before it runs, and the declaration does no I/O. [`Access`]
/// says whether the run writes at all. This type says which files the run
/// reads, and which files it writes when it writes. A run that declares
/// [`Access::Read`] writes no file, whatever the write side covers.
///
/// Each side covers the whole project unless the action narrows it to
/// patterns. The patterns describe the files that the tool of the action
/// selects when the project configures no selection, and the files that the
/// action and its tool read as configuration, such as `mise.toml`. Nothing
/// enforces a declaration, and the tool of the action never receives the
/// patterns: it still selects its files itself.
///
/// A caller that orders runs does not compare patterns. It calls
/// [`expand`][FileAccess::expand] and compares the lists of files that exist
/// in the project. A list does not show a file that a run creates, so a run
/// that declares [`Access::Write`] and whose write side covers the whole
/// project conflicts with every other run, whatever the lists hold.
///
/// # Examples
///
/// ```
/// use rakko_action::{FileAccess, Files, Pattern};
///
/// let declaration = FileAccess::builder()
///     .writes(Files::Matching(vec![Pattern::from_static("**/*.toml")]))
///     .build();
///
/// assert_eq!(declaration.reads(), &Files::Project);
/// ```
///
/// [`Access`]: crate::Access
/// [`Access::Read`]: crate::Access::Read
/// [`Access::Write`]: crate::Access::Write
#[derive(Clone, Eq, PartialEq, Debug, Default, Getters)]
pub struct FileAccess {
    /// The files that the run reads
    #[getset(get = "pub")]
    reads: Files,
    /// The files that the run writes, when it writes
    #[getset(get = "pub")]
    writes: Files,
}

#[bon]
impl FileAccess {
    /// Creates a declaration from the files that a run reads and writes
    ///
    /// A side that the caller leaves out covers the whole project.
    #[builder]
    pub fn new(#[builder(default)] reads: Files, #[builder(default)] writes: Files) -> Self {
        Self { reads, writes }
    }

    /// Returns the files of the project that each side covers
    ///
    /// The method lists every file below the root once, and keeps the files
    /// that each side covers. It reads the project as it is: it enters every
    /// directory, also a hidden directory, `.git`, a directory that git
    /// ignores, and a project that is nested inside this one. It does not
    /// follow a symbolic link, and it lists the link itself as a file. A side
    /// that covers the whole project gets every file. A side of patterns gets
    /// the files that match a pattern, and the file that each pattern without
    /// a wildcard describes, whether that file exists or not.
    ///
    /// The answer describes the project at the moment of the call, so it
    /// does not show a file that a run creates later. A side that covers the
    /// whole project also covers such a file, which its list cannot show.
    /// The method reads the whole tree for each call, so it can take seconds
    /// in a large project.
    ///
    /// # Errors
    ///
    /// Returns [`ExpandFilesError::UnreadableDirectory`] when the root or a
    /// directory below it cannot be listed.
    pub fn expand(&self, root: &ProjectRoot) -> Result<FileLists, ExpandFilesError> {
        let project = every_file(root.get())?;

        Ok(FileLists::new(
            select(&self.reads, &project),
            select(&self.writes, &project),
        ))
    }
}

/// Returns the files of a project that one side of a declaration covers
fn select(files: &Files, project: &[PathBuf]) -> FileList {
    let Files::Matching(patterns) = files else {
        return project.iter().cloned().collect();
    };

    let matched = project
        .iter()
        .filter(|path| patterns.iter().any(|pattern| pattern.matches(path)))
        .cloned();
    let described = patterns.iter().filter_map(Pattern::path);

    matched.chain(described).collect()
}

/// Returns every file below the root, by its path relative to the root
///
/// Everything that is not a directory counts as a file, a symbolic link
/// included. The function does not follow a link, so a link to a directory
/// never makes it list a directory twice or loop.
///
/// # Errors
///
/// Returns [`ExpandFilesError::UnreadableDirectory`] when the root or a
/// directory below it cannot be listed.
fn every_file(root: &Path) -> Result<Vec<PathBuf>, ExpandFilesError> {
    let mut files = Vec::new();
    let mut directories = vec![PathBuf::new()];

    while let Some(directory) = directories.pop() {
        let absolute = root.join(&directory);
        let unreadable = |source| ExpandFilesError::UnreadableDirectory {
            directory: absolute.clone(),
            source,
        };

        for entry in fs::read_dir(&absolute).map_err(unreadable)? {
            let entry = entry.map_err(unreadable)?;
            let relative = directory.join(entry.file_name());

            if entry.file_type().map_err(unreadable)?.is_dir() {
                directories.push(relative);
            } else {
                files.push(relative);
            }
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;
    use tempfile::TempDir;

    use super::*;

    /// The files of the fixture, by their path relative to its root
    ///
    /// The fixture holds TOML files in the root, in a hidden directory, and in
    /// a directory that its `.gitignore` names, next to files of other types.
    const FIXTURE: [&str; 7] = [
        ".config/rakko.toml",
        ".git/config",
        ".gitignore",
        "Cargo.toml",
        "README.md",
        "src/lib.rs",
        "target/debug/build.toml",
    ];

    /// Returns a project that holds the files of the fixture
    fn fixture() -> TempDir {
        let directory = tempfile::tempdir().unwrap();

        for file in FIXTURE {
            let file = directory.path().join(path(file));
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, "").unwrap();
        }

        fs::write(directory.path().join(".gitignore"), "/target\n").unwrap();

        directory
    }

    /// Expands a declaration in the fixture
    fn expand(declaration: &FileAccess) -> FileLists {
        let directory = fixture();
        let root = ProjectRoot::new(directory.path().to_path_buf());

        declaration.expand(&root).unwrap()
    }

    /// Returns a list of the files that the literals name
    fn list<const N: usize>(files: [&str; N]) -> FileList {
        files.into_iter().map(path).collect()
    }

    /// Returns a side of a declaration that covers the files of the patterns
    fn matching<const N: usize>(patterns: [&'static str; N]) -> Files {
        Files::Matching(patterns.into_iter().map(Pattern::from_static).collect())
    }

    #[test]
    fn builder_without_reads_reads_the_whole_project() {
        let declaration = FileAccess::builder()
            .writes(matching(["**/*.toml"]))
            .build();

        assert_eq!(declaration.reads(), &Files::Project);
    }

    #[test]
    fn builder_without_writes_writes_the_whole_project() {
        let declaration = FileAccess::builder().reads(matching(["**/*.toml"])).build();

        assert_eq!(declaration.writes(), &Files::Project);
    }

    #[test]
    fn expand_enters_a_nested_project() {
        let directory = tempfile::tempdir().unwrap();
        for file in [
            ".config/rakko.toml",
            "demo/.config/rakko.toml",
            "demo/Cargo.toml",
        ] {
            let file = directory.path().join(path(file));
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, "").unwrap();
        }
        let root = ProjectRoot::new(directory.path().to_path_buf());
        let declaration = FileAccess::builder()
            .writes(matching(["**/*.toml"]))
            .build();

        let lists = declaration.expand(&root).unwrap();

        assert_eq!(
            lists.writes(),
            &list([
                ".config/rakko.toml",
                "demo/.config/rakko.toml",
                "demo/Cargo.toml"
            ])
        );
    }

    #[test]
    fn expand_in_a_root_that_cannot_be_listed_returns_error() {
        let directory = tempfile::tempdir().unwrap();
        let root = ProjectRoot::new(directory.path().join("absent"));

        let error = FileAccess::default().expand(&root).unwrap_err();

        assert!(matches!(
            error,
            ExpandFilesError::UnreadableDirectory { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn expand_lists_a_link_to_a_directory_as_a_file() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs").join("guide.md"), "").unwrap();
        std::os::unix::fs::symlink("docs", directory.path().join("manual")).unwrap();
        let root = ProjectRoot::new(directory.path().to_path_buf());

        let lists = FileAccess::default().expand(&root).unwrap();

        assert_eq!(lists.reads(), &list(["docs/guide.md", "manual"]));
    }

    #[test]
    fn expand_of_a_pattern_without_a_wildcard_lists_its_file_when_it_is_absent() {
        let declaration = FileAccess::builder()
            .writes(matching(["**/*.toml", "CHANGELOG.md"]))
            .build();

        let lists = expand(&declaration);

        assert_eq!(
            lists.writes(),
            &list([
                ".config/rakko.toml",
                "CHANGELOG.md",
                "Cargo.toml",
                "target/debug/build.toml",
            ])
        );
    }

    #[test]
    fn expand_of_no_pattern_lists_no_file() {
        let declaration = FileAccess::builder()
            .writes(Files::Matching(vec![]))
            .build();

        let lists = expand(&declaration);

        assert_eq!(lists.writes(), &FileList::default());
    }

    #[test]
    fn expand_of_patterns_lists_the_files_that_they_match() {
        let declaration = FileAccess::builder()
            .writes(matching(["**/*.toml"]))
            .build();

        let lists = expand(&declaration);

        assert_eq!(
            lists.writes(),
            &list([
                ".config/rakko.toml",
                "Cargo.toml",
                "target/debug/build.toml",
            ])
        );
    }

    #[test]
    fn expand_of_the_whole_project_lists_every_file() {
        let declaration = FileAccess::default();

        let lists = expand(&declaration);

        assert_eq!(lists.reads(), &list(FIXTURE));
    }

    #[test]
    fn expand_of_two_sides_lists_the_files_of_each() {
        let declaration = FileAccess::builder()
            .reads(matching(["**/*.md"]))
            .writes(matching(["src/**"]))
            .build();

        let lists = expand(&declaration);

        assert_eq!(
            lists,
            FileLists::new(list(["README.md"]), list(["src/lib.rs"]))
        );
    }
}
