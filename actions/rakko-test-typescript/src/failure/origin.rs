use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, ProjectRoot};

/// The place in the project where a test that failed was declared
///
/// Node reports the place of a test as the path, the line, and the column of
/// the call that declared it, and not the place where the test broke. The two
/// differ for a test whose body calls a helper, and the declaration is the one
/// that a reader goes to, because it names the test that the report named.
///
/// The path stands as Node wrote it, which is absolute. A caller that reports
/// the failure asks for the [relative][relative] path.
///
/// [relative]: Origin::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct Origin {
    /// The path of the file, as Node wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line that the place points at, starting at 1
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column that the place points at, starting at 1
    #[getset(get_copy = "pub")]
    column: u32,
}

impl Origin {
    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file. A run starts
    /// node in the root and lets node walk down from there, so a path that
    /// does not fit points at a report that the caller misread, and the caller
    /// decides what to do about that.
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within(&self.path, root)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    /// Returns the place of a test in the given file
    fn origin(path: PathBuf) -> Origin {
        Origin::builder().path(path).line(2).column(3).build()
    }

    /// The root that the failures of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn relative_path_of_a_path_below_the_root_keeps_it() {
        let relative = origin(path("src/add.test.ts")).relative_path(&root());

        assert_eq!(relative, Some("src/add.test.ts".parse().unwrap()));
    }

    #[test]
    fn relative_path_of_a_path_outside_the_root_is_none() {
        let relative = origin(path("/elsewhere/add.test.ts")).relative_path(&root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_of_a_path_that_names_the_current_directory_drops_it() {
        let relative = origin(path("./src/add.test.ts")).relative_path(&root());

        assert_eq!(relative, Some("src/add.test.ts".parse().unwrap()));
    }

    #[test]
    fn relative_path_of_an_absolute_path_below_the_root_strips_it() {
        let relative = origin(path("/home/otter/project/src/add.test.ts")).relative_path(&root());

        assert_eq!(relative, Some("src/add.test.ts".parse().unwrap()));
    }
}
