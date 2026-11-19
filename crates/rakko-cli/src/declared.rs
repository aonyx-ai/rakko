//! The tools that the tests of the crate declare
//!
//! Only the reader of a mise file creates the declaration of a tool, so that
//! no action declares a tool that no mise file pins. A test of the crate
//! needs declarations with tools, versions, and platforms that only the test
//! knows. So this module writes the text of a mise file with one pin and
//! gives it to the reader at run time. The text and the pin then stay until
//! the run of the tests ends. The text of a file that a crate embeds also
//! stays until its program ends.

use rakko::__private::{MiseText, ToolPin};
use rakko::{Platform, ToolRequirement, Tools};

/// Returns the tools of a mise file that pins one tool at an exact version
///
/// Without platforms, the pin needs the tool on every platform. Otherwise,
/// the pin lists the platforms in the form of the `os` option of mise.
#[bon::builder]
pub(crate) fn tools(tool: &str, version: &str, platforms: Option<&[Platform]>) -> Tools {
    let value = match platforms {
        Some(platforms) => {
            let listed: Vec<String> = platforms
                .iter()
                .map(|platform| format!("\"{platform}\""))
                .collect();

            format!(
                "{{ version = \"{version}\", os = [{}] }}",
                listed.join(", ")
            )
        }
        None => format!("\"{version}\""),
    };
    let text: &'static str = format!("[tools]\n\"{tool}\" = {value}\n").leak();
    let pins: &'static [ToolPin; 1] = Box::leak(Box::new(MiseText::new(text).pins::<1>()));

    rakko::__private::tools(pins)
}

/// Returns the requirement of one tool at an exact version
///
/// The requirement comes from the tools that [`tools`] returns.
#[bon::builder]
pub(crate) fn requirement(
    tool: &str,
    version: &str,
    platforms: Option<&[Platform]>,
) -> ToolRequirement {
    let mut requirements = tools()
        .tool(tool)
        .version(version)
        .maybe_platforms(platforms)
        .call()
        .requirements();

    requirements.remove(0)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::collections::BTreeSet;

    use rakko::OperatingSystem;

    use super::*;

    #[test]
    fn requirement_with_platforms_holds_them() {
        let requirement = requirement()
            .tool("zizmor")
            .version("1.32.0")
            .platforms(&[Platform::new(OperatingSystem::Linux)])
            .call();

        assert_eq!(
            requirement.platforms(),
            Some(&BTreeSet::from([Platform::new(OperatingSystem::Linux)]))
        );
    }

    #[test]
    fn requirement_without_platforms_needs_every_platform() {
        let requirement = requirement().tool("taplo").version("0.10.0").call();

        assert_eq!(requirement.platforms(), None);
    }

    #[test]
    fn tools_hold_the_tool_and_the_version() {
        let tools = tools().tool("npm:prettier").version("3.10.1").call();

        let requirement = &tools.requirements()[0];

        assert_eq!(
            (requirement.tool().get(), requirement.version().get()),
            ("npm:prettier", "3.10.1")
        );
    }
}
