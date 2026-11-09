//! The pins of the project that override a declared tool

use std::path::{Path, PathBuf};

use rakko::{Context, FilePath, Finding, Location, MiseTool, ProjectRoot};
use rakko_tool::Invocation;
use serde::Deserialize;
use toml_edit::{DocumentMut, Item};

use super::check_error::CheckToolPinsError;
use super::generated::{self, Pins};
use super::repair_error::RepairToolPinsError;
use super::{MISE, diagnosis};

/// The arguments that ask mise for the configuration files that apply in a
/// directory, in JSON
const LIST: [&str; 3] = ["config", "ls", "--json"];

/// The key of the table of the tools in a configuration file of mise
const TOOLS: &str = "tools";

/// The extension of a configuration file that the repair edits
///
/// Mise also reads `.tool-versions`, a format that is not TOML. The repair
/// leaves such a file as it is. Mise ranks that file below the generated
/// file, so its pin overrides nothing once the generated file exists.
const TOML: &str = "toml";

/// One configuration file that mise lists, with the tools that it pins
#[derive(Debug, Deserialize)]
struct ConfigFile {
    /// The location of the file
    path: PathBuf,

    /// The tools that the file pins
    #[serde(default)]
    tools: Vec<String>,
}

/// A pin of the project that overrides a tool that a mounted action declares
///
/// Mise ranks some configuration files above the generated file, such as
/// `mise.toml`. A pin in such a file wins over the same tool in the generated
/// file. An editor and a run of the tool by hand then get the version of the
/// project. They do not get the version that the tests of the action cover.
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct Shadow {
    /// The configuration file that holds the pin, relative to the root
    file: FilePath,

    /// The tool that the file pins
    tool: MiseTool,
}

impl Shadow {
    /// Returns the finding of the pin
    pub(crate) fn finding(&self) -> Finding {
        Finding::builder()
            .message(format!(
                "the project pins `{}`, which overrides the version that a mounted action \
                 declares, and `doctor --fix` removes the pin",
                self.tool
            ))
            .location(Location::File {
                path: self.file.clone(),
            })
            .build()
    }
}

/// Returns each pin of the project that overrides a declared tool
///
/// Mise lists the configuration files that apply in the root of the project.
/// Only a file inside the root counts. The global configuration of the user
/// and the files of the directories above the project differ on each machine,
/// so the project cannot repair them. Only a file that mise ranks above the
/// generated file counts, because a pin in a file below it does not win. The
/// generated file does not count either, because its pins are the declared
/// ones.
///
/// # Errors
///
/// Returns [`MiseUnavailable`][unavailable] when mise does not run,
/// [`UnlistedConfiguration`][unlisted] when mise ends without success, and
/// [`UnrecognizedConfiguration`][unrecognized] when the list is not the JSON
/// that `doctor` reads.
///
/// [unavailable]: CheckToolPinsError::MiseUnavailable
/// [unlisted]: CheckToolPinsError::UnlistedConfiguration
/// [unrecognized]: CheckToolPinsError::UnrecognizedConfiguration
#[bon::builder]
pub(crate) async fn find(
    pins: &Pins,
    context: &Context,
) -> Result<Vec<Shadow>, CheckToolPinsError> {
    let root = context.root();

    let execution = Invocation::new(MISE)
        .args(LIST)
        .in_directory(root.get())
        .run()
        .await
        .map_err(|source| CheckToolPinsError::MiseUnavailable { source })?;

    if !execution.status().success() {
        return Err(CheckToolPinsError::UnlistedConfiguration {
            details: diagnosis(&execution),
        });
    }

    let files: Vec<ConfigFile> = serde_json::from_slice(execution.stdout().get())
        .map_err(|source| CheckToolPinsError::UnrecognizedConfiguration { source })?;

    Ok(shadows().files(&files).pins(pins).root(root).call())
}

