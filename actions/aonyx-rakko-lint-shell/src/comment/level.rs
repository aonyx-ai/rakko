//! The level that ShellCheck gave a comment
//!
//! ShellCheck gives each of its checks one of four levels. This module holds
//! the level and says nothing about what it means for a run.

use std::fmt;

use serde::Deserialize;

/// The level that ShellCheck gave a comment
///
/// The level belongs to the check and not to the script. A project cannot
/// change it, and its configuration file has no key for a lowest level that
/// counts. A project that wants a check to stay quiet turns that check off.
///
/// A reader of a finding sees the level, because it says how much ShellCheck
/// thinks that the comment matters.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CommentLevel {
    /// The script cannot work, for example because ShellCheck cannot parse it
    Error,

    /// The script works, but probably not as its author wants
    Warning,

    /// The script works, but a small change makes it more robust
    Info,

    /// The script works, but another form is easier to read
    Style,
}

/// The word that ShellCheck writes for a comment of the error level
const ERROR: &str = "error";

/// The word that ShellCheck writes for a comment of the warning level
const WARNING: &str = "warning";

/// The word that ShellCheck writes for a comment of the info level
const INFO: &str = "info";

/// The word that ShellCheck writes for a comment of the style level
const STYLE: &str = "style";

impl fmt::Display for CommentLevel {
    /// Writes the level with the word that ShellCheck uses for it
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Error => ERROR,
            Self::Warning => WARNING,
            Self::Info => INFO,
            Self::Style => STYLE,
        };

        write!(f, "{word}")
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn comment_level_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CommentLevel>();
    }

    #[test]
    fn deserialize_of_a_style_comment_reads_the_style_level() {
        let level = serde_json::from_str::<CommentLevel>("\"style\"").unwrap();

        assert_eq!(level, CommentLevel::Style);
    }

    #[test]
    fn deserialize_of_another_word_fails() {
        let level = serde_json::from_str::<CommentLevel>("\"notice\"");

        assert!(level.is_err(), "expected no level, got {level:?}");
    }

    #[test]
    fn display_of_a_style_writes_the_word_of_shellcheck() {
        let word = CommentLevel::Style.to_string();

        assert_eq!(word, "style");
    }

    #[test]
    fn display_of_a_warning_writes_the_word_of_shellcheck() {
        let word = CommentLevel::Warning.to_string();

        assert_eq!(word, "warning");
    }

    #[test]
    fn display_of_an_error_writes_the_word_of_shellcheck() {
        let word = CommentLevel::Error.to_string();

        assert_eq!(word, "error");
    }

    #[test]
    fn display_of_an_info_writes_the_word_of_shellcheck() {
        let word = CommentLevel::Info.to_string();

        assert_eq!(word, "info");
    }
}
