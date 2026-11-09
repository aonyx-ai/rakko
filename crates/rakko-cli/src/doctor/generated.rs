use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::Path;

use rakko::{
    FilePath, Finding, Location, MiseTool, ProjectRoot, RequiredVersion, ToolRequirement,
    ToolVersion,
};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use super::check_error::CheckToolPinsError;
use super::repair_error::RepairToolPinsError;

/// The place of the generated file, relative to the root of the project
///
/// Mise merges every file of `.config/mise/conf.d` with the configuration of
/// the project, so the pins of the file reach an install, an editor, and a
/// run of the tool by hand. Mise locks the file in `.config/mise/mise.lock`.
pub(crate) const PATH: &str = ".config/mise/conf.d/rakko.toml";

/// The place of the lockfile that mise writes for the generated file
///
/// Mise writes one lockfile for every configuration file in
/// `.config/mise`, so the project can share it with a file of its own, such
/// as `.config/mise/config.toml`.
const LOCKFILE: &str = ".config/mise/mise.lock";

/// The directory of the configuration files that share the lockfile
const DIRECTORY: &str = ".config/mise";

/// The directory of the configuration files that mise merges, inside
/// [`DIRECTORY`]
const MERGED: &str = "conf.d";

/// The extension of a configuration file of mise
const TOML: &str = "toml";

/// The comment at the top of the generated file
///
/// A reader of the project finds the file beside its own configuration, so
/// the comment says where the file comes from and how it changes.
const HEADER: &str = "\
# Rakko generates this file from the tools that the actions of the harness
# declare. Do not edit it, because `mise run rakko -- doctor --fix` writes it
# again.
";

/// The message of a generated file that does not exist
const MISSING: &str =
    "the generated file of the tool pins is missing, and `doctor --fix` writes it";

/// The message of a generated file in a project whose actions declare no tool
const UNNEEDED: &str = "the generated file of the tool pins is left over, because no mounted \
                        action declares a tool, and `doctor --fix` removes it";

/// The message of a generated file whose pins differ from the declarations
const STALE: &str = "the generated file of the tool pins does not match the tools that the mounted \
                     actions declare, and `doctor --fix` writes it again";

/// The exact versions of the tools that the mounted actions declare
///
/// A toolchain is a decision of the project, so its range does not reach the
/// generated file. Two actions can pin two versions of one tool. The file then
/// pins both, and each action finds its own version.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub(crate) struct Pins(BTreeMap<MiseTool, BTreeSet<ToolVersion>>);

impl Pins {
    /// Returns the exact versions of the requirements
    pub(crate) fn of<'a>(requirements: impl IntoIterator<Item = &'a ToolRequirement>) -> Self {
        let mut pins: BTreeMap<MiseTool, BTreeSet<ToolVersion>> = BTreeMap::new();

        for requirement in requirements {
            if let RequiredVersion::Exact(version) = requirement.version() {
                pins.entry(requirement.tool().clone())
                    .or_default()
                    .insert(version.clone());
            }
        }

        Self(pins)
    }

    /// Returns whether the mounted actions pin the tool
    pub(crate) fn contains(&self, tool: &str) -> bool {
        self.0.keys().any(|pinned| pinned.get() == tool)
    }

    /// Returns whether the mounted actions pin no tool
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the content of the generated file, or `None` when the project
    /// needs no file
    ///
    /// The tools are in alphabetical order, so the same declarations give the
    /// same file on every machine.
    fn render(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut tools = Table::new();

        for (tool, versions) in &self.0 {
            let value = match versions.first() {
                Some(version) if versions.len() == 1 => Value::from(version.get()),
                _ => Value::Array(versions.iter().map(ToolVersion::get).collect::<Array>()),
            };

            tools.insert(tool.get(), Item::Value(value));
        }

        let mut document = DocumentMut::new();
        document.insert("tools", Item::Table(tools));

        Some(format!("{HEADER}\n{document}"))
    }
}

