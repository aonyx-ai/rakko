//! The action that checks the specifications of a project
//!
//! [Tracey] does the work. A specification states what a crate does as a list
//! of requirements, each with an identifier. The code that implements or
//! verifies a requirement names that identifier in a comment. Tracey holds the
//! two together. The action declares the version of tracey that it runs. It
//! starts that version, which [mise] installed.
//!
//! A run asks three questions, because a specification can fail a project in
//! three ways. Does every reference point at a requirement that exists, at the
//! version that the requirement carries now? Does a change to the text of a
//! requirement carry the version bump that such a change needs? And how much
//! of the specification does the code answer for?
//!
//! The first two questions gate a run, and the third does not. A reference that
//! names nothing, and a requirement whose text moved under the code that
//! implements it, are both broken links. Coverage is information. A
//! specification may land before the code that answers it. A run that failed
//! for that would punish the order in which the work arrives.
//!
//! A run only reports, so the action takes no argument and changes nothing
//! about the project or its repository. Tracey answers the second question by
//! comparing the index of a repository with its HEAD. A code host checks out a
//! merge commit and stages nothing. So the comparison that a pull request needs
//! is built in a copy of the repository instead of by moving the HEAD of the
//! checkout.
//!
//! # Asynchronous Runtime
//!
//! A run starts programs and waits for them. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Requirements
//!
//! A run that compares a pull request with its base branch needs two things.
//! The project must be a git repository. Git must be reachable by name, the way
//! the operating system finds a program. Every other run needs neither.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_check_specs::CheckSpecs;
//!
//! let actions = Bundle::default().with(CheckSpecs);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs
//! [tracey]: https://tracey.bearcove.eu/

mod check_specs;
mod comparison;
mod environment;
mod problem;
mod tracey;

pub use self::check_specs::{CheckSpecs, CheckSpecsError};
pub use self::comparison::PrepareComparisonError;
pub use self::tracey::ObserveTraceyError;
