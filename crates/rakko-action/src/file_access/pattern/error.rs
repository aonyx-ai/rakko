use std::error::Error as StdError;

use thiserror::Error;

/// An error that occurs when parsing a pattern
///
/// A [`Pattern`] accepts only a glob that describes files below the project
/// root. This error reports the text that it refused, and why.
///
/// [`Pattern`]: super::Pattern
#[derive(Debug, Error)]
pub enum ParsePatternError {
    /// A component of the pattern is empty, `.`, or `..`
    ///
    /// A pattern that starts or ends with `/`, or that holds `//`, has an
    /// empty component. A pattern that starts with `/` names a path outside
    /// the project, and so does a pattern that holds `..`. A file below the
    /// root never has a path with an empty or a `.` component, so such a
    /// pattern matches no file.
    #[error("pattern '{pattern}' holds a component that is empty, '.', or '..'")]
    InvalidComponent {
        /// The text that the caller gave
        pattern: String,
    },

    /// The text is not a valid glob
    ///
    /// The source describes the part of the text that does not parse, such as
    /// a `{` without its `}`.
    #[error("failed to parse the pattern '{pattern}' as a glob")]
    InvalidGlob {
        /// The text that the caller gave
        pattern: String,
        /// The reason why the text is not a valid glob
        source: Box<dyn StdError + Send + Sync>,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn invalid_component_reports_the_pattern() {
        let error = ParsePatternError::InvalidComponent {
            pattern: "../notes.md".to_owned(),
        };

        let message = error.to_string();

        assert_eq!(
            message,
            "pattern '../notes.md' holds a component that is empty, '.', or '..'"
        );
    }

    #[test]
    fn invalid_glob_reports_the_pattern() {
        let error = ParsePatternError::InvalidGlob {
            pattern: "**/*.{json".to_owned(),
            source: "unclosed alternate group".into(),
        };

        let message = error.to_string();

        assert_eq!(
            message,
            "failed to parse the pattern '**/*.{json' as a glob"
        );
    }
}
