use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use globset::{GlobBuilder, GlobMatcher};

/// The error type for pattern parsing
mod error;
pub use self::error::ParsePatternError;

/// The characters that make a pattern match more than one path
///
/// A backslash escapes the character after it, so a pattern that holds one
/// counts as a pattern with wildcards as well.
const WILDCARDS: [char; 5] = ['*', '?', '[', '{', '\\'];

/// The character that separates the components of a pattern
const SEPARATOR: char = '/';

/// A glob that describes files of a project by their path
///
/// A pattern is relative to the project root, and it separates the components
/// of a path with `/` on every platform. It matches the whole path of a file,
/// and not only its name:
///
/// - `*` matches any number of characters inside one component,
/// - `?` matches one character inside one component,
/// - `**` as a whole component matches any number of components, none
///   included,
/// - `[ab]` matches one of the characters between the brackets, and
/// - `{json,json5}` matches one of the alternatives between the braces.
///
/// A backslash escapes the character after it, and a pattern that holds one
/// counts as a pattern with a wildcard. A pattern treats a file whose
/// name starts with a dot like every other file, so `**/*.toml` matches
/// `.taplo.toml` and `.config/rakko.toml`. The comparison is case-sensitive.
///
/// Construct a pattern through [`FromStr`], [`TryFrom<&str>`], or
/// [`TryFrom<String>`]. Every constructor validates the input and returns a
/// [`ParsePatternError`] when the input is not a valid glob, or when a
/// component of it is empty, `.`, or `..`. An action that declares a literal
/// pattern uses [`Pattern::from_static`] instead.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use rakko_action::Pattern;
///
/// let pattern = Pattern::from_static("**/*.toml");
///
/// assert!(pattern.matches(Path::new("crates/rakko-action/Cargo.toml")));
/// assert!(!pattern.matches(Path::new("README.md")));
/// ```
///
/// [`FromStr`]: std::str::FromStr
#[derive(Clone)]
pub struct Pattern {
    /// The glob that the pattern was made from, ready to match paths
    matcher: GlobMatcher,
}

impl Pattern {
    /// Constructs a pattern from a static string
    ///
    /// An action declares its patterns as literals in its code, and a literal
    /// that is not valid is a mistake in that code and not in the project.
    /// The function therefore panics instead of returning an error. A test
    /// that asks the action for its declaration finds the mistake.
    ///
    /// # Panics
    ///
    /// Panics when `input` is not a valid pattern, for the reasons that
    /// [`ParsePatternError`] describes.
    #[track_caller]
    pub fn from_static(input: &'static str) -> Self {
        input.parse().unwrap_or_else(|error| panic!("{error}"))
    }

    /// Returns the text of the pattern
    pub fn get(&self) -> &str {
        self.matcher.glob().glob()
    }

    /// Returns whether the pattern matches the path of a file
    ///
    /// The path is relative to the project root. The method compares text
    /// and does no I/O, so the file does not have to exist. On a platform
    /// that separates the components of a path with another character, such
    /// as Windows, the method reads that character as `/`.
    pub fn matches(&self, path: &Path) -> bool {
        self.matcher.is_match(path)
    }

    /// Returns the path of the one file that the pattern describes, if the
    /// pattern holds no wildcard
    ///
    /// A pattern without a wildcard, such as `.taplo.toml`, describes one
    /// file whether that file exists or not. The answer is that file, by its
    /// path relative to the project root. A pattern with a wildcard returns
    /// `None`, because it describes only the files that exist. A backslash
    /// counts as a wildcard here, so a pattern that escapes a character
    /// returns `None` as well.
    pub fn path(&self) -> Option<PathBuf> {
        let text = self.get();

        if text.contains(WILDCARDS) {
            return None;
        }

        Some(text.split(SEPARATOR).collect())
    }

    /// Checks that every component of `input` names a file or a directory
    /// below the project root
    ///
    /// # Errors
    ///
    /// Returns [`ParsePatternError::InvalidComponent`] when a component of
    /// `input` is empty, `.`, or `..`.
    fn validate(input: &str) -> Result<(), ParsePatternError> {
        let invalid = input
            .split(SEPARATOR)
            .any(|component| component.is_empty() || component == "." || component == "..");

        if invalid {
            return Err(ParsePatternError::InvalidComponent {
                pattern: input.to_owned(),
            });
        }

        Ok(())
    }
}

impl FromStr for Pattern {
    type Err = ParsePatternError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::validate(input)?;

        let glob = GlobBuilder::new(input)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map_err(|source| ParsePatternError::InvalidGlob {
                pattern: input.to_owned(),
                source: Box::new(source),
            })?;

