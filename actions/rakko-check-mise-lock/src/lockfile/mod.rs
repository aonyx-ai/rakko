//! The entries of one lockfile of mise
//!
//! The action reads a lockfile before and after `mise lock`, and compares the
//! two readings part by part. This module holds the reading and the
//! comparison. It reads text and starts nothing, so the comparison does not
//! depend on the order of the keys, on the comments, or on the spaces of the
//! file.

/// The error that keeps the action from reading a lockfile
mod error;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

pub use self::error::ReadLockfileError;
use crate::difference::{Change, Difference};
use crate::part::{Part, Platform, ToolName, Version};

/// The key of the table that holds the tools of a lockfile
const TOOLS: &str = "tools";

/// The key of the field of an entry that names its version
const VERSION: &str = "version";

/// The key of the field of a platform that names its download
const URL: &str = "url";

/// The key of the content of a part that holds the table of its platform
const PLATFORM: &str = "platform";

/// The prefix of the key of a table that records one platform of an entry
///
/// Mise writes the key in quotes, such as `"platforms.linux-x64"`, so the
/// key holds the prefix and the platform as one name.
const PLATFORM_PREFIX: &str = "platforms.";

/// The entries of one lockfile of mise, part by part
///
/// Each part holds the table of its platform together with the fields that
/// its entry shares, so a change of a shared field, such as the backend,
/// changes every part of the entry. A tool can hold more than one entry for
/// the same version, and the parts of such entries keep the order of the
/// lockfile.
///
/// A lockfile that does not exist reads as a lockfile without parts, which is
/// the [`Default`] value.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Lockfile {
    /// The content of each part, in the order of the lockfile
    parts: BTreeMap<Part, Vec<Value>>,
}

impl Lockfile {
    /// Returns the differences between this lockfile and the same lockfile
    /// after a run of `mise lock`
    ///
    /// The comparison takes every part that either side holds. A part that
    /// only the other side holds was added, a part that only this side holds
    /// was removed, and a part that both sides hold with other content was
    /// changed. A part that a tool records in more than one entry for the
    /// same version was added when the other side holds more of those
    /// entries, and removed when it holds fewer. The differences come in the
    /// order of their parts.
    ///
    /// An entry that gains or loses its platforms also loses or gains its
    /// part without a platform. The parts of the platforms name that change,
    /// so the part without a platform gives no difference of its own then.
    pub fn differences(&self, after: &Self) -> Vec<Difference> {
        let parts: BTreeSet<&Part> = self.parts.keys().chain(after.parts.keys()).collect();

        let differences: Vec<Difference> = parts
            .into_iter()
            .filter_map(|part| {
                let before = self.parts.get(part);
                let later = after.parts.get(part);

                let change = match (before, later) {
                    (Some(_), None) => Change::Removed,
                    (None, Some(_)) => Change::Added,
                    (Some(before), Some(later)) if later.len() > before.len() => Change::Added,
                    (Some(before), Some(later)) if later.len() < before.len() => Change::Removed,
                    (Some(before), Some(later)) if before != later => Change::Changed,
                    _ => return None,
                };

                Some(Difference::new(part.clone(), change))
            })
            .collect();

        let platformed: BTreeSet<(&ToolName, &Version)> = differences
            .iter()
            .map(Difference::part)
            .filter(|part| part.platform().is_some())
            .map(|part| (part.tool(), part.version()))
            .collect();

        differences
            .iter()
            .filter(|difference| {
                let part = difference.part();

                part.platform().is_some() || !platformed.contains(&(part.tool(), part.version()))
            })
            .cloned()
            .collect()
    }

    /// Returns the parts that record a platform without the address of its
    /// download
    ///
    /// An install in locked mode refuses a platform whose entry names no
    /// download. `mise lock` writes such an entry when it could not resolve
    /// the tool, such as one that holds only the provenance of the download.
    pub fn unaddressed(&self) -> Vec<&Part> {
        self.parts
            .iter()
            .filter(|(part, contents)| {
                part.platform().is_some()
                    && contents.iter().any(|content| {
                        content
                            .get(PLATFORM)
                            .is_some_and(|table| table.get(URL).is_none())
                    })
            })
            .map(|(part, _)| part)
            .collect()
    }

