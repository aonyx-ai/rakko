//! Types for the operations of prettier that an action runs

/// One operation of prettier that an action runs
///
/// Prettier does one job in two ways, and an action wraps one of them. This
/// vocabulary names the ways, so that an action states what it wants and
/// never writes a command line of its own.
///
/// The two operations differ in one thing: whether prettier writes. A report
/// names the files that a rewrite would change and leaves the project alone,
/// so a run that a user started in order to look changes nothing. A rewrite
/// formats what it can and says which files it changed.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_prettier::{Filter, Operation, Prettier};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let prettier = Prettier::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
///
/// // A report changes nothing, so it rewrites no file.
/// let observation = prettier.observe(Operation::Report, &Filter::any()).await?;
/// assert!(observation.rewritten().is_empty());
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum Operation {
    /// Report the files that prettier would change, and rewrite nothing
    #[default]
    Report,

    /// Rewrite the files that prettier can format
    Rewrite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Operation>();
    }
}
