//! The paths where the tree of a contributor differs from its commit
//!
//! Git names those paths in the report that `git status` writes, and the sync
//! copies exactly them. This module turns the report into the paths.
//!
//! The report separates its entries with a zero byte, which is the one byte
//! that no path holds, so a path with a space, a quotation mark, or a line
//! break arrives as git read it from the directory. Every other format of the
//! report escapes such a path, and the escape would have to be undone here.

use std::path::PathBuf;

/// The byte that separates the entries of the report
const SEPARATOR: u8 = 0;

/// The number of bytes that an entry spends on the state of its path
///
/// Git writes the state of the index, the state of the working tree, and a
/// space, and the path follows them.
const STATE: usize = 3;

/// The letter that marks the path of a copy
const COPIED: u8 = b'C';

/// The letter that marks the path of a rename
const RENAMED: u8 = b'R';

/// Returns the paths that the report of git names
///
/// The paths are relative to the top level of the repository, in the order in
/// which git wrote them, and each of them appears once.
///
/// An entry of a rename or of a copy carries the path that the file has now
/// and the path that it had before, in this order. Both are paths where the
/// tree differs from the commit, so both travel to the caller: the sync copies
/// the one that the project holds and removes the one that it does not.
///
/// # Errors
///
/// Returns the description of the first entry that the crate cannot read. The
/// sync names its paths from this report, so an entry that it skipped would
/// leave a file of the contributor out of the copy.
pub(super) fn changed_paths(report: &[u8]) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let mut entries = report
        .split(|byte| *byte == SEPARATOR)
        .filter(|entry| !entry.is_empty());

    while let Some(entry) = entries.next() {
        let Some((state, path)) = entry.split_at_checked(STATE) else {
            return Err(describe(entry));
        };

        if state[2] != b' ' || path.is_empty() {
            return Err(describe(entry));
        }

        paths.push(path_of(path).ok_or_else(|| describe(entry))?);

        if !moved(state) {
            continue;
        }

        let Some(previous) = entries.next() else {
            return Err(describe(entry));
        };

        paths.push(path_of(previous).ok_or_else(|| describe(previous))?);
    }

    Ok(paths)
}

/// Returns what the crate can say about an entry that it cannot read
///
/// The entry holds a path, and a path can hold bytes that are part of no
/// character, so the description replaces those instead of dropping the entry
/// from the message.
fn describe(entry: &[u8]) -> String {
    format!(
        "git wrote an entry that the crate cannot read: `{}`",
        String::from_utf8_lossy(entry)
    )
}

/// Returns whether the entry of this state names a second path
///
/// Git writes the path that the file has now and the path that it had before
/// when it reports a rename or a copy, in the index or in the working tree.
fn moved(state: &[u8]) -> bool {
    state[..2]
        .iter()
        .any(|letter| *letter == RENAMED || *letter == COPIED)
}

/// Returns the path that the bytes of an entry name
///
/// A path of this platform is a sequence of bytes, so the bytes reach the path
/// as git wrote them, and a name that is part of no character stays readable.
#[cfg(unix)]
fn path_of(bytes: &[u8]) -> Option<PathBuf> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    Some(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// Returns the path that the bytes of an entry name
///
/// A path of this platform is a sequence of characters, and git writes a path
/// as UTF-8, so a sequence that is no valid UTF-8 names no path here.
#[cfg(not(unix))]
fn path_of(bytes: &[u8]) -> Option<PathBuf> {
    std::str::from_utf8(bytes).ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // worktree[verify sync.changed]
    #[test]
    fn changed_paths_of_a_modified_file_names_it() {
        let report = b" M src/lib.rs\0";

        let paths = changed_paths(report);

        assert_eq!(paths, Ok(vec![PathBuf::from("src/lib.rs")]));
    }

    // worktree[verify sync.changed]
    #[test]
    fn changed_paths_of_a_rename_names_both_paths() {
        let report = b"R  src/new.rs\0src/old.rs\0";

        let paths = changed_paths(report);

        assert_eq!(
            paths,
            Ok(vec![
                PathBuf::from("src/new.rs"),
                PathBuf::from("src/old.rs"),
            ])
        );
    }

    #[test]
    fn changed_paths_of_a_rename_without_its_second_path_reports_the_entry() {
        let report = b"R  src/new.rs\0";

        let paths = changed_paths(report);

        assert!(paths.is_err());
    }

    #[test]
    fn changed_paths_of_an_empty_report_names_nothing() {
        let report = b"";

        let paths = changed_paths(report);

        assert_eq!(paths, Ok(Vec::new()));
    }

    #[test]
    fn changed_paths_of_an_entry_without_a_path_reports_the_entry() {
        let report = b"?? \0";

        let paths = changed_paths(report);

        assert!(paths.is_err());
    }

    #[test]
    fn changed_paths_of_an_entry_without_a_state_reports_the_entry() {
        let report = b"?\0";

        let paths = changed_paths(report);

        assert!(paths.is_err());
    }

    // worktree[verify sync.changed]
    #[test]
    fn changed_paths_of_a_path_with_a_space_keeps_the_path_whole() {
        let report = b"?? two words.txt\0";

        let paths = changed_paths(report);

        assert_eq!(paths, Ok(vec![PathBuf::from("two words.txt")]));
    }

    #[test]
    fn changed_paths_of_several_entries_keeps_the_order_of_git() {
        let report = b" M a.txt\0D  b.txt\0?? c.txt\0";

        let paths = changed_paths(report);

        assert_eq!(
            paths,
            Ok(vec![
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt"),
                PathBuf::from("c.txt"),
            ])
        );
    }
}
