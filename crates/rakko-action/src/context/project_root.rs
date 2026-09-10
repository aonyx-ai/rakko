use std::io::Result as IoResult;
use std::path::Path;

use crate::path;

typed_fields::path! {
    /// The root directory of the project that the action runs in
    ///
    /// A [`Context`](crate::Context) carries the project root, and the
    /// [`Layout`](crate::Layout) derives its defaults from it. All paths that
    /// an action reads or writes are relative to this directory.
    ProjectRoot
}

impl ProjectRoot {
    /// Returns the root of a project, as the file system names the directory
    ///
    /// The file system resolves the path, so a relative path and a path
    /// through a symbolic link both reach an action as the directory that
    /// they name. A caller that has a path instead of a root uses this rather
    /// than the plain constructor: an action reports every location relative
    /// to the root, and a root that spells a directory differently from the
    /// programs that the action starts leaves every path they report outside
    /// the project.
    ///
    /// # Errors
    ///
    /// Returns the error of the file system when it does not answer for the
    /// path, which is what a path that names no directory produces.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let root = ProjectRoot::canonical(".")?;
    ///
    /// println!("{}", root.get().display());
    /// # Ok(())
    /// # }
    /// ```
    // action[impl path.root]
    pub fn canonical(path: impl AsRef<Path>) -> IoResult<Self> {
        Ok(Self::new(path::canonical(path.as_ref())?))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]
    // The helpers of this module exist only for tests.
    #![allow(clippy::expect_used)]

    use super::*;

    // action[verify path.root]
    #[test]
    fn canonical_of_a_directory_that_exists_names_it() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let root = ProjectRoot::canonical(directory.path()).expect("the directory exists");

        assert!(root.get().is_dir());
    }

    // action[verify path.root]
    #[test]
    fn canonical_of_a_directory_that_does_not_exist_reports_the_error() {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let root = ProjectRoot::canonical(directory.path().join("absent"));

        assert!(root.is_err());
    }
}
