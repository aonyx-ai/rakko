use std::error::Error;
use std::fmt;

/// An error together with every cause behind it
///
/// The `Display` of an error writes its own layer only. An error usually
/// states what failed and carries the reason as its source, so the layer that
/// a reader needs is usually the innermost one. A chain writes the error and
/// then each source in turn, joined by a colon, so that a reader learns why a
/// run stopped without a flag that asks for more.
///
/// Every place that shows an error to a reader writes it through this type,
/// so a failed command, a stopped action in text, and a stopped action in
/// JSON all show the same chain. A message that holds a line break, such as
/// the output of a tool, keeps it.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Chain<'a> {
    /// The outermost error of the chain
    error: &'a (dyn Error + 'a),
}

impl<'a> Chain<'a> {
    /// Creates the chain that starts at the given error
    #[must_use]
    pub(crate) fn new(error: &'a (dyn Error + 'a)) -> Self {
        Self { error }
    }
}

impl fmt::Display for Chain<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.error)?;

        let mut cause = self.error.source();
        while let Some(error) = cause {
            write!(formatter, ": {error}")?;
            cause = error.source();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::io;

    use super::*;

    /// An error that carries its cause in a field, as the errors of an action do
    #[derive(Debug, thiserror::Error)]
    #[error("failed to read Cargo.toml")]
    struct ReadManifestError {
        /// Why the file could not be read
        source: io::Error,
    }

    #[test]
    fn display_with_a_field_source_writes_the_source() {
        let error = ReadManifestError {
            source: io::Error::other("the disk is gone"),
        };

        let line = Chain::new(&error).to_string();

        assert_eq!(line, "failed to read Cargo.toml: the disk is gone");
    }

    #[test]
    fn display_with_no_source_writes_the_error_alone() {
        let error = io::Error::other("the port is taken");

        let line = Chain::new(&error).to_string();

        assert_eq!(line, "the port is taken");
    }

    #[test]
    fn display_with_three_layers_writes_every_layer_in_order() {
        let error = clawless::Error::msg("the file is not TOML")
            .context("failed to parse Cargo.toml")
            .context("failed to load the workspace");

        let line = Chain::new(error.as_ref()).to_string();

        assert_eq!(
            line,
            "failed to load the workspace: failed to parse Cargo.toml: the file is not TOML"
        );
    }
}
