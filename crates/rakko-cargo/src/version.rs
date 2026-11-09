//! A version of the Rust compiler
//!
//! A question of this crate answers with a version: which toolchain the
//! packages of a workspace promise to compile on. It reads the version out
//! of text that cargo wrote, and it picks the newest of several, so this
//! module holds the version and the comparison that orders them.
//!
//! Nothing here fails. The comparison answers for every value that a caller
//! can build.

use std::cmp::Ordering;

/// The character that separates the parts of a version
const SEPARATOR: char = '.';

/// A version of the Rust compiler
///
/// A version names a release of the compiler, such as `1.88.0`. It reaches
/// the crate as text that cargo wrote: the version that a package declares
/// as the oldest one it compiles on.
///
/// The value carries that text as it was written, because it is the name
/// that rustup and a provisioning layer know the toolchain by. `1.88` and
/// `1.88.0` name one release and are two spellings, so they are two values
/// here.
///
/// The type has no order, because the order of the text is not the order of
/// the versions: `1.9` comes before `1.88` as text and after it as a
/// version.
///
/// # Examples
///
/// A caller reads the version that a workspace declares:
///
/// ```no_run
/// use rakko::Context;
/// use rakko_cargo::Cargo;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root("/home/otter/project").build();
/// let cargo = Cargo::resolve(&context).await?;
///
/// for root in cargo.roots().await? {
///     if let Some(version) = root.rust_version() {
///         println!("{}", version.get());
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct RustVersion(String);

impl RustVersion {
    /// Creates a version from the text that names it
    pub(crate) fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }

    /// Returns the version as it was written
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Returns the highest of the versions, or `None` when there is none
    ///
    /// A caller that holds several versions and needs one asks for the
    /// newest of them: the newest declaration of a workspace is the only
    /// toolchain that can compile every package in it.
    pub(crate) fn highest(versions: impl IntoIterator<Item = Self>) -> Option<Self> {
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
fn part(parts: &[u64], index: usize) -> u64 {
    parts.get(index).copied().unwrap_or(0)
}

/// Returns the numbers of a version, in the order that it wrote them
///
/// Cargo refuses a manifest whose `rust-version` is not a version, so a part
/// that is not a number cannot come from a manifest that cargo read. Such a
/// part counts as zero, so that the comparison answers for every value that
/// a caller can build.
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

    #[test]
    fn highest_of_no_version_answers_nothing() {
        let highest = RustVersion::highest([]);

        assert_eq!(highest, None);
    }

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

    #[test]
    fn highest_reads_a_part_as_a_number_and_not_as_text() {
        let versions = [RustVersion::new("1.9.0"), RustVersion::new("1.88.0")];

        let highest = RustVersion::highest(versions);

        assert_eq!(highest, Some(RustVersion::new("1.88.0")));
    }

    #[test]
    fn highest_counts_a_part_that_a_version_leaves_out_as_zero() {
        let versions = [RustVersion::new("1.88"), RustVersion::new("1.88.1")];

        let highest = RustVersion::highest(versions);

        assert_eq!(highest, Some(RustVersion::new("1.88.1")));
    }
}
