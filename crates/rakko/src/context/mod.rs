//! Types for the data that an action reads when it runs

mod project_root;

use bon::bon;
use getset::Getters;

pub use self::project_root::ProjectRoot;

/// The data that an action reads when it runs
///
/// A context holds the root, which is the directory where the run happens.
/// Every action receives a context, so the type stays small by design. A later
/// release can add data to it, and the builder then takes that data as an
/// optional member.
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
}

#[bon]
impl Context {
    /// Creates a context from a project root
    #[builder]
    pub fn new(#[builder(into)] root: ProjectRoot) -> Self {
        Self { root }
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
    fn root_returns_project_root() {
        let context = Context::builder().root("/tmp/my-project").build();

        assert_eq!(
            context.root().get(),
            &std::path::PathBuf::from("/tmp/my-project")
        );
    }
}
