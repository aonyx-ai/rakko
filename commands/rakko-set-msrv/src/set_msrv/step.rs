use std::fmt;

use super::args::Msrv;

/// A step of a run that the run hands to mise
///
/// Both steps come after the run wrote its files. An error names the step as
/// the command that the run started, so that the user can start it again
/// after they fixed the cause.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum MiseStep {
    /// Mise locks the Rust pins of the project again
    Lock,

    /// Mise installs the Rust toolchain at the new version
    Install {
        /// The version that the run set
        msrv: Msrv,
    },
}

impl MiseStep {
    /// Returns the arguments that ask mise for the step
    pub fn arguments(&self) -> Vec<String> {
        match self {
            Self::Lock => vec!["lock".to_owned(), "rust".to_owned()],
            Self::Install { msrv } => vec!["install".to_owned(), format!("rust@{msrv}")],
        }
    }
}

/// Shows the step as the command that starts it
impl fmt::Display for MiseStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "mise {}", self.arguments().join(" "))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn display_of_install_names_the_version() {
        let step = MiseStep::Install {
            msrv: Msrv::new("1.89.0"),
        };

        assert_eq!(step.to_string(), "mise install rust@1.89.0");
    }

    #[test]
    fn display_of_lock_names_the_rust_pins() {
        let step = MiseStep::Lock;

        assert_eq!(step.to_string(), "mise lock rust");
    }
}