/// Removes the pin from its configuration file and returns whether it did
///
/// The edit keeps the comments and the layout of the file. A file that is
/// not TOML stays as it is. So does a file whose table of tools does not hold
/// the pin, and the caller keeps the pin as a finding.
///
/// # Errors
///
/// Returns [`UnreadableConfiguration`][unreadable] when the file system does
/// not give the content of the file. Returns
/// [`UnparsableConfiguration`][unparsable] when the file is not valid TOML.
/// Returns [`UnwritableFile`][unwritable] when the file system refuses the
/// write.
///
/// [unparsable]: RepairToolPinsError::UnparsableConfiguration
/// [unreadable]: RepairToolPinsError::UnreadableConfiguration
/// [unwritable]: RepairToolPinsError::UnwritableFile
#[bon::builder]
pub(crate) fn remove(shadow: &Shadow, root: &ProjectRoot) -> Result<bool, RepairToolPinsError> {
    if shadow
        .file
        .get()
        .extension()
        .and_then(|extension| extension.to_str())
        != Some(TOML)
    {
        return Ok(false);
    }

    let path = root.get().join(shadow.file.get());

    let text = std::fs::read_to_string(&path).map_err(|source| {
        RepairToolPinsError::UnreadableConfiguration {
            path: path.clone(),
            source,
        }
    })?;
    let mut document: DocumentMut =
        text.parse()
            .map_err(|source| RepairToolPinsError::UnparsableConfiguration {
                path: path.clone(),
                source,
            })?;

    let removed = document
        .get_mut(TOOLS)
        .and_then(Item::as_table_like_mut)
        .and_then(|tools| tools.remove(shadow.tool.get()))
        .is_some();

    if removed {
        std::fs::write(&path, document.to_string())
            .map_err(|source| RepairToolPinsError::UnwritableFile { path, source })?;
    }

    Ok(removed)
}

