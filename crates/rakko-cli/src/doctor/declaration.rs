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

impl Declaration {
    /// Creates the declaration of a tool by an action
    pub(crate) fn new(action: Name, requirement: ToolRequirement) -> Self {
        Self {
            action,
            requirement,
        }
    }
}
