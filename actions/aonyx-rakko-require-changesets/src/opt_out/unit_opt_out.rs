//! The opt-out of one release unit

use getset::Getters;

use super::Reason;
use crate::release_unit::UnitName;

/// A line of the body of a pull request that opts one release unit out
///
/// The line has the form `No changeset for <unit>: <reason>`. The name can
/// refer to a unit that does not exist, such as after a typo. The check then
/// fails, so that a typo cannot skip a unit without a sign.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, bon::Builder, Getters)]
pub(crate) struct UnitOptOut {
    /// The name of the unit that the line opts out
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    unit: UnitName,

    /// The reason that the line gives
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    reason: Reason,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_opt_out_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<UnitOptOut>();
    }
}
