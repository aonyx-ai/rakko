//! Types for the data that an action reads when it runs

mod project_root;

use bon::bon;
use getset::Getters;

pub use self::project_root::ProjectRoot;
use crate::tool_output::ToolOutput;

/// The data that an action reads when it runs
///
/// A context holds the root, which is the directory where the run happens. It
/// also holds the [`ToolOutput`] that receives what the tools of the action
/// write while they run. Every action receives a context, so the type stays
/// small by design. A later release can add data to it, and the builder then
/// takes that data as an optional member.
///
/// # Examples
///
/// ```
/// use rakko::{Context, ProjectRoot};
///
/// let root = ProjectRoot::new(std::env::temp_dir());
/// let context = Context::builder().root(root.clone()).build();
///
/// assert_eq!(context.root(), &root);
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

    /// The place where the tools of the action send what they write
    ///
    /// The machinery that starts a tool for the action sends the events of the
    /// tool here. The harness chooses what a reader sees of them.
    #[getset(get = "pub")]
    tool_output: ToolOutput,
}

#[bon]
impl Context {
    /// Creates a context from a project root and a tool output
    ///
    /// A context without a tool output discards what the tools of the action
    /// write. So a caller that runs an action as a library gets no output that
    /// it did not ask for.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::Context;
    ///
    /// let context = Context::builder().root("project").build();
    ///
    /// assert_eq!(context.root().get(), std::path::Path::new("project"));
    /// ```
    #[builder]
    pub fn new(
        #[builder(into)] root: ProjectRoot,
        #[builder(default)] tool_output: ToolOutput,
    ) -> Self {
        Self { root, tool_output }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::tool_output::{ToolCommand, ToolEvent};

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
    fn new_with_a_tool_output_keeps_it() {
        let sent = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&sent);
        let output = ToolOutput::new(move |_event| {
            counted.fetch_add(1, Ordering::SeqCst);
        });
        let context = Context::builder()
            .root("/tmp/my-project")
            .tool_output(output)
            .build();

        context
            .tool_output()
            .send(ToolEvent::Started(ToolCommand::new("taplo check")));

        assert_eq!(sent.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn new_without_a_tool_output_discards() {
        let context = Context::builder().root("/tmp/my-project").build();

        assert!(context.tool_output().discards());
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
