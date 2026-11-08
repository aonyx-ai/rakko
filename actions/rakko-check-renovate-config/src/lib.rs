//! The action that checks the Renovate configuration of a project
//!
//! `renovate-config-validator`, the validator that [Renovate] ships, does the
//! work: it finds the configurations of the project, reads each of them, and
//! reports every option that Renovate would refuse. A run of the action
//! agrees with a contributor that runs `renovate-config-validator --strict`,
//! and then `renovate-config-validator --strict --no-global default.json`.
//! The action starts the validator that [mise] installed for the project, at
//! the version that the project pinned, and translates what the validator
//! reported into the outcome of the run.
//!
//! A configuration that Renovate refuses on the default branch stops Renovate
//! from opening pull requests until someone fixes it. The action asks whether
//! Renovate would accept the configuration before it merges.
//!
//! A run asks for strict validation, so an option that needs a migration fails
//! the run like an error and a warning do. Each error and each warning becomes
//! a finding at the file that the validator named, and a configuration that
//! needs a migration becomes one finding that names the options that the
//! migration changes.
//!
//! A preset is a configuration that other configurations extend. Renovate
//! reads a preset such as `github>owner/repository` from the `default.json` in
//! the root of that repository. The validator does not find this file by
//! itself, so the action also validates a `default.json` in the root of the
//! project. It does this for every such file, also for one that belongs to
//! another tool. The validator checks the preset as the configuration of a
//! repository. These checks are stricter than the check of a preset in
//! Renovate. So a preset that only fits inside a package rule fails the run,
//! such as one that sets `matchPackageNames` at the top.
//!
//! The validator also reads the global configuration that a self-hosted
//! Renovate starts with, which is `config.js` in the root of the project unless
//! the environment names another file. A `config.js` that is not a
//! configuration of Renovate therefore produces findings as well.
//!
//! A run asks the validator for the regular expressions of JavaScript instead
//! of RE2, so that every machine validates with the same engine. RE2 refuses
//! some patterns that JavaScript accepts, and such a pattern passes the
//! action.
//!
//! A run only reports, so the action takes no argument. The action applies to
//! a project that holds a Renovate configuration or a preset, and it skips
//! visibly otherwise.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_check_renovate_config::CheckRenovateConfig;
//!
//! let actions = Bundle::default().with(CheckRenovateConfig);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [renovate]: https://docs.renovatebot.com
//! [tokio]: https://tokio.rs

/// Types for the action that checks the Renovate configuration of a project
mod check_renovate_config;
/// Types for what one run of the validator produced
mod observation;
/// Types for one problem that the validator reported about a configuration
mod problem;
/// Types for the validator that a project runs
mod validator;

pub use self::check_renovate_config::{CheckRenovateConfig, CheckRenovateConfigError};
pub use self::validator::report::ReadReportError;