        Ok(Self {
            matcher: glob.compile_matcher(),
        })
    }
}

impl TryFrom<&str> for Pattern {
    type Error = ParsePatternError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        input.parse()
    }
}

impl TryFrom<String> for Pattern {
    type Error = ParsePatternError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        input.parse()
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for Pattern {}

impl fmt::Debug for Pattern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Pattern").field(&self.get()).finish()
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.get())
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
    fn debug_shows_the_text_that_the_pattern_was_made_from() {
        let pattern = Pattern::from_static("**/*.toml");

        assert_eq!(format!("{pattern:?}"), r#"Pattern("**/*.toml")"#);
    }

    #[test]
    fn display_shows_the_text_that_the_pattern_was_made_from() {
        let pattern = Pattern::from_static("**/*.toml");

        assert_eq!(pattern.to_string(), "**/*.toml");
    }

    #[test]
    fn eq_of_patterns_with_other_text_is_false() {
        let pattern = Pattern::from_static("**/*.toml");

        assert_ne!(pattern, Pattern::from_static("**/*.md"));
    }

    #[test]
    #[should_panic(expected = "failed to parse the pattern '**/*.{json' as a glob")]
    fn from_static_with_invalid_glob_panics() {
        Pattern::from_static("**/*.{json");
    }

    #[test]
    fn from_str_accepts_a_valid_pattern() {
        let pattern: Pattern = "**/*.{json,json5}".parse().unwrap();

        assert_eq!(pattern.get(), "**/*.{json,json5}");
    }

    #[test]
    fn from_str_with_current_directory_returns_error() {
        let error = "./mise.toml".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidComponent { .. }));
    }

    #[test]
    fn from_str_with_empty_component_returns_error() {
        let error = "src//lib.rs".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidComponent { .. }));
    }

    #[test]
    fn from_str_with_empty_string_returns_error() {
        let error = "".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidComponent { .. }));
    }

    #[test]
    fn from_str_with_invalid_glob_returns_error() {
        let error = "**/*.{json".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidGlob { .. }));
    }

    #[test]
    fn from_str_with_leading_separator_returns_error() {
        let error = "/etc/hosts".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidComponent { .. }));
    }

    #[test]
    fn from_str_with_parent_directory_returns_error() {
        let error = "../notes.md".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidComponent { .. }));
    }

    #[test]
    fn from_str_with_trailing_separator_returns_error() {
        let error = "src/".parse::<Pattern>().unwrap_err();

        assert!(matches!(error, ParsePatternError::InvalidComponent { .. }));
    }

    #[test]
    fn matches_a_file_in_a_hidden_directory() {
        let pattern = Pattern::from_static("**/*.toml");

        assert!(pattern.matches(&path(".config/rakko.toml")));
    }

    #[test]
    fn matches_a_file_in_the_root_with_a_pattern_for_every_directory() {
        let pattern = Pattern::from_static("**/*.toml");

        assert!(pattern.matches(&path("Cargo.toml")));
    }

    #[test]
    fn matches_a_hidden_file() {
        let pattern = Pattern::from_static("**/*.toml");

        assert!(pattern.matches(&path(".taplo.toml")));
    }

    #[test]
    fn matches_an_escaped_wildcard_as_its_character() {
        let pattern = Pattern::from_static(r"\*.md");

        assert!(pattern.matches(&path("*.md")));
    }

    #[test]
    fn matches_each_alternative() {
        let pattern = Pattern::from_static("**/*.{json,json5}");

        assert!(pattern.matches(&path("config/settings.json5")));
    }

    #[test]
    fn matches_no_file_in_a_directory_with_a_single_star() {
        let pattern = Pattern::from_static("*.toml");

        assert!(!pattern.matches(&path("crates/Cargo.toml")));
    }

    #[test]
    fn matches_no_file_of_another_extension() {
        let pattern = Pattern::from_static("**/*.toml");

        assert!(!pattern.matches(&path("README.md")));
    }

    #[test]
    fn path_of_a_pattern_with_a_wildcard_is_none() {
        let pattern = Pattern::from_static("**/*.toml");

        assert_eq!(pattern.path(), None);
    }

    #[test]
    fn path_of_a_pattern_without_a_wildcard_names_its_file() {
        let pattern = Pattern::from_static(".github/renovate.json");

        assert_eq!(pattern.path(), Some(path(".github/renovate.json")));
    }

    #[test]
    fn try_from_string_accepts_a_valid_pattern() {
        let pattern = Pattern::try_from(String::from(".taplo.toml")).unwrap();

        assert_eq!(pattern.get(), ".taplo.toml");
    }
}
