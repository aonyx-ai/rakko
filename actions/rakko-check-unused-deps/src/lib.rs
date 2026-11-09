//! The action that finds the dependencies a project declares and never uses
//!
//! A dependency that nothing reaches still costs the project. It is built, it
//! is audited, and it is updated. It also widens the surface that a reader of
//! the manifest has to understand. Nothing in the manifest says whether a
//! dependency is used, so the answer comes from a build.
//!
//! [Cargo-udeps] does the work. It reads the record that the compiler writes
//! about the crates each target actually loaded. It holds that record against
//! the dependencies of the manifest. The action declares the version of
//! cargo-udeps that it runs. It starts that version, which [mise] installed, on
//! the nightly toolchain that the project pins, and it installs nothing. The
//! nightly channel is not a preference: the record comes from an unstable
//! option of the compiler, and a stable toolchain refuses it.
//!
//! A run only reports, and it takes no argument. Every unused dependency
//! becomes a finding at the manifest that declares it. The action applies to
//! a project that holds a manifest of cargo, and it skips visibly otherwise.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_check_unused_deps::CheckUnusedDeps;
//!
//! let actions = Bundle::default().with(CheckUnusedDeps);
//! ```
//!
//! [cargo-udeps]: https://github.com/est31/cargo-udeps
//! [mise]: https://mise.jdx.dev

mod check_unused_deps;
mod report;
mod udeps;

pub use self::check_unused_deps::{CheckUnusedDeps, CheckUnusedDepsError};
pub use self::report::ReadUdepsReportError;