/// Returns the problem of the generated file, or `None` when the file
/// matches the pins
///
/// A project whose actions declare no tool needs no file, so the check finds
/// nothing there when the file does not exist, and it reports a file that
/// is left over. The comparison ignores the line endings, because Git can
/// check the file out with the line endings of Windows.
///
/// # Errors
///
/// Returns [`CheckToolPinsError::UnreadableGeneratedFile`] when the file
/// exists and the file system does not give its content.
pub(crate) fn check(
    pins: &Pins,
    root: &ProjectRoot,
) -> Result<Option<Finding>, CheckToolPinsError> {
    let path = root.get().join(PATH);
    let expected = pins.render();

    let message = match (std::fs::read_to_string(&path), expected) {
        (Ok(_), None) => UNNEEDED,
        (Ok(actual), Some(expected)) if actual.replace("\r\n", "\n") == expected => {
            return Ok(None);
        }
        (Ok(_), Some(_)) => STALE,
        (Err(error), None) if error.kind() == ErrorKind::NotFound => return Ok(None),
        (Err(error), Some(_)) if error.kind() == ErrorKind::NotFound => MISSING,
        (Err(source), _) => {
            return Err(CheckToolPinsError::UnreadableGeneratedFile { path, source });
        }
    };

    Ok(Some(finding(message)))
}

/// Writes the generated file of the pins, or removes it when the project
/// needs none
///
/// The write creates the directory of the file when it does not exist. A
/// project whose actions declare no tool needs no file, so the write removes
/// a file that is left over.
///
/// # Errors
///
/// Returns [`RepairToolPinsError::UnwritableFile`] when the file system
/// refuses to create the directory, to write the file, or to remove it.
pub(crate) fn write(pins: &Pins, root: &ProjectRoot) -> Result<(), RepairToolPinsError> {
    let path = root.get().join(PATH);
    let unwritable = |source| RepairToolPinsError::UnwritableFile {
        path: path.clone(),
        source,
    };

    let Some(content) = pins.render() else {
        return std::fs::remove_file(&path).map_err(unwritable);
    };

    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory).map_err(unwritable)?;
    }

    std::fs::write(&path, content).map_err(unwritable)
}

/// Removes the lockfile of the generated file when no other configuration
/// file shares it, and returns whether one does
///
/// Mise prunes the entries of a removed file from the lockfile only when
/// another configuration file in `.config/mise` remains, such as
/// `.config/mise/config.toml`. Without one, it leaves the lockfile as it is,
/// with the pins of the removed file. So the repair removes the lockfile
/// when no TOML file is left in `.config/mise` or in `.config/mise/conf.d`.
/// When one is left, the caller locks the tools, and mise rewrites the
/// lockfile from what is left.
///
/// # Errors
///
/// Returns [`RepairToolPinsError::UnreadableConfiguration`] when the file
/// system does not list one of the two directories, and
/// [`RepairToolPinsError::UnwritableFile`] when it refuses to remove the
/// lockfile.
pub(crate) fn retire_lockfile(root: &ProjectRoot) -> Result<bool, RepairToolPinsError> {
    let directory = root.get().join(DIRECTORY);

    for listed in [directory.clone(), directory.join(MERGED)] {
        if holds_configuration(&listed)? {
            return Ok(true);
        }
    }

    let path = root.get().join(LOCKFILE);

    match std::fs::remove_file(&path) {
        Err(error) if error.kind() != ErrorKind::NotFound => {
            Err(RepairToolPinsError::UnwritableFile {
                path,
                source: error,
            })
        }
        _ => Ok(false),
    }
}

/// Returns whether a directory holds a TOML file
///
/// A directory that does not exist holds none.
///
/// # Errors
///
/// Returns [`RepairToolPinsError::UnreadableConfiguration`] when the file
/// system does not list the directory.
fn holds_configuration(directory: &Path) -> Result<bool, RepairToolPinsError> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(RepairToolPinsError::UnreadableConfiguration {
                path: directory.to_path_buf(),
                source,
            });
        }
    };

    Ok(entries.flatten().any(|entry| {
        let path = entry.path();

        path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some(TOML)
    }))
}

