//! The action that checks a project against the newest versions of its
//! dependencies
//!
//! A manifest states a floor for each dependency, and a floor is a promise:
//! every later version of that dependency works as well. The action resolves
//! the newest versions that the manifests allow and runs the tests against
//! them, so the promise is confirmed instead of assumed.
//!
//! [Cargo] resolves, [nextest] tests, and both run at the version that [mise]
//! installed for the project. The whole run happens in a disposable copy of
//! the project, because a resolution rewrites a lockfile, and the checkout of
//! a contributor is no place for that.
//!
//! A run only reports, and it takes no argument. An update that cargo could
//! not finish becomes a finding at the manifest of its workspace, a test that
//! failed becomes a finding at the position where it panicked, and a build
//! that does not finish becomes findings from the diagnostics of the compiler.
//! The action applies to a project that holds a manifest of cargo, and it
//! skips visibly otherwise.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_latest_deps::CheckLatestDeps;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckLatestDeps)];
//! ```
//!
//! [cargo]: https://doc.rust-lang.org/cargo/
//! [mise]: https://mise.jdx.dev
//! [nextest]: https://nexte.st

/// Types for the action that checks a project against the newest versions of
/// its dependencies
pub mod check_latest_deps;

pub use self::check_latest_deps::{CheckLatestDeps, CheckLatestDepsError};
