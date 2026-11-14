//! One comment that ShellCheck wrote about a script
//!
//! A run of ShellCheck writes one comment for each problem that a check found
//! in a script. This module holds one of them. It says where the problem is,
//! which check found it, how much it matters, and what ShellCheck said about
//! it.

mod level;

use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko::{FilePath, ProjectRoot, Span};

pub(crate) use self::level::CommentLevel;

/// The character that opens the level of a comment in a message
const LEVEL_OPEN: char = '[';

/// The text that closes the level of a comment in a message
const LEVEL_CLOSE: &str = "] ";

/// The text that opens the code of a check in a message
///
/// ShellCheck writes the code of each check with this prefix. The wiki of
/// ShellCheck has a page for each code, so a reader can look it up.
const CODE_OPEN: &str = " (SC";

/// The character that closes the code of a check in a message
const CODE_CLOSE: char = ')';

/// One comment that ShellCheck wrote about a script
///
/// The path stands as ShellCheck wrote it. ShellCheck writes the path of a
/// script as the command line gave it. A run starts ShellCheck in the project
/// root and gives each script relative to the root. A caller that reports the
/// comment asks for the [relative][relative] path.
///
/// Every comment has a start and an end, so a comment covers a range of the
/// script. ShellCheck counts lines and columns from 1, and a tab counts as one
/// column. The end is the first column after the range.
///
/// The message is the sentence that ShellCheck wrote for a reader. A reader of
/// a finding therefore reads the answer of the tool, and not one that this
/// crate wrote about it.
///
/// [relative]: ShellcheckComment::relative_path
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub(crate) struct ShellcheckComment {
    /// The path of the script, as ShellCheck wrote it
    #[getset(get = "pub(crate)")]
    path: PathBuf,

    /// The range of the script that the comment covers
    #[getset(get_copy = "pub(crate)")]
    span: Span,

    /// How much ShellCheck thinks that the comment matters
    #[getset(get_copy = "pub(crate)")]
    level: CommentLevel,

    /// The number of the check that wrote the comment
    #[getset(get_copy = "pub(crate)")]
    code: u32,

    /// What ShellCheck said about the problem
    #[getset(get = "pub(crate)")]
    description: String,
}

impl ShellcheckComment {
    /// Returns the sentence that ShellCheck wrote about the problem
    ///
    /// The sentence holds the level, the description, and the code of the
    /// check. The code is what a project gives to turn the check off, so the
    /// reader of a finding has it at hand.
    pub(crate) fn message(&self) -> String {
        format!(
            "{LEVEL_OPEN}{}{LEVEL_CLOSE}{}{CODE_OPEN}{}{CODE_CLOSE}",
            self.level, self.description, self.code
        )
    }

    /// Returns the path of the script, relative to the project root
    ///
    /// Returns `None` when the root does not contain the script. A run gives
    /// ShellCheck the scripts of the root. So a path that does not fit points
    /// at a report that the caller misread. The caller decides what to do
    /// about that.
    pub(crate) fn relative_path(&self, root: &ProjectRoot) -> Option<FilePath> {
        FilePath::within().path(&self.path).root(root).call()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::Position;
    use rakko_test_utils::path;

    use super::*;

    /// Returns a comment about the given path
    fn comment(path: PathBuf) -> ShellcheckComment {
        ShellcheckComment::builder()
            .path(path)
            .span(
                Span::builder()
                    .start(Position::builder().line(2).column(6).build())
                    .end(Position::builder().line(2).column(8).build())
                    .build(),
            )
            .level(CommentLevel::Info)
            .code(2086)
            .description("Double quote to prevent globbing and word splitting.".to_owned())
            .build()
    }

    /// The root that the comments of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    #[test]
    fn message_holds_the_level_the_description_and_the_code() {
        let message = comment(path("run.sh")).message();

        assert_eq!(
            message,
            "[info] Double quote to prevent globbing and word splitting. (SC2086)"
        );
    }

    #[test]
    fn relative_path_outside_the_root_gives_nothing() {
        let relative = comment(path("/home/otter/elsewhere/run.sh")).relative_path(&root());

        assert_eq!(relative, None);
    }

    #[test]
    fn relative_path_that_arrived_absolute_drops_the_root() {
        let relative = comment(path("/home/otter/project/bin/run.sh")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("bin/run.sh")).ok());
    }

    #[test]
    fn relative_path_that_arrived_relative_stays_as_it_is() {
        let relative = comment(path("bin/run.sh")).relative_path(&root());

        assert_eq!(relative, FilePath::try_from(path("bin/run.sh")).ok());
    }

    #[test]
    fn shellcheck_comment_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ShellcheckComment>();
    }
}
