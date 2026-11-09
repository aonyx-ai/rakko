//! The cause of a failure to parse a file of the project as TOML

use thiserror::Error;

/// The cause of a failure to parse a file of the project as TOML
///
/// The error keeps the message of the TOML parser, which usually gives the
/// line and the column of the problem. It keeps no other part of the error of
/// the parser, so a later release can change the parser without a break.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct ParseTomlError {
    /// What the TOML parser said about the file
    message: String,
}

impl ParseTomlError {
    /// Returns the cause with the message of an error of the TOML parser
    pub(crate) fn new(source: &toml_edit::TomlError) -> Self {
        Self {
            message: source.to_string(),
        }
    }
}
