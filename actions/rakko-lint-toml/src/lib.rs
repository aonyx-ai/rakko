//! The action that validates the TOML files of a project
//!
//! [Taplo] does the work: it discovers the TOML files, reads its own
//! configuration, and validates what that configuration selects, so a run of
//! the action agrees with an editor and with a contributor that runs taplo
//! bare. The action starts the taplo that [mise] installed for the project,
//! at the version that the project pinned, and translates what taplo
//! reported into the outcome of the run.
//!
//! A run only reports. Taplo repairs nothing that a validation finds, so the
//! action takes no argument. A project that holds no TOML file passes,
//! because the run found nothing to report. The action does not skip such a
//! project, because taplo does not always report that it found no file.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_lint_toml::LintToml;
//!
//! let actions = Bundle::default().with(LintToml);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [taplo]: https://taplo.tamasfe.dev

/// Types for the action that validates the TOML files of a project
mod lint_toml;

pub use self::lint_toml::{LintToml, LintTomlError};
