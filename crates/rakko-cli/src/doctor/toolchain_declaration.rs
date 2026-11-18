//! What one mounted action requires of one toolchain of the project

use bon::bon;
use getset::Getters;
use rakko::{Name, ToolchainRequirement};

/// What one mounted action requires of one toolchain of the project
///
/// A finding about a toolchain gives the action that requires it, so the
/// declaration keeps the name of the action beside the requirement.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Getters)]
pub(crate) struct ToolchainDeclaration {
    /// The action that requires the toolchain
    #[getset(get = "pub(crate)")]
    action: Name,

    /// What the action requires of the toolchain
    #[getset(get = "pub(crate)")]
    requirement: ToolchainRequirement,
}

#[bon]
impl ToolchainDeclaration {
    /// Creates the declaration of a requirement of a toolchain by an action
    #[builder]
    pub(crate) fn new(action: Name, requirement: ToolchainRequirement) -> Self {
        Self {
            action,
            requirement,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn toolchain_declaration_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolchainDeclaration>();
    }
}
