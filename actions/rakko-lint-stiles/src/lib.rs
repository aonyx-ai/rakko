//! The action that applies the stiles rules of a project
//!
//! [Stiles] does the work: it reads its own configuration, loads the lints
//! that the configuration names, discovers the Rust and TypeScript files, and
//! applies the rules that the configuration turns on, so a run of the action
//! agrees with a contributor that runs stiles bare. The action declares the
//! version of stiles that it runs. It starts that version, which [mise]
//! installed, and translates what stiles reported into the outcome of the run.
//!
//! Stiles ships no rule of its own. A project names the lints that it runs in
//! `.config/stiles.toml`, and those rules are all that a check applies. The
//! action therefore first asks stiles to list the rules that the project
//! runs, and it skips visibly when stiles lists none, because a check with no
//! rule passes whatever the code holds. It skips as well when stiles finds no
//! file to check.
//!
//! A run only reports, so the action takes no argument. Each rule gives its
//! diagnostics a severity. This crate reports a warning and an error alike,
//! because both are rules that the project asked stiles to run, and a run with
//! either fails. A note and a help fail no run, as in stiles itself.
//!
//! The status of the process does not answer alone. Stiles ends with success
//! when every diagnostic is a warning, and it ends with the same failure
//! status for an error in the code and for a run that it stopped. The report
//! and the errors that stiles writes beside it decide which one a run was, and
//! a run that stopped stops the action instead of passing quietly.
//!
//! # Provisioning
//!
//! The action declares stiles with the GitHub backend of mise, which downloads
//! the archive of a release. The declaration names the tag of the release,
//! such as `stiles@0.4.0`. The action asks mise for the program named `stiles`
//! at that version, whatever version the project pins.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that, and
//! it panics without one.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_lint_stiles::LintStiles;
//!
//! let actions = Bundle::default().with(LintStiles);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [stiles]: https://github.com/aonyx-ai/stiles
//! [tokio]: https://tokio.rs

/// Types for the action that applies the stiles rules of a project
mod lint_stiles;
/// Types for what one check of stiles produced
mod observation;
/// Types for one diagnostic that stiles reported about a file
mod problem;
/// Types for the stiles that a project runs
mod stiles;

pub use self::lint_stiles::{LintStiles, LintStilesError};
