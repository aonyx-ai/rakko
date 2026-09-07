/// The word that names a dependency of the normal kind
const DEPENDENCY: &str = "dependency";

/// The word that names a dependency that only the tests and the examples of
/// a package reach
const DEV_DEPENDENCY: &str = "dev-dependency";

/// The word that names a dependency that only the build script of a package
/// reaches
const BUILD_DEPENDENCY: &str = "build-dependency";

/// The kind of dependency that a manifest declares an entry as
///
/// A manifest declares a dependency in one of three tables, and the table
/// decides which targets of the package may reach the dependency. The kind
/// therefore belongs to a report of an unused dependency: a reader needs it
/// to find the entry, because one name can appear in more than one table of
/// the same manifest.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum DependencyKind {
    /// The package declares the dependency in `[dependencies]`
    Normal,

    /// The package declares the dependency in `[dev-dependencies]`
    Development,

    /// The package declares the dependency in `[build-dependencies]`
    Build,
}

impl DependencyKind {
    /// Returns the word that names this kind in a message
    ///
    /// The word is the singular of the table that declares the dependency,
    /// so that a reader of a finding can search the manifest for it.
    // checkunuseddeps[impl check.finding]
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => DEPENDENCY,
            Self::Development => DEV_DEPENDENCY,
            Self::Build => BUILD_DEPENDENCY,
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // checkunuseddeps[verify check.finding]
    #[test]
    fn label_of_a_build_dependency_names_its_table() {
        let label = DependencyKind::Build.label();

        assert_eq!(label, "build-dependency");
    }

    // checkunuseddeps[verify check.finding]
    #[test]
    fn label_of_a_development_dependency_names_its_table() {
        let label = DependencyKind::Development.label();

        assert_eq!(label, "dev-dependency");
    }

    // checkunuseddeps[verify check.finding]
    #[test]
    fn label_of_a_normal_dependency_names_no_table() {
        let label = DependencyKind::Normal.label();

        assert_eq!(label, "dependency");
    }
}
