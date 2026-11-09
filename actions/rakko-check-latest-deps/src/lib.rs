//! The action that checks a project against the newest versions of its
//! dependencies
//!
//! A manifest states a floor for each dependency, and a floor is a promise:
//! every later version of that dependency works as well. The action resolves
//! the newest versions that the manifests allow and runs the tests against
//! them, so the promise is confirmed instead of assumed.
//!
//! [Cargo] resolves and [nextest] tests. Cargo runs on the stable toolchain
//! that the project pins, and nextest at the version that the action declares,
//! which [mise] installed. The whole run happens in a disposable copy of the
//! project, because a resolution rewrites a lockfile, and the checkout of a
//! contributor is no place for that.
//!
//! A run only reports, and it takes no argument. An update that cargo could
//! not finish becomes a finding at the manifest of its workspace, a test that
//! failed becomes a finding at the position where it panicked, and a build
//! that does not finish becomes findings from the diagnostics of the compiler.
//! A project whose cargo discovers no workspace skips visibly, and says so.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_check_latest_deps::CheckLatestDeps;
//!
//! let actions = Bundle::default().with(CheckLatestDeps);
//! ```
//!
//! [cargo]: https://doc.rust-lang.org/cargo/
//! [mise]: https://mise.jdx.dev
//! [nextest]: https://nexte.st

mod check_latest_deps;

pub use self::check_latest_deps::{CheckLatestDeps, CheckLatestDepsError};