/// Returns each pin of a declared tool in a configuration file inside the
/// root that mise ranks above the generated file
///
/// Mise lists the files from the highest priority down, and a pin in a file
/// that comes first wins. So a file after the generated file, such as
/// `.tool-versions`, overrides nothing. A project that holds no generated
/// file yet gives no rank to compare with, so every file of the project
/// counts.
#[bon::builder]
fn shadows(files: &[ConfigFile], pins: &Pins, root: &ProjectRoot) -> Vec<Shadow> {
    let generated = Path::new(generated::PATH);
    let mut shadows = Vec::new();

    for file in files {
        let Some(path) = FilePath::within().path(&file.path).root(root).call() else {
            continue;
        };

        if path.get() == generated {
            break;
        }

        shadows.extend(
            file.tools
                .iter()
                .filter(|tool| pins.contains(tool))
                .map(|tool| Shadow {
                    file: path.clone(),
                    tool: MiseTool::new(tool.as_str()),
                }),
        );
    }

    shadows
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{RequiredVersion, ToolRequirement, ToolVersion};
    use tempfile::TempDir;

    use super::*;

    /// The root of the project that the tests of the list read
    #[cfg(unix)]
    const ROOT: &str = "/home/otter/project";

    /// The root of the project that the tests of the list read
    #[cfg(windows)]
    const ROOT: &str = "C:\\otter\\project";

    /// Returns the pins of taplo
    fn taplo() -> Pins {
        Pins::of(&[ToolRequirement::builder()
            .tool("taplo")
            .executable("taplo")
            .version(RequiredVersion::Exact(ToolVersion::new("0.10.0")))
            .build()])
    }

    /// Returns a configuration file at a path below the root that pins the
    /// tools
    fn file_below(path: &str, tools: &[&str]) -> ConfigFile {
        ConfigFile {
            path: Path::new(ROOT).join(path),
            tools: tools.iter().map(|&tool| tool.to_owned()).collect(),
        }
    }

    /// Returns the files of the pins that the list gives for the root
    fn shadowed(files: &[ConfigFile]) -> Vec<String> {
        shadows()
            .files(files)
            .pins(&taplo())
            .root(&ProjectRoot::from(ROOT))
            .call()
            .iter()
            .map(|shadow| format!("{} {}", shadow.file, shadow.tool))
            .collect()
    }

    /// Returns the project in a temporary directory whose file holds the text
    #[bon::builder]
    fn project_with(name: &str, text: &str) -> (TempDir, Shadow) {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        std::fs::write(directory.path().join(name), text).expect("the test writes the file");
        let shadow = Shadow {
            file: FilePath::try_from(name).expect("the test names a relative file"),
            tool: MiseTool::new("taplo"),
        };

        (directory, shadow)
    }

    /// Removes the pin from the file of a project and returns the file and
    /// whether the pin went
    #[bon::builder]
    fn removed_from(name: &str, text: &str) -> (String, bool) {
        let (directory, shadow) = project_with().name(name).text(text).call();

        let removed = remove()
            .shadow(&shadow)
            .root(&ProjectRoot::from(directory.path()))
            .call()
            .expect("the test project is readable and writable");

        let text =
            std::fs::read_to_string(directory.path().join(name)).expect("the test reads the file");

        (text, removed)
    }

    #[test]
    fn config_file_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ConfigFile>();
    }

    #[test]
    fn remove_keeps_the_comments_and_the_other_pins_of_the_file() {
        let (text, _) = removed_from().name("mise.toml").text("# The tools of the project\n[tools]\njq = \"1.7.1\" # for scripts\ntaplo = \"0.9.0\"\n").call();

        assert_eq!(
            text,
            "# The tools of the project\n[tools]\njq = \"1.7.1\" # for scripts\n"
        );
    }

    #[test]
    fn remove_with_a_dotted_pin_removes_it() {
        let (text, _) = removed_from()
            .name("mise.toml")
            .text("tools.taplo = \"0.9.0\"\ntools.jq = \"1\"\n")
            .call();

        assert_eq!(text, "tools.jq = \"1\"\n");
    }

    #[test]
    fn remove_with_a_file_that_is_no_toml_leaves_the_pin() {
        let (text, removed) = removed_from()
            .name(".tool-versions")
            .text("taplo 0.9.0\n")
            .call();

        assert_eq!((text.as_str(), removed), ("taplo 0.9.0\n", false));
    }

    #[test]
    fn remove_with_a_file_that_is_no_valid_toml_reports_it() {
        let (directory, shadow) = project_with().name("mise.toml").text("[tools\n").call();

        let result = remove()
            .shadow(&shadow)
            .root(&ProjectRoot::from(directory.path()))
            .call();

        assert!(
            matches!(
                result,
                Err(RepairToolPinsError::UnparsableConfiguration { .. })
            ),
            "expected a file that is no valid TOML, got {result:?}"
        );
    }

    #[test]
    fn remove_with_a_file_without_the_pin_reports_nothing_removed() {
        let (_, removed) = removed_from()
            .name("mise.toml")
            .text("[tools]\njq = \"1.7.1\"\n")
            .call();

        assert!(!removed);
    }

    #[test]
    fn remove_with_a_missing_file_reports_it_unreadable() {
        let directory = TempDir::new().expect("the test creates a temporary directory");
        let shadow = Shadow {
            file: FilePath::try_from("mise.toml").expect("the test names a relative file"),
            tool: MiseTool::new("taplo"),
        };

        let result = remove()
            .shadow(&shadow)
            .root(&ProjectRoot::from(directory.path()))
            .call();

        assert!(
            matches!(
                result,
                Err(RepairToolPinsError::UnreadableConfiguration { .. })
            ),
            "expected an unreadable file, got {result:?}"
        );
    }

    #[test]
    fn remove_with_a_pin_reports_it_removed() {
        let (_, removed) = removed_from()
            .name("mise.toml")
            .text("[tools]\ntaplo = \"0.9.0\"\n")
            .call();

        assert!(removed);
    }

    #[test]
    fn shadow_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Shadow>();
    }

    #[test]
    fn shadows_leaves_out_a_file_outside_the_root() {
        let outside = ConfigFile {
            path: Path::new(ROOT)
                .parent()
                .expect("the root has a parent")
                .join("mise.toml"),
            tools: vec!["taplo".to_owned()],
        };

        let pins = shadowed(&[outside]);

        assert!(pins.is_empty(), "got {pins:?}");
    }

    #[test]
    fn shadows_leaves_out_a_file_that_mise_ranks_below_the_generated_file() {
        let pins = shadowed(&[
            file_below("mise.toml", &["taplo"]),
            file_below(generated::PATH, &["taplo"]),
            file_below(".tool-versions", &["taplo"]),
        ]);

        assert_eq!(pins, ["mise.toml taplo"]);
    }

    #[test]
    fn shadows_leaves_out_a_tool_that_no_action_declares() {
        let pins = shadowed(&[file_below("mise.toml", &["jq"])]);

        assert!(pins.is_empty(), "got {pins:?}");
    }

    #[test]
    fn shadows_leaves_out_the_generated_file() {
        let pins = shadowed(&[file_below(generated::PATH, &["taplo"])]);

        assert!(pins.is_empty(), "got {pins:?}");
    }

    #[test]
    fn shadows_reports_each_file_that_pins_a_declared_tool() {
        let pins = shadowed(&[
            file_below("mise.toml", &["jq", "taplo"]),
            file_below("mise.local.toml", &["taplo"]),
        ]);

        assert_eq!(pins, ["mise.toml taplo", "mise.local.toml taplo"]);
    }
}
