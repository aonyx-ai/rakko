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
/// A context holds the root directory of the project and the directory layout
/// of that project. Every action receives a context, so the type stays small
/// by design.
// action[impl context.send]
// action[impl context.sync]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Context {
    /// The root directory of the project
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
    // action[impl context.root]
    // action[impl context.layout]
    // action[impl context.derived]
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

    // action[verify context.send]
    #[test]
    fn context_is_send() {
        assert_send::<Context>();
    }

    // action[verify context.sync]
    #[test]
    fn context_is_sync() {
        assert_sync::<Context>();
    }

    // action[verify context.layout]
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

    // action[verify context.root]
    #[test]
    fn root_returns_project_root() {
        let context = Context::builder().root("/tmp/my-project").build();

        assert_eq!(
            context.root().get(),
            &std::path::PathBuf::from("/tmp/my-project")
        );
    }

    // action[verify context.derived]
    #[test]
    fn without_layout_derives_from_root() {
        let context = Context::builder().root("/tmp/my-project").build();
        let default_layout = Layout::builder().root("/tmp/my-project").build();

        assert_eq!(context.layout(), &default_layout);
    }
}
