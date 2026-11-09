//! One tool that one mounted action declares

use bon::bon;
use getset::Getters;
use rakko::{Name, ToolRequirement};

/// One tool that one mounted action declares
///
/// A finding about a toolchain gives the action that requires it, so the
/// declaration keeps the name of the action beside the requirement.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Getters)]
pub(crate) struct Declaration {
    /// The action that declares the tool
    #[getset(get = "pub(crate)")]
    action: Name,

    /// What the action requires of the tool
    #[getset(get = "pub(crate)")]
    requirement: ToolRequirement,
}

#[bon]
impl Declaration {
    /// Creates the declaration of a tool by an action
    #[builder]
    pub(crate) fn new(action: Name, requirement: ToolRequirement) -> Self {
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
    fn declaration_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Declaration>();
    }
}