/// Returns the finding of a problem of the generated file
///
/// The place of the file is a constant that is a valid relative path, so the
/// finding always gives the file.
fn finding(message: &str) -> Finding {
    let location = FilePath::try_from(Path::new(PATH))
        .map_or(Location::Project, |path| Location::File { path });

    Finding::builder()
        .message(message)
        .location(location)
        .build()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko::VersionRange;
    use tempfile::TempDir;

    use super::*;

    /// Returns the requirement of a tool at an exact version
    fn exact(tool: &str, version: &str) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(tool)
            .executable(tool)
            .version(RequiredVersion::Exact(ToolVersion::new(version)))
            .build()
    }

    /// Returns the requirement of a toolchain in a range
    fn range(tool: &str, range: &str) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(tool)
            .executable(tool)
            .version(RequiredVersion::Range(VersionRange::new(range)))
            .build()
    }

    /// Returns the message of the problem that the check finds in a project
    /// whose generated file holds the text, or that has no file
    fn problem_of(pins: &Pins, text: Option<&str>) -> Option<String> {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let root = ProjectRoot::from(directory.path());

        if let Some(text) = text {
            let path = directory.path().join(PATH);
            std::fs::create_dir_all(path.parent().expect("the file has a directory"))
                .expect("the test creates the directory of the file");
            std::fs::write(path, text).expect("the test writes the file");
        }

        check(pins, &root)
            .expect("the test project has a readable file")
            .map(|finding| finding.message().get().to_owned())
    }

    /// Returns a project whose generated file is gone and whose lockfile
    /// remains, with an empty directory of merged files
    fn locked_project() -> (TempDir, ProjectRoot) {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::create_dir_all(directory.path().join(DIRECTORY).join(MERGED))
            .expect("the test creates the directories");
        std::fs::write(directory.path().join(LOCKFILE), "lockfile_version = 3\n")
            .expect("the test writes the lockfile");
        let root = ProjectRoot::from(directory.path());

        (directory, root)
    }

    /// Returns the pins of one tool at an exact version
    fn taplo() -> Pins {
        Pins::of(&[exact("taplo", "0.10.0")])
    }

    #[test]
    fn check_with_a_current_file_finds_nothing() {
        let pins = taplo();
        let text = pins.render().expect("the pins give a file");

        let problem = problem_of(&pins, Some(&text));

        assert_eq!(problem, None);
    }

    #[test]
    fn check_with_a_current_file_in_the_line_endings_of_windows_finds_nothing() {
        let pins = taplo();
        let text = pins
            .render()
            .expect("the pins give a file")
            .replace('\n', "\r\n");

        let problem = problem_of(&pins, Some(&text));

        assert_eq!(problem, None);
    }

    #[test]
    fn check_with_a_directory_in_place_of_the_file_reports_an_error() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::create_dir_all(directory.path().join(PATH))
            .expect("the test creates a directory in place of the file");

        let result = check(&taplo(), &ProjectRoot::from(directory.path()));

        assert!(
            matches!(
                result,
                Err(CheckToolPinsError::UnreadableGeneratedFile { .. })
            ),
            "expected an unreadable file, got {result:?}"
        );
    }

    #[test]
    fn check_with_a_file_and_no_pins_reports_it_left_over() {
        let problem = problem_of(&Pins::default(), Some("[tools]\ntaplo = \"0.9.0\"\n"));

        assert_eq!(problem.as_deref(), Some(UNNEEDED));
    }

    #[test]
    fn check_with_a_file_of_other_pins_reports_it_stale() {
        let problem = problem_of(&taplo(), Some("[tools]\ntaplo = \"0.9.0\"\n"));

        assert_eq!(problem.as_deref(), Some(STALE));
    }

    #[test]
    fn check_with_no_file_and_no_pins_finds_nothing() {
        let problem = problem_of(&Pins::default(), None);

        assert_eq!(problem, None);
    }

    #[test]
    fn check_with_no_file_reports_it_missing() {
        let problem = problem_of(&taplo(), None);

        assert_eq!(problem.as_deref(), Some(MISSING));
    }

    #[test]
    fn contains_with_a_tool_that_no_action_pins_is_false() {
        let pins = taplo();

        assert!(!pins.contains("jq"));
    }

    #[test]
    fn of_leaves_out_a_toolchain() {
        let pins = Pins::of(&[range("node", ">=22")]);

        assert!(pins.is_empty());
    }

    #[test]
    fn render_pins_each_tool_in_alphabetical_order() {
        let pins = Pins::of(&[
            exact("taplo", "0.10.0"),
            exact("npm:markdownlint-cli", "0.50.0"),
        ]);

        let text = pins.render().expect("the pins give a file");

        assert!(
            text.ends_with("[tools]\n\"npm:markdownlint-cli\" = \"0.50.0\"\ntaplo = \"0.10.0\"\n"),
            "got {text}"
        );
    }

    #[test]
    fn render_pins_two_versions_of_one_tool_as_an_array() {
        let pins = Pins::of(&[exact("taplo", "0.9.0"), exact("taplo", "0.10.0")]);

        let text = pins.render().expect("the pins give a file");

        assert!(
            text.ends_with("taplo = [\"0.10.0\", \"0.9.0\"]\n"),
            "got {text}"
        );
    }

    #[test]
    fn render_starts_with_the_header() {
        let text = taplo().render().expect("the pins give a file");

        assert!(text.starts_with(HEADER), "got {text}");
    }

    #[test]
    fn write_creates_the_file_that_the_check_accepts() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let root = ProjectRoot::from(directory.path());
        let pins = taplo();

        write(&pins, &root).expect("the test project is writable");

        assert!(matches!(check(&pins, &root), Ok(None)));
    }

    #[test]
    fn retire_lockfile_with_a_configuration_beside_it_keeps_the_lockfile() {
        let (directory, root) = locked_project();
        std::fs::write(directory.path().join(DIRECTORY).join("config.toml"), "")
            .expect("the test writes the configuration");

        let shared = retire_lockfile(&root).expect("the test project is writable");

        assert_eq!(
            (shared, directory.path().join(LOCKFILE).exists()),
            (true, true)
        );
    }

    #[test]
    fn retire_lockfile_with_another_merged_file_keeps_the_lockfile() {
        let (directory, root) = locked_project();
        std::fs::write(
            directory
                .path()
                .join(DIRECTORY)
                .join(MERGED)
                .join("other.toml"),
            "",
        )
        .expect("the test writes the configuration");

        let shared = retire_lockfile(&root).expect("the test project is writable");

        assert_eq!(
            (shared, directory.path().join(LOCKFILE).exists()),
            (true, true)
        );
    }

    #[test]
    fn retire_lockfile_without_a_configuration_removes_the_lockfile() {
        let (directory, root) = locked_project();

        let shared = retire_lockfile(&root).expect("the test project is writable");

        assert_eq!(
            (shared, directory.path().join(LOCKFILE).exists()),
            (false, false)
        );
    }

    #[test]
    fn retire_lockfile_without_a_lockfile_reports_nothing_shared() {
        let directory = TempDir::new().expect("the test creates a temporary directory");

        let shared = retire_lockfile(&ProjectRoot::from(directory.path()));

        assert!(matches!(shared, Ok(false)), "got {shared:?}");
    }

    #[test]
    fn write_without_pins_removes_the_file() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let root = ProjectRoot::from(directory.path());
        write(&taplo(), &root).expect("the test project is writable");

        write(&Pins::default(), &root).expect("the test project is writable");

        assert!(!directory.path().join(PATH).exists());
    }
}
