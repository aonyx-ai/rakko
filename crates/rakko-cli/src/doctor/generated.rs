//! The mise file that Rakko generates from the tools of the mounted actions

use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::Path;

use rakko::{
    FilePath, Finding, Location, MiseTool, Platform, ProjectRoot, RequiredVersion, ToolRequirement,
    ToolVersion,
};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Key, Table, Value};

use super::check_error::CheckToolPinsError;
use super::repair_error::RepairToolPinsError;

/// The place of the generated file, relative to the root of the project
///
/// Mise merges every file of `.config/mise/conf.d` with the configuration of
/// the project. So the pins of the file reach an install, an editor, and a
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

/// The key of the version in the table of a pin with platforms
const VERSION: &str = "version";

/// The key of the platforms in the table of a pin, which mise calls the `os`
/// option
const OS: &str = "os";

/// The widest line of a list that taplo keeps on one line
///
/// This is the limit of the default settings of the formatter taplo. When the
/// line of a list is wider, taplo puts each element on a line of its own.
/// When a list fits, taplo puts it back on one line.
const COLUMNS: usize = 80;

/// The text before an element of a list that has each element on a line of
/// its own
const INDENT: &str = "\n  ";

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

/// The exact versions of the tools that the mounted actions declare, each
/// with the platforms where the tool has a build
///
/// A toolchain is a decision of the project, so its range does not reach the
/// generated file. Two actions can pin two versions of one tool. The file then
/// pins both, and each action finds its own version.
///
/// A version without a list of platforms is needed on every platform. A
/// version with a list is needed only there, and mise installs and locks it
/// only on those platforms. When two actions pin the same version, the file
/// needs the version wherever one of them needs it. So the lists merge, and a
/// version that one action needs on every platform gets no list.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub(crate) struct Pins(BTreeMap<MiseTool, BTreeMap<ToolVersion, Option<BTreeSet<Platform>>>>);