    /// Returns whether the lockfile records any platform
    pub fn records_platforms(&self) -> bool {
        self.parts.keys().any(|part| part.platform().is_some())
    }

    /// Reads the entries of a lockfile from its text
    ///
    /// # Errors
    ///
    /// Returns [`UnparsableToml`][unparsable] when the text is not TOML, and
    /// the other variants of [`ReadLockfileError`] when the tools, an entry,
    /// or a platform of an entry have a shape that the action does not know.
    ///
    /// [unparsable]: ReadLockfileError::UnparsableToml
    pub fn parse(text: &str) -> Result<Self, ReadLockfileError> {
        let document: Map<String, Value> = toml_edit::de::from_str(text)
            .map_err(|source| ReadLockfileError::UnparsableToml { source })?;

        let Some(tools) = document.get(TOOLS) else {
            return Ok(Self::default());
        };

        let tools = tools.as_object().ok_or(ReadLockfileError::MisshapenTools)?;

        let mut parts: BTreeMap<Part, Vec<Value>> = BTreeMap::new();

        for (name, entries) in tools {
            let tool = ToolName::new(name.as_str());
            let entries = entries
                .as_array()
                .ok_or_else(|| ReadLockfileError::MisshapenTool { tool: tool.clone() })?;

            for entry in entries {
                let entry = entry
                    .as_object()
                    .ok_or_else(|| ReadLockfileError::MisshapenTool { tool: tool.clone() })?;

                for (part, content) in Self::split(&tool, entry)? {
                    parts.entry(part).or_default().push(content);
                }
            }
        }

        Ok(Self { parts })
    }

