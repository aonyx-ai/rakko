//! One package that a report of cargo-deny is about
//!
//! Cargo-deny reports about the code that a workspace depends on, and it
//! names each piece of that code in the graph of every entry. This module
//! holds one of those names.

use std::fmt;

use bon::Builder;
use getset::Getters;

/// The character between the name and the version of a package in a message
const VERSION_SEPARATOR: char = ' ';

/// One package that a report of cargo-deny is about
///
/// The value holds a name and a version, because a dependency graph can hold
/// several versions of one package, and a report about a duplicate is about
/// all of them. The name alone would leave a reader with the question that
/// the report answers.
///
/// Cargo-deny calls this a crate, and so does its message. The type carries
/// the other name that cargo gives the same thing, because `crate` is a
/// keyword of Rust and no module can be named for it.
///
/// # Examples
///
/// ```
/// use rakko_check_dependencies::problem::Package;
///
/// let pkg = Package::builder()
///     .name("syn")
///     .version("2.0.0")
///     .build();
/// assert_eq!(pkg.to_string(), "syn 2.0.0");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, Getters)]
pub struct Package {
    /// The name of the package
    #[builder(into)]
    #[getset(get = "pub")]
    name: String,

    /// The version of the package
    #[builder(into)]
    #[getset(get = "pub")]
    version: String,
}

impl fmt::Display for Package {
    /// Writes the package the way that cargo-deny names one for a reader
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{VERSION_SEPARATOR}{}", self.name, self.version)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn display_names_the_package_and_its_version() {
        let name = Package::builder()
            .name("option-ext".to_owned())
            .version("0.2.0".to_owned())
            .build()
            .to_string();

        assert_eq!(name, "option-ext 0.2.0");
    }

    #[test]
    fn package_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Package>();
    }
}
