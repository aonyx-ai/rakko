//! The toolchains that the mounted actions require of the project

use std::collections::BTreeSet;

use rakko::{Context, Finding, Location, Name, RequiredVersion, VersionRange};
use rakko_tool::{ResolveRequirementError, Tool};

use super::Declaration;
use super::check_error::CheckToolPinsError;

/// Returns a finding for each toolchain that a mounted action requires and
/// that the project does not pin in its range
///
/// A toolchain is a decision of the project, so `doctor --fix` leaves it. The
/// finding says which pin to change. A toolchain that the project pins in its
/// range passes, also when mise did not install that version yet. An install
/// of mise repairs that, and the condition is about the pin.
///
/// # Errors
///
/// Returns [`CheckToolPinsError::UnresolvedToolchain`] when the resolution of
/// a toolchain fails for a reason that no pin of the project repairs. Examples
/// are mise that does not run, and a range that the action wrote wrong.
#[bon::builder]
pub(crate) async fn check(
    declarations: &BTreeSet<Declaration>,
    context: &Context,
) -> Result<Vec<Finding>, CheckToolPinsError> {
    let mut findings = Vec::new();

    for declaration in declarations {
        let RequiredVersion::Range(range) = declaration.requirement().version() else {
            continue;
        };

        let result = Tool::resolve_requirement()
            .requirement(declaration.requirement())
            .context(context)
            .call()
            .await;

        findings.extend(
            finding()
                .action(declaration.action())
                .range(range)
                .result(result)
                .call()?,
        );
    }

    Ok(findings)
}

/// Returns the finding of the resolution of a toolchain, or `None` when the
/// project pins the toolchain in its range
///
/// # Errors
///
/// Returns [`CheckToolPinsError::UnresolvedToolchain`] for an error of the
/// resolution that no pin of the project repairs.
#[bon::builder]
fn finding(
    action: &Name,
    range: &VersionRange,
    result: Result<Tool, ResolveRequirementError>,
) -> Result<Option<Finding>, CheckToolPinsError> {
    let message = match result {
        Ok(_) | Err(ResolveRequirementError::UnresolvedVersion { .. }) => return Ok(None),
        Err(ResolveRequirementError::VersionOutOfRange { tool, version, .. }) => format!(
            "the project pins `{tool}` {version}, outside the range `{range}` that the action \
             `{action}` accepts, so change the pin of `{tool}` to a version in the range"
        ),
        Err(ResolveRequirementError::UnresolvedToolchain { tool, details }) => format!(
            "the project pins no version of `{tool}` that the action `{action}` accepts, so pin \
             a version in the range `{range}`: {details}"
        ),
        Err(ResolveRequirementError::UnreadableVersion { tool, version, .. }) => format!(
            "the project pins `{tool}` {version}, which has no version number to compare with \
             the range `{range}` that the action `{action}` accepts, so pin a version with a \
             number"
        ),
        Err(source) => {
            return Err(CheckToolPinsError::UnresolvedToolchain {
                action: action.clone(),
                source,
            });
        }
    };

    Ok(Some(
        Finding::builder()
            .message(message)
            .location(Location::Project)
            .build(),
    ))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{Executable, MiseTool, ToolVersion};

    use super::*;

    /// Returns the message that the resolution gives, or `None` when it gives
    /// no finding
    fn message_of(error: ResolveRequirementError) -> Option<String> {
        let action: Name = "check-msrv".parse().expect("the test names an action");

        finding()
            .action(&action)
            .range(&VersionRange::new(">=1.85"))
            .result(Err(error))
            .call()
            .expect("the resolution gives a finding or nothing")
            .map(|finding| finding.message().get().to_owned())
    }

    #[test]
    fn finding_with_a_pin_outside_the_range_gives_the_action_and_the_pin() {
        let message = message_of(ResolveRequirementError::VersionOutOfRange {
            tool: MiseTool::new("rust"),
            version: ToolVersion::new("1.80.0"),
            range: VersionRange::new(">=1.85"),
        });

        assert_eq!(
            message.as_deref(),
            Some(
                "the project pins `rust` 1.80.0, outside the range `>=1.85` that the action \
                 `check-msrv` accepts, so change the pin of `rust` to a version in the range"
            )
        );
    }

    #[test]
    fn finding_with_a_pin_that_is_not_installed_gives_nothing() {
        let message = message_of(ResolveRequirementError::UnresolvedVersion {
            tool: MiseTool::new("rust"),
            executable: Executable::new("cargo"),
            version: ToolVersion::new("1.99.1"),
            details: "rust@1.99.1 is not installed".to_owned(),
        });

        assert_eq!(message, None);
    }

    #[test]
    fn finding_with_a_pin_without_a_number_gives_the_pin() {
        let source = semver::Version::parse("stable").expect_err("the text is no version");

        let message = message_of(ResolveRequirementError::UnreadableVersion {
            tool: MiseTool::new("rust"),
            version: ToolVersion::new("stable"),
            source,
        });

        assert!(
            message
                .as_deref()
                .is_some_and(|message| message.contains("`rust` stable")),
            "got {message:?}"
        );
    }

    #[test]
    fn finding_with_an_error_that_no_pin_repairs_reports_it() {
        let action: Name = "check-msrv".parse().expect("the test names an action");
        let error = ResolveRequirementError::UnknownRequirement {
            tool: MiseTool::new("rust"),
        };

        let result = finding()
            .action(&action)
            .range(&VersionRange::new(">=1.85"))
            .result(Err(error))
            .call();

        assert!(
            matches!(result, Err(CheckToolPinsError::UnresolvedToolchain { .. })),
            "expected an unresolved toolchain, got {result:?}"
        );
    }

    #[test]
    fn finding_with_no_pin_gives_the_range() {
        let message = message_of(ResolveRequirementError::UnresolvedToolchain {
            tool: MiseTool::new("rust"),
            details: "no configuration inside the project pins a version".to_owned(),
        });

        assert!(
            message
                .as_deref()
                .is_some_and(|message| message.contains("in the range `>=1.85`")),
            "got {message:?}"
        );
    }
}
