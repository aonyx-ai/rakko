/// The files of one side of a declaration, as paths
mod file_list;

use getset::Getters;

pub use self::file_list::FileList;

/// The files that a run reads and the files that it writes, as lists of
/// paths
///
/// [`FileAccess::expand`] returns this. Each list holds the files that exist
/// in the project and that the side of the declaration covers, together with
/// the file that each pattern without a wildcard describes. A caller that
/// orders several runs compares these lists, and never has to compare the
/// patterns themselves.
///
/// A list does not show a file that a run creates. A side that covers the
/// whole project also covers such files, so a run that writes the whole
/// project conflicts with every other run, whatever the lists hold. A caller
/// that orders runs reads that side from [`FileAccess::writes`], because a
/// list of the whole project looks like any other list.
///
/// A run that declares [`Access::Read`] writes no file, whatever its list of
/// writes holds.
///
/// [`Access::Read`]: crate::Access::Read
/// [`FileAccess::expand`]: crate::FileAccess::expand
/// [`FileAccess::writes`]: crate::FileAccess::writes
#[derive(Clone, Eq, PartialEq, Debug, Default, Getters)]
pub struct FileLists {
    /// The files that the run reads
    #[getset(get = "pub")]
    reads: FileList,
    /// The files that the run writes
    #[getset(get = "pub")]
    writes: FileList,
}

impl FileLists {
    /// Creates the lists of a run from the files that it reads and the files
    /// that it writes
    pub fn new(reads: FileList, writes: FileList) -> Self {
        Self { reads, writes }
    }
}
