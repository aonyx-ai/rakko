//! One problem that oxfmt reported about a project
//!
//! A run of oxfmt names the files that it could not leave alone and the
//! things that it could not do, and this module holds one of them together
//! with what oxfmt knew about it.

use std::path::{Path, PathBuf};

/// One problem that oxfmt reported about a project
///
/// Oxfmt names a problem as precisely as the operation allows, and a variant
/// carries one of those levels. A list of the files that a rewrite would
/// change knows the file and nothing in it. A file that oxfmt read and could
/// not parse points at the character that broke it. A file that oxfmt could
/// not read at all, or could not write back, has a sentence and no place,
/// because oxfmt never got far enough to have one.
///
/// A path stands as oxfmt wrote it. A run starts oxfmt in the project root, so
/// oxfmt reports a path relative to it, and a caller that reports the problem
/// asks [`FilePath::within`][within] for the path that a finding names.
///
/// [within]: rakko_action::FilePath::within
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum OxfmtProblem {
    /// Oxfmt could not format the file, and it named the place that stopped it
    Invalid {
        /// The path of the file, as oxfmt wrote it
        path: PathBuf,

        /// The line that oxfmt pointed at, starting at 1
        line: u32,

        /// The column that oxfmt pointed at, starting at 1
        column: u32,

        /// What oxfmt said about the file
        message: String,
    },

    /// The file differs from what oxfmt would write
    ///
    /// Oxfmt names the file and nothing in it, so the problem has no position.
    Unformatted {
        /// The path of the file, as oxfmt wrote it
        path: PathBuf,
    },

    /// Oxfmt could not do something, and it named no place for the failure
    ///
    /// A file that oxfmt cannot open and a file that it cannot write back both
    /// end this way. Oxfmt writes the path of the file into the sentence
    /// instead of reporting it as a place, so the message carries it and the
    /// problem belongs to the project.
    Unplaced {
        /// What oxfmt said about the failure
        message: String,
    },
}

impl OxfmtProblem {
    /// Returns the path of the file that the problem is about
    ///
    /// Returns `None` for a problem that oxfmt placed in no file.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Invalid { path, .. } | Self::Unformatted { path } => Some(path),
            Self::Unplaced { .. } => None,
        }
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
    fn path_of_a_failure_without_a_place_names_no_file() {
        let problem = OxfmtProblem::Unplaced {
            message: String::from("Failed to read file: /home/otter/project/src/index.ts"),
        };

        assert_eq!(problem.path(), None);
    }

    #[test]
    fn path_of_an_unformatted_file_stands_as_oxfmt_wrote_it() {
        let problem = OxfmtProblem::Unformatted {
            path: path("src/index.ts"),
        };

        assert_eq!(problem.path(), Some(path("src/index.ts").as_path()));
    }
}