impl Pins {
    /// Returns the exact versions of the requirements, with their platforms
    pub(crate) fn of<'a>(requirements: impl IntoIterator<Item = &'a ToolRequirement>) -> Self {
        let mut pins: BTreeMap<MiseTool, BTreeMap<ToolVersion, Option<BTreeSet<Platform>>>> =
            BTreeMap::new();

        for requirement in requirements {
            let RequiredVersion::Exact(version) = requirement.version() else {
                continue;
            };

            let platforms = requirement.platforms().cloned();
            let versions = pins.entry(requirement.tool().clone()).or_default();

            match versions.get_mut(version) {
                Some(merged) => *merged = merge(merged.take(), platforms),
                None => {
                    versions.insert(version.clone(), platforms);
                }
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
    /// The tools, their versions, and their platforms are in a fixed order, so
    /// the same declarations give the same file on every machine. The layout
    /// is the one that the formatter taplo gives with its default settings. So
    /// a project that checks its TOML files with taplo and its default
    /// settings accepts the file that `doctor --fix` writes.
    fn render(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut tools = Table::new();

        for (tool, versions) in &self.0 {
            tools.insert(tool.get(), entry(tool, versions));
        }

        let mut document = DocumentMut::new();
        document.insert("tools", Item::Table(tools));

        Some(format!("{HEADER}\n{document}"))
    }
}

/// Returns the entry of one tool in the table of the tools
///
/// A tool whose versions have no platforms is a key of the table of the
/// tools. Its value is a string for one version, and a list for more.
///
/// A tool with platforms for one of its versions gets a table of its own,
/// such as `[tools.taplo]`. The table has the `version` and the `os` keys.
/// Mise reads this form for a tool with options. More versions of such a tool
/// get an array of tables, such as `[[tools.taplo]]`, with one table for each
/// version. The tables come after the keys of the table of the tools,
/// because TOML puts the keys of a table before its subtables.
fn entry(tool: &MiseTool, versions: &BTreeMap<ToolVersion, Option<BTreeSet<Platform>>>) -> Item {
    if versions.values().all(Option::is_none) {
        let mut values = versions.keys().map(|version| Value::from(version.get()));

        return match (values.next(), versions.len()) {
            (Some(value), 1) => Item::Value(value),
            (first, _) => Item::Value(Value::Array(list(
                tool.get(),
                first.into_iter().chain(values),
            ))),
        };
    }

    let mut tables = versions
        .iter()
        .map(|(version, platforms)| table(version, platforms.as_ref()));

    match (tables.next(), versions.len()) {
        (Some(table), 1) => Item::Table(table),
        (first, _) => {
            let mut array = ArrayOfTables::new();
            first
                .into_iter()
                .chain(tables)
                .for_each(|table| array.push(table));

            Item::ArrayOfTables(array)
        }
    }
}

/// Returns the table of one pinned version in the generated file
///
/// The table holds the `os` option of mise only when the version has
/// platforms. A blank line comes before the header of the table, so that the
/// tables of the file stay apart.
fn table(version: &ToolVersion, platforms: Option<&BTreeSet<Platform>>) -> Table {
    let mut table = Table::new();
    table.decor_mut().set_prefix("\n");
    table.insert(VERSION, toml_edit::value(version.get()));

    if let Some(platforms) = platforms {
        let platforms = platforms
            .iter()
            .map(|platform| Value::from(platform.to_string()));
        table.insert(OS, Item::Value(Value::Array(list(OS, platforms))));
    }

    table
}

/// Returns the list of the values of a key
///
/// The list is on one line when the line of the key fits in [`COLUMNS`].
/// Otherwise, each value is on a line of its own, with a comma after the
/// last value, as taplo writes a long list.
fn list(key: &str, values: impl IntoIterator<Item = Value>) -> Array {
    let mut array = values.into_iter().collect::<Array>();
    let line = format!("{} = {array}", Key::new(key).display_repr());

    if line.chars().count() > COLUMNS {
        for value in array.iter_mut() {
            value.decor_mut().set_prefix(INDENT);
            value.decor_mut().set_suffix("");
        }

        array.set_trailing_comma(true);
        array.set_trailing("\n");
    }

    array
}

/// Returns the platforms of a version that two actions pin
///
/// The version is needed wherever one of the actions needs it, so a version
/// without a list on either side gets no list.
fn merge(
    first: Option<BTreeSet<Platform>>,
    second: Option<BTreeSet<Platform>>,
) -> Option<BTreeSet<Platform>> {
    match (first, second) {
        (Some(mut first), Some(second)) => {
            first.extend(second);
            Some(first)
        }
        (None, _) | (_, None) => None,
    }
}

/// Returns the problem of the generated file, or `None` when the file
/// matches the pins
///
/// A project whose actions declare no tool needs no file. So the check finds
/// nothing there when the file does not exist, and it reports a file that is
/// left over.
///
/// The check compares the values that the file holds, and not its text. A
/// formatter of the project can change the layout of the file. For example,
/// a formatter with settings other than its defaults can put each platform
/// of a pin on a line of its own. Git can check
/// the file out with the line endings of Windows. Neither change makes the
/// file stale, so `doctor --fix` and the formatter do not undo each other. A
/// file that is not TOML is stale.
///
/// # Errors
///
/// Returns [`CheckToolPinsError::UnreadableGeneratedFile`] when the file
/// exists and the file system does not give its content.
#[bon::builder]
pub(crate) fn check(
    pins: &Pins,
    root: &ProjectRoot,
) -> Result<Option<Finding>, CheckToolPinsError> {
    let path = root.get().join(PATH);
    let expected = pins.render();

    let message = match (std::fs::read_to_string(&path), expected) {
        (Ok(_), None) => UNNEEDED,
        (Ok(actual), Some(expected))
            if values(&actual).is_some_and(|actual| Some(actual) == values(&expected)) =>
        {
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

/// Returns the values of a TOML text, or `None` when the text is not TOML
///
/// The values leave out the layout, the comments, and the line endings of the
/// text. So two texts that differ only in these hold the same values.
fn values(text: &str) -> Option<serde_json::Value> {
    toml_edit::de::from_str(text).ok()
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
#[bon::builder]
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
        Ok(()) | Err(_) => Ok(false),
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
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{Architecture, OperatingSystem, VersionRange};
    use tempfile::TempDir;

    use super::*;

    /// Returns the requirement of a tool at an exact version on the platforms
    #[bon::builder]
    fn limited(tool: &str, version: &str, platforms: &[Platform]) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(tool)
            .executable(tool)
            .exact_version_on(
                ToolVersion::new(version),
                platforms.iter().copied().collect::<BTreeSet<_>>(),
            )
            .build()
    }

    /// Returns the generated file of the requirements, without its header
    ///
    /// The tests of the layout compare the file with the text that taplo
    /// 0.10.0 leaves unchanged with its default settings, which
    /// `taplo fmt --check` confirmed by hand. No test runs taplo, because this
    /// crate pins no version of it.
    fn rendered(requirements: &[ToolRequirement]) -> String {
        Pins::of(requirements)
            .render()
            .expect("the pins give a file")
            .strip_prefix(HEADER)
            .expect("the file starts with the header")
            .to_owned()
    }

    /// Returns a version of a tool, and the generated file of the tool at
    /// 0.9.0 and at that version, without its header
    ///
    /// With the version, the line of the list of the two versions has as many
    /// columns as the test gives. The file puts a key with a colon in
    /// quotation marks, so the count includes them.
    fn two_versions(tool: &str, columns: usize) -> (String, String) {
        let key = if tool.contains(':') {
            format!("\"{tool}\"")
        } else {
            tool.to_owned()
        };
        let line = format!("{key} = [\"0.9.0\", \"1.0.0-\"]").len();
        let version = format!("1.0.0-{}", "a".repeat(columns - line));

        let text = rendered(&[
            exact().tool(tool).version("0.9.0").call(),
            exact().tool(tool).version(&version).call(),
        ]);

        (version, text)
    }

    /// The platform of Linux, on every architecture
    const LINUX: Platform = Platform::new(OperatingSystem::Linux);

    /// The platform of macOS on ARM
    const MACOS_ARM64: Platform =
        Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64);

    /// Every platform with an architecture, whose list is wider than the
    /// columns that taplo allows
    const EVERY_ARCHITECTURE: [Platform; 6] = [
        Platform::new(OperatingSystem::Linux).with_architecture(Architecture::X64),
        Platform::new(OperatingSystem::Linux).with_architecture(Architecture::Arm64),
        Platform::new(OperatingSystem::Macos).with_architecture(Architecture::X64),
        MACOS_ARM64,
        Platform::new(OperatingSystem::Windows).with_architecture(Architecture::X64),
        Platform::new(OperatingSystem::Windows).with_architecture(Architecture::Arm64),
    ];

    /// Returns the requirement of a tool at an exact version
    #[bon::builder]
    fn exact(tool: &str, version: &str) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(tool)
            .executable(tool)
            .version(RequiredVersion::Exact(ToolVersion::new(version)))
            .build()
    }

    /// Returns the requirement of a toolchain in a range
    #[bon::builder]
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

        check()
            .pins(pins)
            .root(&root)
            .call()
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
        Pins::of(&[exact().tool("taplo").version("0.10.0").call()])
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

    /// Shows that a formatter of the project and `doctor --fix` do not undo
    /// each other
    ///
    /// `doctor --fix` writes the short list of platforms of stiles on one
    /// line. A formatter with settings other than its defaults can spread the
    /// list over several lines. The check must accept the file that the
    /// formatter left, or `doctor --fix` writes it again and the formatter
    /// changes it back.
    #[test]
    fn check_with_a_current_file_that_a_formatter_wrapped_finds_nothing() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let root = ProjectRoot::from(directory.path());
        let pins = Pins::of(&[limited()
            .tool("github:aonyx-ai/stiles")
            .version("stiles@0.6.0")
            .platforms(&[LINUX, MACOS_ARM64])
            .call()]);
        write()
            .pins(&pins)
            .root(&root)
            .call()
            .expect("the test project is writable");
        let path = directory.path().join(PATH);
        let written = std::fs::read_to_string(&path).expect("the test reads the written file");
        let wrapped = written.replace(
            "os = [\"linux\", \"macos/arm64\"]\n",
            "os = [\n  \"linux\",\n  \"macos/arm64\",\n]\n",
        );
        std::fs::write(&path, &wrapped).expect("the test writes the wrapped file");

        let problem = check()
            .pins(&pins)
            .root(&root)
            .call()
            .expect("the test project has a readable file");

        assert!(
            wrapped != written && problem.is_none(),
            "expected no problem in the wrapped file, got {problem:?} for {wrapped}"
        );
    }

    #[test]
    fn check_with_a_directory_in_place_of_the_file_reports_an_error() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::create_dir_all(directory.path().join(PATH))
            .expect("the test creates a directory in place of the file");

        let result = check()
            .pins(&taplo())
            .root(&ProjectRoot::from(directory.path()))
            .call();

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
    fn check_with_a_file_that_is_no_toml_reports_it_stale() {
        let problem = problem_of(&taplo(), Some("[tools\ntaplo = \"0.10.0\"\n"));

        assert_eq!(problem.as_deref(), Some(STALE));
    }

    /// Shows that a file of an earlier layout stays current
    ///
    /// Before the layout of taplo, `doctor --fix` wrote a pin with platforms
    /// as an inline table. Such a file holds the same values as the table of
    /// its own that the pin has now.
    #[test]
    fn check_with_a_pin_with_platforms_in_an_inline_table_finds_nothing() {
        let pins = Pins::of(&[limited()
            .tool("stiles")
            .version("0.6.0")
            .platforms(&[LINUX])
            .call()]);

        let problem = problem_of(
            &pins,
            Some("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\"] }\n"),
        );

        assert_eq!(problem, None);
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
        let pins = Pins::of(&[range().tool("node").range(">=22").call()]);

        assert!(pins.is_empty());
    }

    #[test]
    fn pins_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Pins>();
    }

    #[test]
    fn render_counts_the_quotation_marks_of_a_key_in_the_width_of_its_line() {
        let (long, text) = two_versions("npm:taplo", 81);

        assert_eq!(
            text,
            format!("\n[tools]\n\"npm:taplo\" = [\n  \"0.9.0\",\n  \"{long}\",\n]\n")
        );
    }

    #[test]
    fn render_keeps_a_list_of_80_columns_on_one_line() {
        let (long, text) = two_versions("taplo", 80);

        assert_eq!(
            text,
            format!("\n[tools]\ntaplo = [\"0.9.0\", \"{long}\"]\n")
        );
    }

    #[test]
    fn render_merges_the_platforms_of_one_version() {
        let text = rendered(&[
            limited()
                .tool("stiles")
                .version("0.6.0")
                .platforms(&[MACOS_ARM64])
                .call(),
            limited()
                .tool("stiles")
                .version("0.6.0")
                .platforms(&[LINUX])
                .call(),
        ]);

        assert_eq!(
            text,
            "\n[tools]\n\n[tools.stiles]\nversion = \"0.6.0\"\nos = [\"linux\", \"macos/arm64\"]\n"
        );
    }

    #[test]
    fn render_of_one_version_on_every_platform_and_on_some_lists_none() {
        let text = rendered(&[
            limited()
                .tool("stiles")
                .version("0.6.0")
                .platforms(&[LINUX])
                .call(),
            exact().tool("stiles").version("0.6.0").call(),
        ]);

        assert_eq!(text, "\n[tools]\nstiles = \"0.6.0\"\n");
    }

    #[test]
    fn render_of_one_version_on_some_platforms_and_on_every_platform_lists_none() {
        let text = rendered(&[
            exact().tool("stiles").version("0.6.0").call(),
            limited()
                .tool("stiles")
                .version("0.6.0")
                .platforms(&[LINUX])
                .call(),
        ]);

        assert_eq!(text, "\n[tools]\nstiles = \"0.6.0\"\n");
    }

    #[test]
    fn render_pins_a_tool_on_its_platforms_as_a_table_after_the_plain_pins() {
        let text = rendered(&[
            limited()
                .tool("github:aonyx-ai/stiles")
                .version("stiles@0.6.0")
                .platforms(&[MACOS_ARM64, LINUX])
                .call(),
            exact().tool("taplo").version("0.10.0").call(),
        ]);

        assert_eq!(
            text,
            "\n[tools]\ntaplo = \"0.10.0\"\n\n[tools.\"github:aonyx-ai/stiles\"]\nversion = \"stiles@0.6.0\"\nos = [\"linux\", \"macos/arm64\"]\n"
        );
    }

    #[test]
    fn render_pins_each_tool_in_alphabetical_order() {
        let pins = Pins::of(&[
            exact().tool("taplo").version("0.10.0").call(),
            exact()
                .tool("npm:markdownlint-cli")
                .version("0.50.0")
                .call(),
        ]);

        let text = pins.render().expect("the pins give a file");

        assert!(
            text.ends_with("[tools]\n\"npm:markdownlint-cli\" = \"0.50.0\"\ntaplo = \"0.10.0\"\n"),
            "got {text}"
        );
    }

    #[test]
    fn render_pins_two_versions_of_one_tool_as_an_array() {
        let pins = Pins::of(&[
            exact().tool("taplo").version("0.9.0").call(),
            exact().tool("taplo").version("0.10.0").call(),
        ]);

        let text = pins.render().expect("the pins give a file");

        assert!(
            text.ends_with("taplo = [\"0.10.0\", \"0.9.0\"]\n"),
            "got {text}"
        );
    }

    #[test]
    fn render_pins_two_versions_with_and_without_platforms_as_an_array_of_tables() {
        let text = rendered(&[
            limited()
                .tool("taplo")
                .version("0.9.0")
                .platforms(&[LINUX])
                .call(),
            exact().tool("taplo").version("0.10.0").call(),
        ]);

        assert_eq!(
            text,
            "\n[tools]\n\n[[tools.taplo]]\nversion = \"0.10.0\"\n\n[[tools.taplo]]\nversion = \"0.9.0\"\nos = [\"linux\"]\n"
        );
    }

    #[test]
    fn render_spreads_a_list_of_81_columns() {
        let (long, text) = two_versions("taplo", 81);

        assert_eq!(
            text,
            format!("\n[tools]\ntaplo = [\n  \"0.9.0\",\n  \"{long}\",\n]\n")
        );
    }

    #[test]
    fn render_spreads_a_list_of_platforms_wider_than_80_columns() {
        let text = rendered(&[limited()
            .tool("stiles")
            .version("0.6.0")
            .platforms(&EVERY_ARCHITECTURE)
            .call()]);

        assert_eq!(
            text,
            "\n[tools]\n\n[tools.stiles]\nversion = \"0.6.0\"\nos = [\n  \"linux/x64\",\n  \"linux/arm64\",\n  \"macos/x64\",\n  \"macos/arm64\",\n  \"windows/x64\",\n  \"windows/arm64\",\n]\n"
        );
    }

    #[test]
    fn render_starts_with_the_header() {
        let text = taplo().render().expect("the pins give a file");

        assert!(text.starts_with(HEADER), "got {text}");
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
    fn write_creates_the_file_that_the_check_accepts() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let root = ProjectRoot::from(directory.path());
        let pins = taplo();

        write()
            .pins(&pins)
            .root(&root)
            .call()
            .expect("the test project is writable");

        assert!(matches!(check().pins(&pins).root(&root).call(), Ok(None)));
    }

    #[test]
    fn write_without_pins_removes_the_file() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let root = ProjectRoot::from(directory.path());
        write()
            .pins(&taplo())
            .root(&root)
            .call()
            .expect("the test project is writable");

        write()
            .pins(&Pins::default())
            .root(&root)
            .call()
            .expect("the test project is writable");

        assert!(!directory.path().join(PATH).exists());
    }
}
