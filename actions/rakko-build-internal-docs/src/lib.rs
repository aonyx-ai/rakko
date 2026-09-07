//! The action that builds the internal documentation of a project
//!
//! [Rustdoc] does the work: cargo reads the manifests of the project and
//! renders the documentation of every package, with the private items and
//! with every feature, so a run of the action agrees with a contributor that
//! runs `cargo doc` bare. The action starts the cargo that [mise] installed
//! for the project, at the version that the project pinned, and translates
//! what cargo reported into the outcome of the run.
//!
//! The internal documentation describes the code to whoever maintains it.
//! The documentation that a project publishes for the people who use it is a
//! different task, with a tool of its own.
//!
//! Building the documentation is also the only examination that it gets.
//! Rustdoc resolves the links between items while it renders them, and a link
//! that names nothing is a warning that no other tool reports. Every
//! diagnostic becomes a finding, so a run with a warning fails.
//!
//! A run takes no argument. The action applies to a project that holds a
//! manifest of cargo, and it skips visibly otherwise.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_build_internal_docs::BuildInternalDocs;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(BuildInternalDocs)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [rustdoc]: https://doc.rust-lang.org/rustdoc/

/// Types for the action that builds the internal documentation of a project
pub mod build_internal_docs;

pub use self::build_internal_docs::{BuildInternalDocs, BuildInternalDocsError};
