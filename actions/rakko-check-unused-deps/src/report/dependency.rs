//! One dependency that a package declares and never uses

use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko_action::{Finding, Location, ProjectRoot};
use rakko_cargo::CargoRoot;

use super::DependencyKind;

/// The word that opens the message of a finding
const UNUSED: &str = "unused";

/// One dependency that a package declares and no target of it loads
///
/// Cargo-udeps names the dependency, the manifest that declares it, and the
/// table of that manifest which declares it. All three travel together
/// because a workspace holds more than one manifest. One name can appear in
/// more than one table of the same manifest.
///
/// The manifest is the path that cargo-udeps reported. That path is absolute.
/// A finding names it relative to the project root instead. That is the name
/// that a reader, a machine, and a code host all recognize.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use rakko_check_unused_deps::{UnusedDependency, DependencyKind};
///
/// let dep = UnusedDependency::builder()
///     .kind(DependencyKind::Development)
///     .name("tempfile")
///     .manifest(PathBuf::from("/project/Cargo.toml"))
///     .build();
///
/// assert_eq!(dep.name(), "tempfile");
/// assert_eq!(dep.kind(), DependencyKind::Development);
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct UnusedDependency {
    /// The table of the manifest that declares the dependency
    #[getset(get_copy = "pub")]
    kind: DependencyKind,

    /// The name of the dependency, as the manifest declares it
    #[builder(into)]
    #[getset(get = "pub")]
    name: String,

    /// The manifest of the package that declares the dependency
    #[getset(get = "pub")]
    manifest: PathBuf,
}

impl UnusedDependency {
    /// Returns the finding that reports this dependency
    ///
    /// The finding belongs to the manifest that declares the dependency,
    /// because cargo-udeps reports no line of it. The message names the
    /// dependency together with the kind, so that a reader finds the entry to
    /// remove.
    ///
    /// A manifest that lies outside the project gets a finding at the level
    /// of the project. The message then names the manifest, so the place is
    /// not lost with the level.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_cargo::{CargoRoot, Documentation};
    /// use rakko_action::ProjectRoot;
    /// use rakko_check_unused_deps::{UnusedDependency, DependencyKind};
    /// use rakko_test_utils::path;
    ///
    /// let root = CargoRoot::builder()
    ///     .directory(path("/project"))
    ///     .documentation(Documentation::Testable)
    ///     .build();
    /// let project = ProjectRoot::new(path("/project"));
    /// let dep = UnusedDependency::builder()
    ///     .kind(DependencyKind::Normal)
    ///     .name("serde")
    ///     .manifest(path("/project/Cargo.toml"))
    ///     .build();
    ///
    /// let finding = dep.finding(&root, &project);
    /// assert!(finding.message().get().contains("unused"));
    /// ```
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::Path;

    use rakko_cargo::Documentation;

    use super::*;

    /// Returns the root of the harness of the project
    fn harness() -> CargoRoot {
        CargoRoot::builder()
            .directory(PathBuf::from("/home/otter/project/tools/harness"))
            .documentation(Documentation::Testable)
            .build()
    }

    /// Returns the project root that the tests place a root in
    fn project() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns an unused dependency that the given manifest declares
    fn unused(kind: DependencyKind, manifest: &str) -> UnusedDependency {
        UnusedDependency::builder()
            .kind(kind)
            .name("serde".to_owned())
            .manifest(PathBuf::from(manifest))
            .build()
    }

    #[test]
    fn finding_names_the_dependency_and_its_kind() {
        let dependency = unused(
            DependencyKind::Development,
            "/home/otter/project/tools/harness/Cargo.toml",
        );

        let finding = dependency.finding(&harness(), &project());

        assert_eq!(finding.message().get(), "unused dev-dependency: serde");
    }

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

    #[test]
    fn finding_outside_the_project_belongs_to_the_project() {
        let dependency = unused(DependencyKind::Normal, "/home/otter/elsewhere/Cargo.toml");

        let finding = dependency.finding(&harness(), &project());

        assert_eq!(finding.location(), &Location::Project);
    }

    #[test]
    fn finding_outside_the_project_names_the_manifest_in_the_message() {
        let dependency = unused(DependencyKind::Normal, "/home/otter/elsewhere/Cargo.toml");

        let finding = dependency.finding(&harness(), &project());

        assert_eq!(
            finding.message().get(),
            "unused dependency: serde in /home/otter/elsewhere/Cargo.toml"
        );
    }

    #[test]
    fn unused_dependency_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<UnusedDependency>();
    }
}
