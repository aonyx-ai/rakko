//! The action that applies the stiles rules of a project
//!
//! [Stiles] does the work: it reads its own configuration, loads the lints
//! the configuration names, and discovers the Rust and TypeScript files. It
//! applies the rules the configuration turns on, so a run of the action agrees
//! with a contributor that runs stiles bare.
//!
//! The action starts the stiles that [mise] installed for the project. It uses
//! the version that the project pinned, if the action accepts that version.
//! The action translates what stiles reported into the outcome of the run.
//!
//! Stiles ships no rule of its own. A project names the lints it runs in
//! `.config/stiles.toml`, and those rules are all that a check applies.
//!
//! The action therefore first asks stiles to list the rules that the project
//! runs. It skips visibly when stiles lists none, because a check with no rule
//! passes whatever the code holds. It skips as well when stiles finds no file
//! to check.
//!
//! A run only reports, so the action takes no argument.
//!
//! Each rule gives its diagnostics a severity. This crate reports a warning
//! and an error alike, because both are rules that the project asked stiles to
//! run. A run with either fails. A note and a help fail no run, as in stiles
//! itself.
//!
//! The status of the process does not answer alone.
//!
//! Stiles ends with success when every diagnostic is a warning. It ends with
//! the same failure status for an error in the code and for a run that it
//! stopped. The report and the errors that stiles writes beside it decide which
//! one a run was. A run that stopped stops the action instead of passing
//! quietly.
//!
//! # Provisioning
//!
//! A project pins stiles in its `mise.toml` with the GitHub backend of mise,
//! which downloads the archive of a release. Stiles tags each release with the
//! name of the tool, so the pin names that prefix. Stiles publishes no build
//! for Windows, so the pin installs it only on the platforms that have one:
//!
//! ```toml
//! [tools]
//! "github:aonyx-ai/stiles" = { version = "0.6.0", version_prefix = "stiles@", os = ["linux", "macos"] }
//! ```
//!
//! The action declares stiles as a requirement that accepts exactly one
//! version. The rules that a project loads are built against one version of
//! stiles, and another version refuses them. So the action stops when the
//! project pins another version.
//!
//! The action asks mise for the program named `stiles`. So a release whose
//! archive holds a program of another name needs the `bin` option of the
//! backend to rename it.
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

mod lint_stiles;
mod observation;
mod problem;
mod stiles;

pub use self::lint_stiles::{LintStiles, LintStilesError};
