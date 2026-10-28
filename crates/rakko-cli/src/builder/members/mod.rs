/// The error of a run whose member cannot bind its values
mod error;
/// One flag of a group, merged from the arguments of its members
mod merged_argument;

use rakko_action::{ArgsValues, ErasedAction, Name};

pub(crate) use self::error::BindMembersError;
pub(crate) use self::merged_argument::MergedArgument;
use crate::bound_member::BoundMember;

/// Returns the flags of a group, merged from the arguments of its members
///
/// Each argument that a member declares becomes one flag, and an argument that
/// several members declare stays one flag. The flags follow the order in which
/// the members first declare their arguments.
///
/// # Panics
///
/// Panics when two members declare one argument with two different shapes,
/// and reports the group, both members, the argument, and both shapes. One
/// flag cannot hold two shapes, and only a change of the harness or of an
/// action corrects that, so the failure happens where the harness mounts the
/// group and not where a user runs it.
pub(super) fn merge(group: &Name, members: &[Box<dyn ErasedAction>]) -> Vec<MergedArgument> {
    let mut merged: Vec<MergedArgument> = Vec::new();

    for member in members {
        let schema = member.arguments();

        for argument in schema.arguments() {
            match merged
                .iter_mut()
                .find(|flag| flag.argument().name() == argument.name())
            {
                Some(flag) => flag.add(group, member.name(), argument),
                None => merged.push(MergedArgument::new(member.name(), argument)),
            }
        }
    }

    merged
}

/// Binds every member of a group to the values of the run
///
/// The values hold one entry for each member, in the order of the members, and
/// each entry holds the values of the arguments that its member declares. The
/// members are bound before any of them runs, so a run that cannot bind one of
/// them runs none of them.
///
/// # Errors
///
/// Returns [`BindMembersError::UnboundMember`] for the first member whose
/// values do not convert into its arguments.
pub(super) fn bind<'a>(
    members: &'a [Box<dyn ErasedAction>],
    values: &[ArgsValues],
) -> Result<Vec<BoundMember<'a>>, BindMembersError> {
    members
        .iter()
        .zip(values)
        .map(|(member, values)| {
            BoundMember::bind(member.as_ref(), values).map_err(|source| {
                BindMembersError::UnboundMember {
                    member: member.name(),
                    source,
                }
            })
        })
        .collect()
}
