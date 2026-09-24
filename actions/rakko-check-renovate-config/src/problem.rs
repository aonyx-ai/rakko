//! One problem that the validator reported about a configuration
//!
//! The validator reports errors and warnings for each configuration that it
//! reads, a migration that a configuration needs, and a configuration that it
//! cannot read. This module holds one of those problems: the file, the place
//! in the file where the validator names one, and what the validator wrote.

use std::path::{Component, Path, PathBuf};

use getset::{CopyGetters, Getters};
use rakko_action::{FilePath, Location, Position, ProjectRoot};

/// The text between the topic of a problem and its detail
const TOPIC_CLOSE: &str = ": ";

/// One problem that the validator reported about a configuration
///
/// The path names the file that holds the configuration, as the validator
/// wrote it. The validator starts in the project root, so it writes a path
/// relative to the root. A caller that reports the problem asks for the
/// [relative][relative] path, which drops a `./` that the path starts with.
///
/// The topic and the detail are the words of the validator. The topic names
/// the kind of problem, such as `Configuration Error` for an error or `Config
/// migration necessary` for a migration, and the detail says what the
/// validator found. A reader of a finding therefore reads the answer of the
/// tool, and not one that this crate wrote about it.
///
/// A problem sits at a position only where the validator names one. Most of
/// what the validator reports is about an option and not about a place in the
/// file, and the problem then belongs to the file as a whole.
///
/// [relative]: RenovateProblem::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct RenovateProblem {
    /// The path of the file, as the validator wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The place in the file, where the validator names one
    #[getset(get_copy = "pub")]
    position: Option<Position>,

    /// The kind of the problem, in the words of the validator
    #[getset(get = "pub")]
    topic: String,

    /// What the validator found, in its own words
    #[getset(get = "pub")]
    detail: String,
}

impl RenovateProblem {
    /// Creates a problem from what the validator reported
    pub fn new(path: PathBuf, position: Option<Position>, topic: String, detail: String) -> Self {
        Self {
            path,
            position,
            topic,
            detail,
        }
    }

    /// Returns where the problem is, relative to the project root
    ///
    /// The location is the position that the validator named, or the whole
    /// file where it named none.
    ///
    /// Returns `None` when the root does not contain the file. The validator
    /// starts in the root, so a path that does not fit points at a report that
    /// the caller misread, or at a global configuration that the environment
    /// of the run named outside the project, and the caller decides what to
    /// do about that.
    pub fn location(&self, root: &ProjectRoot) -> Option<Location> {
        let path = self.relative_path(root)?;

        Some(match self.position {
            Some(position) => Location::Position { path, position },
            None => Location::File { path },
        })
    }

    /// Returns the sentence that the validator wrote about the problem
    ///
    /// The sentence starts with the topic and ends with the detail. A problem
    /// without a detail is its topic alone.
    // checkrenovateconfig[impl check.error]
    // checkrenovateconfig[impl check.warning]
    pub fn message(&self) -> String {
        if self.detail.is_empty() {
            return self.topic.clone();
        }

        format!("{}{TOPIC_CLOSE}{}", self.topic, self.detail)
    }

    /// Returns the path of the file, relative to the project root
    ///
    /// Returns `None` when the root does not contain the file.
    pub fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::try_from(strip(&self.path, root)?).ok()
    }
}

/// Returns the path without the prefix that names where the validator started
///
/// The validator starts in the project root, so a path that it writes is
/// relative to the root. A path that the environment of the run named can
/// start with `./`, which names the same file, and the finding drops it,
/// because a reader and a code host expect the plain path. A relative path that
/// climbs above the root with `..` names a file outside the project, and the
/// answer is `None`.
///
/// A path that arrives absolute loses the project root instead. The root of a
/// context can name the same directory through a symbolic link, which is why
/// the canonical root is tried as well.
fn strip(path: &Path, root: &ProjectRoot) -> Option<PathBuf> {
    if path.is_relative() {
        if path
            .components()
            .any(|component| component == Component::ParentDir)
        {
            return None;
        }

        let plain: PathBuf = path
            .components()
            .filter(|component| *component != Component::CurDir)
            .collect();

        return Some(plain);
    }

    if let Ok(stripped) = path.strip_prefix(root.get()) {
        return Some(stripped.to_path_buf());
    }

    let canonical = root.get().canonicalize().ok()?;

    path.strip_prefix(canonical).ok().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;

    /// Returns an error of the validator about the given path
    fn problem(path: PathBuf) -> RenovateProblem {
        RenovateProblem::new(
            path,
            None,
            "Configuration Error".to_owned(),
            "Invalid configuration option: foo".to_owned(),
        )
    }

    /// The root that the problems of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn location_above_the_root_names_nothing() {
        let location = problem(path("../shared/config.js")).location(&root());

        assert_eq!(location, None);
    }

    #[test]
    fn location_of_a_problem_at_a_position_names_the_position() {
        let position = Position::builder().line(2).column(1).build();
        let problem = RenovateProblem::new(
            path("renovate.json"),
            Some(position),
            "File could not be parsed".to_owned(),
            "JSON5: invalid end of input at 2:1".to_owned(),
        );

        let location = problem.location(&root());

        assert_eq!(
            location,
            Some(Location::Position {
                path: path("renovate.json").try_into().unwrap(),
                position,
            })
        );
    }

    #[test]
    fn location_of_a_problem_without_a_position_names_the_file() {
        let location = problem(path("renovate.json")).location(&root());

        assert_eq!(
            location,
            Some(Location::File {
                path: path("renovate.json").try_into().unwrap(),
            })
        );
    }

    #[test]
    fn location_outside_the_root_names_nothing() {
        let location = problem(path("/home/otter/elsewhere/config.js")).location(&root());

        assert_eq!(location, None);
    }

    // checkrenovateconfig[verify check.error]
    // checkrenovateconfig[verify check.warning]
    #[test]
    fn message_holds_the_topic_and_the_detail() {
        let message = problem(path("renovate.json")).message();

        assert_eq!(
            message,
            "Configuration Error: Invalid configuration option: foo"
        );
    }

    #[test]
    fn message_without_a_detail_is_the_topic() {
        let problem = RenovateProblem::new(
            path("renovate.json"),
            None,
            "Config migration necessary".to_owned(),
            String::new(),
        );

        let message = problem.message();

        assert_eq!(message, "Config migration necessary");
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = problem(path("/home/otter/project/config.js")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("config.js")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_drops_the_place_of_the_run() {
        let relative = problem(path("./config.js")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("config.js")).ok());
    }
}
