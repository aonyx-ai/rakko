/// The directory layout of a project, with default and overridable paths
mod layout;
/// The root directory of the project
mod project_root;

use bon::bon;
use getset::Getters;

pub use self::layout::{CacheDirectory, ConfigDirectory, Layout};
pub use self::project_root::ProjectRoot;

/// The data that an action reads when it runs
///
/// A context holds the root, which is the directory where the run happens,
/// and the directory layout of the project. Every action receives a context,
/// so the type stays small by design.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Context {
    /// The directory where the run happens
    ///
    /// The root can be a copy of the project and not the tree that the user
    /// edits, and a copy can lack the files that the project ignores. Paths
    /// relative to the root name the same file in a copy and in the project.
    /// [`ProjectRoot`] gives what an action can rely on.
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
    #[builder]
    pub fn new(#[builder(into)] root: ProjectRoot, layout: Option<Layout>) -> Self {
        let layout = layout.unwrap_or_else(|| Layout::builder().root(root.clone()).build());

        Self { root, layout }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn context_is_send() {
        assert_send::<Context>();
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
            .cache("/tmp/cache")
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
