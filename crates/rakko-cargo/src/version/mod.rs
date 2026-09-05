//! A version of the Rust compiler
//!
//! Two questions of this crate answer with a version: which toolchain a
//! project pins for its builds, and which toolchain the packages of a
//! workspace promise to compile on. Both read a version out of text that
//! another program wrote, and both pick the newest of several, so this
//! module holds the version and the comparison that orders them.

/// The error that stops the reading of a declaration
mod error;

use std::cmp::Ordering;

pub use self::error::ReadRustVersionError;

/// The character that separates the parts of a version
const SEPARATOR: char = '.';

/// A version of the Rust compiler
///
/// A version names a release of the compiler, such as `1.88.0`. It reaches
/// the crate as text that another program wrote: mise reports the version
/// that a pin of a project resolved to, and cargo reports the version that a
/// package declares as the oldest one it compiles on.
///
/// The value carries that text as it was written, because it is the name
/// that rustup and a provisioning layer know the toolchain by. `1.88` and
/// `1.88.0` name one release and are two spellings, so they are two values
/// here.
///
/// The type has no order, because the order of the text is not the order of
/// the versions: `1.9` comes before `1.88` as text and after it as a
/// version. [`highest`][highest] reads the numbers instead.
///
/// # Examples
///
/// ```
/// use rakko_cargo::RustVersion;
///
/// let versions = [RustVersion::new("1.9.0"), RustVersion::new("1.88.0")];
///
/// assert_eq!(RustVersion::highest(versions), Some(RustVersion::new("1.88.0")));
/// ```
///
/// [highest]: RustVersion::highest
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct RustVersion(String);

impl RustVersion {
    /// Creates a version from the text that names it
    pub fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }

    /// Returns the version as it was written
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Returns the version that a name states, or `None` when the name is
    /// not a version
    ///
    /// A version is a sequence of numbers that dots separate. A name such as
    /// `nightly-2026-08-11` is a toolchain and not a version, and it answers
    /// `None`, so a caller can tell the two apart.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_cargo::RustVersion;
    ///
    /// assert_eq!(RustVersion::parse("1.88.0"), Some(RustVersion::new("1.88.0")));
    /// assert_eq!(RustVersion::parse("nightly-2026-08-11"), None);
    /// ```
    // cargo[impl version.parse]
    pub fn parse(name: &str) -> Option<Self> {
        let numeric = !name.is_empty()
            && name
                .split(SEPARATOR)
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));

        numeric.then(|| Self::new(name))
    }

    /// Returns the highest of the versions, or `None` when there is none
    ///
    /// A caller that holds several versions and needs one asks for the
    /// newest of them: the newest pin of a project is the toolchain that it
    /// builds with, and the newest declaration of a workspace is the only
    /// toolchain that can compile every package in it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_cargo::RustVersion;
    ///
    /// assert_eq!(RustVersion::highest([]), None);
    /// ```
    // cargo[impl version.highest]
    pub fn highest(versions: impl IntoIterator<Item = Self>) -> Option<Self> {
        versions.into_iter().max_by(compare)
    }
}

impl std::fmt::Display for RustVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Returns which of two versions names the newer toolchain
///
/// The comparison reads the parts as numbers, because `1.9` comes before
/// `1.88` as text and after it as a version.
// cargo[impl version.compare]
fn compare(left: &RustVersion, right: &RustVersion) -> Ordering {
    let (left, right) = (parts(left), parts(right));

    for index in 0..left.len().max(right.len()) {
        match part(&left, index).cmp(&part(&right, index)) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    Ordering::Equal
}

/// Returns the number at an index of a version, and zero past its end
///
/// A version can leave the parts behind the first one out, and `1.88` and
/// `1.88.0` name one toolchain, so a part that is not there is a zero.
// cargo[impl version.compare]
fn part(parts: &[u64], index: usize) -> u64 {
    parts.get(index).copied().unwrap_or(0)
}

/// Returns the numbers of a version, in the order that it wrote them
///
/// Cargo refuses a manifest whose `rust-version` is not a version, so a part
/// that is not a number cannot come from a manifest that cargo read. Such a
/// part counts as zero, so that the comparison answers for every value that
/// a caller can build.
// cargo[impl version.compare]
fn parts(version: &RustVersion) -> Vec<u64> {
    version
        .0
        .split(SEPARATOR)
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // cargo[verify version.parse]
    #[test]
    fn parse_a_name_that_is_not_a_version_answers_nothing() {
        let version = RustVersion::parse("nightly-2026-08-11");

        assert_eq!(version, None);
    }

    // cargo[verify version.parse]
    #[test]
    fn parse_a_version_answers_it() {
        let version = RustVersion::parse("1.88.0");

        assert_eq!(version, Some(RustVersion::new("1.88.0")));
    }

    // cargo[verify version.highest]
    #[test]
    fn highest_of_no_version_answers_nothing() {
        let highest = RustVersion::highest([]);

        assert_eq!(highest, None);
    }

    // cargo[verify version.highest]
    #[test]
    fn highest_of_several_versions_answers_the_newest() {
        let versions = [
            RustVersion::new("1.85.0"),
            RustVersion::new("1.88.0"),
            RustVersion::new("1.87.0"),
        ];

        let highest = RustVersion::highest(versions);

        assert_eq!(highest, Some(RustVersion::new("1.88.0")));
    }

    // cargo[verify version.compare]
    #[test]
    fn highest_reads_a_part_as_a_number_and_not_as_text() {
        let versions = [RustVersion::new("1.9.0"), RustVersion::new("1.88.0")];

        let highest = RustVersion::highest(versions);

        assert_eq!(highest, Some(RustVersion::new("1.88.0")));
    }

    // cargo[verify version.compare]
    #[test]
    fn highest_counts_a_part_that_a_version_leaves_out_as_zero() {
        let versions = [RustVersion::new("1.88"), RustVersion::new("1.88.1")];

        let highest = RustVersion::highest(versions);

        assert_eq!(highest, Some(RustVersion::new("1.88.1")));
    }
}
