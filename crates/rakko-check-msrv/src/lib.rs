//! The action that checks a project against the Rust version it promises
//!
//! A package writes the oldest toolchain that it compiles on as the
//! `rust-version` of its manifest, and whoever depends on the package reads
//! that as a fact. Only the compiler can confirm the fact, so this action
//! runs the compiler on the toolchain that the promise names and reports
//! what it said.
//!
//! Cargo does the work: it reads the manifests of the project, and rustup
//! selects the toolchain, so a run agrees with a contributor who checks the
//! same thing by hand. The action starts the cargo that [mise] installed for
//! the project, on the toolchain that mise installed for the declared
//! version, and it installs nothing.
//!
//! A run only reports, and it takes no argument. Every diagnostic of the
//! compiler becomes a finding, so a run with a warning fails: the older
//! compiler answers for the promise, and a deprecation that only it reports
//! is part of the answer. The action applies to a project that holds a
//! manifest of cargo and declares a Rust version in it, and it skips visibly
//! otherwise.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_msrv::CheckMsrv;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckMsrv)];
//! ```
//!
//! [mise]: https://mise.jdx.dev

/// Types for the action that checks the Rust version of a project
pub mod check_msrv;

pub use self::check_msrv::{CheckMsrv, CheckMsrvError};
