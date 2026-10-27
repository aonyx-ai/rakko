use crate::file_access::pattern::Pattern;

/// The files of a project that one side of a declaration covers
///
/// A [`FileAccess`] holds two of these: one for the files that a run reads,
/// and one for the files that it writes. Each side covers either the whole
/// project or the files that its patterns match. The whole project is the
/// default, because it is the answer that is always safe.
///
/// The two answers differ for a file that a run creates. The whole project
/// covers every file, also a file that does not exist yet. A list of
/// patterns covers the files that exist and match a pattern, and the one
/// file that each pattern without a wildcard describes.
///
/// # Examples
///
/// ```
/// use rakko_action::{Files, Pattern};
///
/// let formatted = Files::Matching(vec![Pattern::from_static("**/*.toml")]);
///
/// assert_ne!(formatted, Files::Project);
/// ```
///
/// [`FileAccess`]: crate::FileAccess
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub enum Files {
    /// Every file of the project, also the files that do not exist yet
    #[default]
    Project,
    /// The files that match at least one of the patterns
    ///
    /// An empty list covers no file.
    Matching(Vec<Pattern>),
}
