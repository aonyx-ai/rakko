//! The action that checks the Renovate configuration of a project
//!
//! `renovate-config-validator`, the validator that [Renovate] ships, does the
//! work: it finds the configurations of the project, reads each of them, and
//! reports every option that Renovate would refuse, so a run of the action
//! agrees with a contributor that runs the validator bare. The action starts
//! the validator that [mise] installed for the project, at the version that the
//! project pinned, and translates what the validator reported into the outcome
//! of the run.
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
//! a project that holds a Renovate configuration, and it skips visibly
//! otherwise.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it. A [Tokio] runtime drives that,
//! and it panics without one.
//!
//! # Examples
//!
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_renovate_config::CheckRenovateConfig;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckRenovateConfig)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [renovate]: https://docs.renovatebot.com
//! [tokio]: https://tokio.rs

/// Types for the action that checks the Renovate configuration of a project
pub mod check_renovate_config;
/// Types for what one run of the validator produced
pub mod observation;
/// Types for one problem that the validator reported about a configuration
pub mod problem;
/// Types for the validator that a project runs
pub mod validator;

pub use self::check_renovate_config::{CheckRenovateConfig, CheckRenovateConfigError};
pub use self::observation::Observation;
pub use self::problem::RenovateProblem;
pub use self::validator::report::{ReadReportError, Report};
pub use self::validator::{ObserveValidatorError, Validator};
