//! The files that a run of prettier examines
//!
//! Prettier discovers no files of its own: a run names them, and this module
//! holds that name. An action states the extensions of the group that it
//! wraps, and the filter turns them into the pattern that prettier reads and
//! into the question that the look of a project asks about a file.

/// The extension of a file that a run examines
mod extension;

use std::path::Path;

use getset::Getters;

pub use self::extension::FileExtension;

/// The part of a pattern that matches every directory below the root
const EVERY_DIRECTORY: &str = "**/*.";

/// The part of a pattern that matches every extension
const EVERY_EXTENSION: &str = "*";

/// The files that a run of prettier examines
///
/// A filter names extensions, and a run examines the files that carry one of
/// them. A filter that names none examines every file that has an extension,
/// and prettier then skips the files whose language it does not know.
///
/// The filter selects files and nothing else. It excludes no directory,
/// because prettier reads the ignore files of the project, and it changes no
/// option of prettier, because the behavior of the tool comes from the
/// configuration of the project alone.
///
/// # Examples
///
/// An action that wraps the JSON files of a project names two extensions:
///
/// ```
/// use rakko_prettier::{FileExtension, Filter};
///
/// let filter = Filter::new([FileExtension::new("json"), FileExtension::new("json5")]);
///
/// assert_eq!(filter.pattern(), "**/*.{json,json5}");
/// ```
///
/// An action that wraps everything names nothing:
///
/// ```
/// use rakko_prettier::Filter;
///
/// assert_eq!(Filter::any().pattern(), "**/*.*");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Getters)]
pub struct Filter {
    /// The extensions that a run examines, empty for every extension
    #[getset(get = "pub")]
    extensions: Vec<FileExtension>,
}

impl Filter {
    /// Returns the filter that examines every file with an extension
    ///
    /// Prettier skips what it cannot assign to a language, so a run with this
    /// filter covers every language that prettier understands.
    pub fn any() -> Self {
        Self::default()
    }

    /// Returns whether a file carries one of the extensions of the filter
    ///
    /// The comparison respects the case of the extension, because the pattern
    /// that prettier reads does the same. A filter that names no extension
    /// accepts every file that has one.
    pub fn matches(&self, path: &Path) -> bool {
        let Some(extension) = path.extension().and_then(std::ffi::OsStr::to_str) else {
            return false;
        };

        if self.extensions.is_empty() {
            return true;
        }

        self.extensions
            .iter()
            .any(|candidate| candidate.get() == extension)
    }

    /// Returns the filter that examines the files with the given extensions
    ///
    /// An extension stands without its dot. A call without an extension
    /// returns the filter that [`any`][any] returns.
    ///
    /// [any]: Filter::any
    pub fn new(extensions: impl IntoIterator<Item = FileExtension>) -> Self {
        Self {
            extensions: extensions.into_iter().collect(),
        }
    }

    /// Returns the pattern that names the files of the filter to prettier
    ///
    /// Prettier reads a brace list only when it holds more than one entry, so
    /// a single extension stands alone and several stand in braces.
    // prettier[impl select.any]
    // prettier[impl select.extensions]
    pub fn pattern(&self) -> String {
        let mut pattern = String::from(EVERY_DIRECTORY);

        match self.extensions.as_slice() {
            [] => pattern.push_str(EVERY_EXTENSION),
            [only] => pattern.push_str(only.get()),
            several => {
                pattern.push('{');

                for (index, extension) in several.iter().enumerate() {
                    if index > 0 {
                        pattern.push(',');
                    }

                    pattern.push_str(extension.get());
                }

                pattern.push('}');
            }
        }

        pattern
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    /// Returns the filter that names the given extensions
    fn filter(extensions: &[&str]) -> Filter {
        Filter::new(extensions.iter().copied().map(FileExtension::new))
    }

    // prettier[verify select.any]
    #[test]
    fn matches_a_file_of_another_extension_without_an_extension_of_its_own() {
        let matched = Filter::any().matches(&PathBuf::from("sub/a.rs"));

        assert!(matched);
    }

    // prettier[verify select.extensions]
    #[test]
    fn matches_a_file_of_another_extension_reports_no_match() {
        let matched = filter(&["md"]).matches(&PathBuf::from("sub/a.rs"));

        assert!(!matched);
    }

    // prettier[verify select.extensions]
    #[test]
    fn matches_a_file_of_the_extension_reports_a_match() {
        let matched = filter(&["json", "json5"]).matches(&PathBuf::from("sub/a.json5"));

        assert!(matched);
    }

    // prettier[verify select.extensions]
    #[test]
    fn matches_a_file_of_the_extension_in_another_case_reports_no_match() {
        let matched = filter(&["md"]).matches(&PathBuf::from("README.MD"));

        assert!(!matched);
    }

    // prettier[verify select.any]
    #[test]
    fn matches_a_file_without_an_extension_reports_no_match() {
        let matched = Filter::any().matches(&PathBuf::from("justfile"));

        assert!(!matched);
    }

    // prettier[verify select.any]
    #[test]
    fn pattern_without_an_extension_matches_every_extension() {
        let pattern = Filter::any().pattern();

        assert_eq!(pattern, "**/*.*");
    }

    // prettier[verify select.extensions]
    #[test]
    fn pattern_with_one_extension_names_it_alone() {
        let pattern = filter(&["md"]).pattern();

        assert_eq!(pattern, "**/*.md");
    }

    // prettier[verify select.extensions]
    #[test]
    fn pattern_with_several_extensions_names_them_in_braces() {
        let pattern = filter(&["yaml", "yml"]).pattern();

        assert_eq!(pattern, "**/*.{yaml,yml}");
    }
}
