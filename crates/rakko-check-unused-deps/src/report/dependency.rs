use std::path::PathBuf;

use getset::{CopyGetters, Getters};
use rakko_action::{Finding, Location, ProjectRoot};
use rakko_cargo::CargoRoot;

use super::DependencyKind;

/// The word that opens the message of a finding
const UNUSED: &str = "unused";

/// One dependency that a package declares and no target of it loads
///
/// Cargo-udeps names the dependency, the manifest that declares it, and the
/// table of that manifest which declares it. All three travel together,
/// because a workspace holds more than one manifest, and one name can appear
/// in more than one table of the same manifest.
///
/// The manifest is the path that cargo-udeps reported, which is absolute. A
/// finding names it relative to the project root instead, which is the name
/// that a reader, a machine, and a code host all recognize.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct UnusedDependency {
    /// The table of the manifest that declares the dependency
    #[getset(get_copy = "pub")]
    kind: DependencyKind,

    /// The name of the dependency, as the manifest declares it
    #[getset(get = "pub")]
    name: String,

    /// The manifest of the package that declares the dependency
    #[getset(get = "pub")]
    manifest: PathBuf,
}

impl UnusedDependency {
    /// Creates an unused dependency from what cargo-udeps reported
    pub fn new(kind: DependencyKind, name: String, manifest: PathBuf) -> Self {
        Self {
            kind,
            name,
            manifest,
        }
    }

    /// Returns the finding that reports this dependency
    ///
    /// The finding belongs to the manifest that declares the dependency,
    /// because cargo-udeps reports no line of it, and the message names the
    /// dependency together with the kind, so that a reader finds the entry
    /// to remove.
    ///
    /// A manifest that lies outside the project gets a finding at the level
    /// of the project. The message then names the manifest, so the place is
    /// not lost with the level.
    // checkunuseddeps[impl check.finding]
    // checkunuseddeps[impl check.foreign]
    pub fn finding(&self, root: &CargoRoot, project: &ProjectRoot) -> Finding {
        let message = format!("{UNUSED} {}: {}", self.kind.label(), self.name);

        match root.relative_path(&self.manifest, project) {
            Some(path) => Finding::builder()
                .message(message)
                .location(Location::File { path })
                .build(),
            None => Finding::builder()
                .message(format!("{message} in {}", self.manifest.display()))
                .location(Location::Project)
                .build(),
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    /// Returns the root of the harness of the project
    fn harness() -> CargoRoot {
        CargoRoot::new(PathBuf::from("/home/otter/project/tools/harness"))
    }

    /// Returns the project root that the tests place a root in
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns an unused dependency that the given manifest declares
    fn unused(kind: DependencyKind, manifest: &str) -> UnusedDependency {
        UnusedDependency::new(kind, "serde".to_owned(), PathBuf::from(manifest))
    }

    // checkunuseddeps[verify check.finding]
    #[test]
    fn finding_names_the_dependency_and_its_kind() {
        let dependency = unused(
            DependencyKind::Development,
            "/home/otter/project/tools/harness/Cargo.toml",
        );

        let finding = dependency.finding(&harness(), &project());

        assert_eq!(finding.message().get(), "unused dev-dependency: serde");
    }

    // checkunuseddeps[verify check.finding]
    #[test]
    fn finding_names_the_manifest_relative_to_the_project() {
        let dependency = unused(
            DependencyKind::Normal,
            "/home/otter/project/tools/harness/Cargo.toml",
        );

        let finding = dependency.finding(&harness(), &project());

        assert!(
            matches!(
                finding.location(),
                Location::File { path } if path.get() == Path::new("tools/harness/Cargo.toml")
            ),
            "expected a path relative to the project, got {:?}",
            finding.location()
        );
    }

    // checkunuseddeps[verify check.foreign]
    #[test]
    fn finding_outside_the_project_belongs_to_the_project() {
        let dependency = unused(DependencyKind::Normal, "/home/otter/elsewhere/Cargo.toml");

        let finding = dependency.finding(&harness(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }

    // checkunuseddeps[verify check.foreign]
    #[test]
    fn finding_outside_the_project_names_the_manifest_in_the_message() {
        let dependency = unused(DependencyKind::Normal, "/home/otter/elsewhere/Cargo.toml");

        let finding = dependency.finding(&harness(), &project());

        assert_eq!(
            finding.message().get(),
            "unused dependency: serde in /home/otter/elsewhere/Cargo.toml"
        );
    }
}