    /// Returns the parts of one entry, each with its content
    ///
    /// The content of a part holds the fields that the entry shares and the
    /// table of the platform. An entry without a platform gives one part
    /// without a platform, whose content is the shared fields.
    ///
    /// # Errors
    ///
    /// Returns [`MissingVersion`][version] when the entry names no version as
    /// text, and [`MisshapenPlatform`][platform] when it records a platform
    /// in a shape that is not a table.
    ///
    /// [platform]: ReadLockfileError::MisshapenPlatform
    /// [version]: ReadLockfileError::MissingVersion
    fn split(
        tool: &ToolName,
        entry: &Map<String, Value>,
    ) -> Result<Vec<(Part, Value)>, ReadLockfileError> {
        let version = entry
            .get(VERSION)
            .and_then(Value::as_str)
            .map(Version::new)
            .ok_or_else(|| ReadLockfileError::MissingVersion { tool: tool.clone() })?;

        let mut shared = Map::new();
        let mut platforms = Vec::new();

        for (key, value) in entry {
            match key.strip_prefix(PLATFORM_PREFIX) {
                Some(platform) => platforms.push((Platform::new(platform), value)),
                None => {
                    shared.insert(key.clone(), value.clone());
                }
            }
        }

        let part = |platform: Option<Platform>| {
            Part::builder()
                .tool(tool.clone())
                .version(version.clone())
                .maybe_platform(platform)
                .build()
        };

        if platforms.is_empty() {
            return Ok(vec![(part(None), Value::Object(shared))]);
        }

        platforms
            .into_iter()
            .map(|(platform, table)| {
                if !table.is_object() {
                    return Err(ReadLockfileError::MisshapenPlatform {
                        tool: tool.clone(),
                        version: version.clone(),
                        platform,
                    });
                }

                let mut content = Map::new();
                content.insert("entry".to_owned(), Value::Object(shared.clone()));
                content.insert(PLATFORM.to_owned(), table.clone());

                Ok((part(Some(platform)), Value::Object(content)))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A lockfile with one tool on two platforms and one tool without one
    const COMPLETE: &str = r#"
lockfile_version = 1

[[tools.jq]]
version = "1.8.1"
backend = "aqua:jqlang/jq"

[tools.jq."platforms.linux-x64"]
url = "https://example.com/jq-linux"

[tools.jq."platforms.macos-arm64"]
url = "https://example.com/jq-macos"

[[tools."cargo:cargo-deny"]]
version = "0.20.2"
backend = "cargo:cargo-deny"
"#;

    /// Returns the differences between two lockfiles, as finding messages
    fn compare(before: &str, after: &str) -> Vec<String> {
        let before = Lockfile::parse(before).unwrap();
        let after = Lockfile::parse(after).unwrap();

        before
            .differences(&after)
            .iter()
            .map(Difference::finding)
            .collect()
    }

    #[test]
    fn differences_with_a_changed_entry_without_a_platform_name_the_entry() {
        let after = COMPLETE.replace(
            "backend = \"cargo:cargo-deny\"",
            "backend = \"cargo:other\"",
        );

        let differences = compare(COMPLETE, &after);

        assert_eq!(
            differences,
            ["mise lock changes the entry of cargo:cargo-deny 0.20.2"]
        );
    }

    #[test]
    fn differences_with_a_changed_platform_name_the_platform() {
        let after = COMPLETE.replace("jq-linux", "jq-linux-amd64");

        let differences = compare(COMPLETE, &after);

        assert_eq!(
            differences,
            ["mise lock changes the entry of jq 1.8.1 for linux-x64"]
        );
    }

    #[test]
    fn differences_with_a_changed_shared_field_name_every_platform() {
        let after = COMPLETE.replace("aqua:jqlang/jq", "github:jqlang/jq");

        let differences = compare(COMPLETE, &after);

        assert_eq!(
            differences,
            [
                "mise lock changes the entry of jq 1.8.1 for linux-x64",
                "mise lock changes the entry of jq 1.8.1 for macos-arm64",
            ]
        );
    }

    #[test]
    fn differences_with_a_removed_entry_name_its_parts() {
        let after = COMPLETE.replace(
            "[[tools.\"cargo:cargo-deny\"]]\nversion = \"0.20.2\"\nbackend = \"cargo:cargo-deny\"\n",
            "",
        );

        let differences = compare(COMPLETE, &after);

        assert_eq!(
            differences,
            ["mise lock removes the entry of cargo:cargo-deny 0.20.2"]
        );
    }

    #[test]
    fn differences_with_a_second_entry_of_the_same_version_name_its_parts() {
        let after = format!(
            "{COMPLETE}\n[[tools.jq]]\nversion = \"1.8.1\"\nbackend = \"aqua:jqlang/jq\"\n\n[tools.jq.\"platforms.linux-x64\"]\nurl = \"https://example.com/jq-linux\"\n"
        );

        let differences = compare(COMPLETE, &after);

        assert_eq!(
            differences,
            ["mise lock adds the entry of jq 1.8.1 for linux-x64"]
        );
    }

    #[test]
    fn differences_with_an_added_platform_name_the_platform() {
        let before = COMPLETE.replace(
            "[tools.jq.\"platforms.macos-arm64\"]\nurl = \"https://example.com/jq-macos\"\n",
            "",
        );

        let differences = compare(&before, COMPLETE);

        assert_eq!(
            differences,
            ["mise lock adds the entry of jq 1.8.1 for macos-arm64"]
        );
    }

    #[test]
    fn differences_with_an_entry_that_gains_its_platforms_name_only_the_platforms() {
        let before = "[[tools.jq]]\nversion = \"1.8.1\"\nbackend = \"aqua:jqlang/jq\"\n";
        let after = format!(
            "{before}\n[tools.jq.\"platforms.linux-x64\"]\nurl = \"https://example.com/jq-linux\"\n"
        );

        let differences = compare(before, &after);

        assert_eq!(
            differences,
            ["mise lock adds the entry of jq 1.8.1 for linux-x64"]
        );
    }

    #[test]
    fn differences_with_no_lockfile_before_name_every_part() {
        let differences = Lockfile::default()
            .differences(&Lockfile::parse(COMPLETE).unwrap())
            .len();

        assert_eq!(differences, 3);
    }

    #[test]
    fn differences_with_other_formatting_are_empty() {
        let after = COMPLETE
            .replace(
                "version = \"1.8.1\"\nbackend = \"aqua:jqlang/jq\"",
                "backend = \"aqua:jqlang/jq\"\nversion = \"1.8.1\"",
            )
            .replace("lockfile_version = 1", "# a comment\nlockfile_version = 1");

        let differences = compare(COMPLETE, &after);

        assert!(
            differences.is_empty(),
            "expected no difference, got {differences:?}"
        );
    }

    #[test]
    fn differences_with_the_same_lockfile_are_empty() {
        let differences = compare(COMPLETE, COMPLETE);

        assert!(
            differences.is_empty(),
            "expected no difference, got {differences:?}"
        );
    }

    #[test]
    fn differences_without_the_second_entry_of_the_same_version_name_its_parts() {
        let before = format!(
            "{COMPLETE}\n[[tools.jq]]\nversion = \"1.8.1\"\nbackend = \"aqua:jqlang/jq\"\n\n[tools.jq.\"platforms.linux-x64\"]\nurl = \"https://example.com/jq-linux\"\n"
        );

        let differences = compare(&before, COMPLETE);

        assert_eq!(
            differences,
            ["mise lock removes the entry of jq 1.8.1 for linux-x64"]
        );
    }

    #[test]
    fn parse_with_a_platform_that_is_not_a_table_reports_the_platform() {
        let text = "[[tools.jq]]\nversion = \"1.8.1\"\n\"platforms.linux-x64\" = \"here\"\n";

        let error = Lockfile::parse(text).unwrap_err();

        assert!(
            matches!(&error, ReadLockfileError::MisshapenPlatform { platform, .. } if platform.get() == "linux-x64"),
            "expected the platform to be named, got {error:?}"
        );
    }

    #[test]
    fn parse_with_a_tool_that_is_a_single_table_reports_the_tool() {
        let error = Lockfile::parse("[tools.jq]\nversion = \"1.8.1\"\n").unwrap_err();

        assert!(
            matches!(&error, ReadLockfileError::MisshapenTool { tool } if tool.get() == "jq"),
            "expected the tool to be named, got {error:?}"
        );
    }

    #[test]
    fn parse_with_an_entry_that_is_not_a_table_reports_the_tool() {
        let error = Lockfile::parse("[tools]\njq = [\"1.8.1\"]\n").unwrap_err();

        assert!(
            matches!(&error, ReadLockfileError::MisshapenTool { tool } if tool.get() == "jq"),
            "expected the tool to be named, got {error:?}"
        );
    }

    #[test]
    fn parse_with_an_entry_without_a_version_reports_the_tool() {
        let error = Lockfile::parse("[[tools.jq]]\nbackend = \"aqua:jqlang/jq\"\n").unwrap_err();

        assert!(
            matches!(&error, ReadLockfileError::MissingVersion { tool } if tool.get() == "jq"),
            "expected the tool to be named, got {error:?}"
        );
    }

    #[test]
    fn parse_with_text_that_is_not_toml_reports_it() {
        let error = Lockfile::parse("this is not toml [[[\n").unwrap_err();

        assert!(
            matches!(error, ReadLockfileError::UnparsableToml { .. }),
            "expected a parse error, got {error:?}"
        );
    }

    #[test]
    fn parse_with_tools_that_are_not_a_table_reports_them() {
        let error = Lockfile::parse("tools = 5\n").unwrap_err();

        assert!(
            matches!(error, ReadLockfileError::MisshapenTools),
            "expected the tools to be named, got {error:?}"
        );
    }

    #[test]
    fn parse_without_tools_reads_no_part() {
        let lockfile = Lockfile::parse("lockfile_version = 1\n").unwrap();

        assert_eq!(lockfile, Lockfile::default());
    }

    #[test]
    fn records_platforms_with_a_platform_says_so() {
        let lockfile = Lockfile::parse(COMPLETE).unwrap();

        assert!(lockfile.records_platforms());
    }

    #[test]
    fn records_platforms_without_a_platform_says_none() {
        let lockfile = Lockfile::parse("[[tools.rust]]\nversion = \"1.99.1\"\n").unwrap();

        assert!(!lockfile.records_platforms());
    }

    #[test]
    fn unaddressed_with_a_platform_that_holds_only_its_provenance_names_it() {
        let text = COMPLETE.replace(
            "url = \"https://example.com/jq-macos\"",
            "provenance = \"github-attestations\"",
        );
        let lockfile = Lockfile::parse(&text).unwrap();

        let parts: Vec<String> = lockfile
            .unaddressed()
            .iter()
            .map(ToString::to_string)
            .collect();

        assert_eq!(parts, ["jq 1.8.1 for macos-arm64"]);
    }

    #[test]
    fn unaddressed_with_complete_platforms_names_nothing() {
        let lockfile = Lockfile::parse(COMPLETE).unwrap();

        assert!(lockfile.unaddressed().is_empty());
    }
}
