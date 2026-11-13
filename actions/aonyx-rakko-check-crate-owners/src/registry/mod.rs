//! The registry that holds the owners of the crates
//!
//! A run asks the registry for the owners of each crate, and with `fix` it
//! asks the registry to add each owner that is missing. The run talks to the
//! registry through the trait of this module. So the comparison of the
//! owners does not depend on the network, and a test gives it a registry
//! that answers from memory.

mod addition;
mod ownership;

use std::future::Future;

use aonyx_rakko_cargo::{CargoRoot, PackageName};

pub(crate) use self::addition::Addition;
pub(crate) use self::ownership::Ownership;
use crate::crate_owner::CrateOwner;
use crate::crates_io::LookUpOwnersError;

/// The registry that holds the owners of the crates of a project
///
/// The trait requires `Sync`, because a run holds a reference to the
/// registry while it waits, and a run can move between threads.
pub(crate) trait Registry: Sync {
    /// Returns the owners of a crate, or that the registry does not have it
    ///
    /// # Errors
    ///
    /// Returns a [`LookUpOwnersError`] when the registry cannot answer.
    fn owners(
        &self,
        name: &PackageName,
    ) -> impl Future<Output = Result<Ownership, LookUpOwnersError>> + Send;

    /// Adds an owner to a crate
    ///
    /// The root is the workspace that holds the crate. A registry that runs
    /// a tool for the addition runs it there. The answer says whether the
    /// registry added the owner, refused it, or failed for another reason,
    /// such as a request that did not reach it.
    fn add(
        &self,
        root: &CargoRoot,
        name: &PackageName,
        owner: &CrateOwner,
    ) -> impl Future<Output = Addition> + Send;
}
