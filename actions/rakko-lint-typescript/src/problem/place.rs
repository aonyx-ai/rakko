//! Where a diagnostic of oxlint belongs
//!
//! Oxlint reports most diagnostics at a position in a file. It also reports
//! diagnostics about a whole file and about the project, and this module holds
//! the answer for all three.

use std::path::PathBuf;

use rakko_action::Position;

/// Where a diagnostic of oxlint belongs
///
/// A diagnostic about the code of a file marks a position in that file. A
/// diagnostic that names a file and marks no position in it belongs to the
/// whole file. Oxlint also reports diagnostics about the project, such as a
/// suppressions file that no longer agrees with the code, and these name no
/// file.
///
/// A path stands as oxlint wrote it. A run starts oxlint in the project root
/// and names the root as the place to look, so oxlint reports every path
/// relative to it.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Place {
    /// The diagnostic belongs to the project, and to no file in it
    Project,

    /// The diagnostic belongs to a file, and to no position in it
    File {
        /// The path of the file, as oxlint wrote it
        path: PathBuf,
    },

    /// The diagnostic sits at one position in a file
    ///
    /// Oxlint writes one diagnostic for a rule, however many places of the
    /// file it marks. The position is the first of them, which is the place
    /// that oxlint itself points a reader at.
    Position {
        /// The path of the file, as oxlint wrote it
        path: PathBuf,

        /// The first place in the file that oxlint marked
        position: Position,
    },
}
