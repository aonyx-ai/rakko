//! The action that finds the dependencies a project declares and never uses
//!
//! A dependency that nothing reaches still costs the project: it is built,
//! it is audited, it is updated, and it widens the surface that a reader of
//! the manifest has to understand. Nothing in the manifest says whether a
//! dependency is used, so the answer comes from a build.
//!
//! [Cargo-udeps] does the work. It reads the record that the compiler writes
//! about the crates each target actually loaded, and it holds that record
//! against the dependencies of the manifest. The action starts the cargo
//! that [mise] installed for the project, on the nightly toolchain that mise
//! installed for the project, and it installs nothing. The nightly channel
//! is not a preference: the record comes from an unstable option of the
//! compiler, and a stable toolchain refuses it.
//!
//! A run only reports, and it takes no argument. Every unused dependency
//! becomes a finding at the manifest that declares it. The action applies to
//! a project that holds a manifest of cargo, and it skips visibly otherwise.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_unused_deps::CheckUnusedDeps;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckUnusedDeps)];
//! ```
//!
//! [cargo-udeps]: https://github.com/est31/cargo-udeps
//! [mise]: https://mise.jdx.dev

/// Types for the action that finds unused dependencies
pub mod check_unused_deps;
/// Types for what cargo-udeps reported about a workspace
pub mod report;

pub use self::check_unused_deps::{CheckUnusedDeps, CheckUnusedDepsError};
pub use self::report::{DependencyKind, ReadUdepsReportError, UdepsReport, UnusedDependency};
