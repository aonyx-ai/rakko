//! The action that formats the TOML files of a project
//!
//! [Taplo] does the work: it discovers the TOML files, reads its own
//! configuration, and formats what that configuration selects, so a run of
//! the action agrees with an editor and with a contributor that runs taplo
//! bare. The action declares the version of taplo that it runs. It starts
//! that version, which [mise] installed, and translates what taplo reported
//! into the outcome of the run.
//!
//! A run reports by default and rewrites with the `fix` argument. A project
//! that holds no TOML file passes, because the run found nothing to format.
//! The action does not skip such a project, because taplo does not always
//! report that it found no file.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_format_toml::FormatToml;
//!
//! let actions = Bundle::default().with(FormatToml);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [taplo]: https://taplo.tamasfe.dev

mod format_toml;

pub use self::format_toml::{FormatMode, FormatToml, FormatTomlArgs, FormatTomlError};
