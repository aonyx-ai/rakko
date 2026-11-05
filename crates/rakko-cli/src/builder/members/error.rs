use rakko_action::{Name, ReadArgsError};
use thiserror::Error;

/// An error that occurs when the command line binds the members of a group
///
/// The command line binds every member of a group to the values of the run
/// before the group runs. A member whose values do not convert stops the run
/// of the group before any member runs, because a run that drives only some of
/// its members would report on a project that it did not fully examine.
#[derive(Debug, Error)]
pub(crate) enum BindMembersError {
    /// The values of the run do not convert into the arguments of a member
    #[error("failed to bind the values of the run to the action '{member}'")]
    UnboundMember {
        /// The name of the member that did not bind
        member: Name,
        /// The error of the conversion, which gives the argument
        source: ReadArgsError,
    },
}
