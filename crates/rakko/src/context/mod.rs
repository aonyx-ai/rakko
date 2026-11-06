//! Types for the data that an action reads when it runs

mod layout;
mod project_root;

use bon::bon;
use getset::Getters;

pub use self::layout::{ConfigDirectory, Layout};
pub use self::project_root::ProjectRoot;

/// The data that an action reads when it runs
///
/// A context holds the root, which is the directory where the run happens,
/// and the directory layout of the project. Every action receives a context,
/// so the type stays small by design.
///
/// # Examples
///
/// Create a context for a project.
///
/// ```
/// use rakko::Context;
/// use rakko_test_utils::path;
///
/// let root = path("/tmp/project");
/// let context = Context::builder().root(root.as_path()).build();
/// assert_eq!(context.root().get(), root);
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Context {
    /// The directory where the run of the action happens
    ///
    /// The root can be the directory that the user edits, or a copy of the
    /// project. A copy can lack the files that the project ignores, such as
    /// build outputs and installed packages. [`ProjectRoot`] says what an
    /// action can rely on.
    #[getset(get = "pub")]
    root: ProjectRoot,
    /// The directory layout of the project
    #[getset(get = "pub")]
    layout: Layout,
}

#[bon]
impl Context {
    /// Creates a context from a project root and an optional layout
    ///
    /// When `layout` is absent the context builds the layout from the project
    /// root with default directory paths.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::Context;
    /// use rakko_test_utils::path;
    ///
    /// let root = path("/tmp/project");
    /// let context = Context::builder().root(root.as_path()).build();
    /// assert!(context.layout().config().get().starts_with(&root));
    /// ```
    #[builder]
    pub fn new(#[builder(into)] root: ProjectRoot, layout: Option<Layout>) -> Self {
        let layout = layout.unwrap_or_else(|| Layout::builder().root(root.clone()).build());

        Self { root, layout }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn context_is_send() {
        assert_send::<Context>();
    }

    #[test]
    fn context_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Context>();
    }

    #[test]
    fn context_is_sync() {
        assert_sync::<Context>();
    }

    #[test]
    fn layout_returns_given_layout() {
        let layout = Layout::builder()
            .root("/tmp/my-project")
            .config("/tmp/.config")
            .build();

        let context = Context::builder()
            .root("/tmp/my-project")
            .layout(layout.clone())
            .build();

        assert_eq!(context.layout(), &layout);
    }

    #[test]
    fn root_returns_project_root() {
        let context = Context::builder().root("/tmp/my-project").build();

        assert_eq!(
            context.root().get(),
            &std::path::PathBuf::from("/tmp/my-project")
        );
    }

    #[test]
    fn without_layout_derives_from_root() {
        let context = Context::builder().root("/tmp/my-project").build();
        let default_layout = Layout::builder().root("/tmp/my-project").build();

        assert_eq!(context.layout(), &default_layout);
    }
}
