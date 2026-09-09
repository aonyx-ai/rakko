//! What the tests of Rakko need on more than one platform
//!
//! A test that names a file compares a path, and a path reads differently on
//! each platform: Windows separates the parts of one with a different
//! character, and it calls a path absolute only when the path names a volume
//! as well. A literal in a test therefore describes one path on one platform
//! and another, or no absolute path at all, on the next.
//!
//! This crate takes a literal in the one form that a reader knows, with a
//! forward slash between the parts, and builds the path that the platform
//! names. One literal then describes the same file everywhere, and no test
//! states a path twice. [`path`][path] answers with the path, and
//! [`path_text`][path_text] with the text of that path, which is what a
//! program writes and what a test that reads a report compares.
//!
//! A crate depends on this one as a development dependency, so nothing that a
//! project runs carries it.
//!
//! # Examples
//!
//! ```
//! use rakko_test_utils::{path, path_text};
//!
//! let path = path("src/lib.rs");
//! let text = path_text("src/lib.rs");
//!
//! assert_eq!(path.to_string_lossy(), text);
//! ```
//!
//! [path]: path()
//! [path_text]: path_text()

use std::path::PathBuf;

/// The root of the file system that Windows reads as absolute
///
/// A path is absolute on Windows only when it names a volume, so a path that
/// opens with a separator is relative there. The letter is the volume that
/// every installation of Windows has.
#[cfg(windows)]
const ROOT: &str = r"C:\";

/// The root of the file system that every other platform reads as absolute
#[cfg(not(windows))]
const ROOT: &str = "/";

/// The character that separates the parts of a literal
const SEPARATOR: char = '/';

/// Returns the path that the parts of a literal name on this platform
///
/// The literal writes a forward slash between the parts, and the answer joins
/// them the way the platform joins them. A literal that opens with a
/// separator names an absolute path, and the answer opens with what the
/// platform reads as the root of the file system.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use rakko_test_utils::path;
///
/// assert_eq!(path("src/lib.rs"), Path::new("src").join("lib.rs"));
/// assert!(path("/home/otter/project").is_absolute());
/// ```
// testutils[impl path.relative]
// testutils[impl path.absolute]
pub fn path(literal: &str) -> PathBuf {
    let Some(rest) = literal.strip_prefix(SEPARATOR) else {
        return literal.split(SEPARATOR).collect();
    };

    let mut path = PathBuf::from(ROOT);
    path.extend(rest.split(SEPARATOR));

    path
}

/// Returns the text of the path that a literal names on this platform
///
/// A program writes a path as text, and a report that names a file therefore
/// carries text and not a path. A test that compares such a report asks for
/// this instead of for [`path`][path].
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use rakko_test_utils::path_text;
///
/// assert_eq!(path_text("src/lib.rs"), Path::new("src").join("lib.rs").to_string_lossy());
/// ```
///
/// [path]: path()
// testutils[impl path.text]
pub fn path_text(literal: &str) -> String {
    path(literal).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    // testutils[verify path.absolute]
    #[test]
    fn path_of_a_literal_that_opens_with_a_separator_is_absolute() {
        let path = path("/home/otter/project");

        assert!(path.is_absolute());
    }

    // testutils[verify path.relative]
    #[test]
    fn path_of_a_literal_joins_its_parts() {
        let path = path("src/lib.rs");

        assert_eq!(path, Path::new("src").join("lib.rs"));
    }

    // testutils[verify path.relative]
    #[test]
    fn path_of_a_literal_without_a_separator_names_the_literal() {
        let path = path("Cargo.toml");

        assert_eq!(path, Path::new("Cargo.toml"));
    }

    // testutils[verify path.absolute]
    #[test]
    fn path_of_an_absolute_literal_keeps_its_parts() {
        let path = path("/home/otter/project");

        assert!(path.ends_with(Path::new("home").join("otter").join("project")));
    }

    // testutils[verify path.text]
    #[test]
    fn path_text_of_a_literal_writes_the_path_of_the_platform() {
        let text = path_text("src/lib.rs");

        assert_eq!(text, Path::new("src").join("lib.rs").to_string_lossy());
    }
}
