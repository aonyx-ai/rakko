use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The files that one side of a declaration covers, as paths
///
/// The paths are relative to the project root, and the list holds each of
/// them once, in order. A list comes from [`FileAccess::expand`], which looks
/// at the files that exist in the project at that moment. So the list does
/// not change when the project changes afterwards.
///
/// [`FileAccess::expand`]: crate::FileAccess::expand
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct FileList(BTreeSet<PathBuf>);

impl FileList {
    /// Returns whether the list holds a file
    ///
    /// The path is relative to the project root.
    pub fn contains(&self, path: &Path) -> bool {
        self.0.contains(path)
    }

    /// Returns the files of the list in order, relative to the project root
    pub fn iter(&self) -> impl Iterator<Item = &Path> {
        self.0.iter().map(PathBuf::as_path)
    }
}

impl FromIterator<PathBuf> for FileList {
    fn from_iter<I: IntoIterator<Item = PathBuf>>(paths: I) -> Self {
        Self(paths.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    #[test]
    fn contains_a_file_of_the_list() {
        let list: FileList = [path("src/lib.rs")].into_iter().collect();

        assert!(list.contains(&path("src/lib.rs")));
    }

    #[test]
    fn contains_no_file_outside_the_list() {
        let list: FileList = [path("src/lib.rs")].into_iter().collect();

        assert!(!list.contains(&path("src/main.rs")));
    }

    #[test]
    fn iter_returns_each_file_once_in_order() {
        let list: FileList = [path("b.md"), path("a.md"), path("b.md")]
            .into_iter()
            .collect();

        let files: Vec<&Path> = list.iter().collect();

        assert_eq!(files, [path("a.md").as_path(), path("b.md").as_path()]);
    }
}
