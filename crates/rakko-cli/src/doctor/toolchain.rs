//! The toolchains that the mounted actions require of the project

use std::collections::{BTreeMap, BTreeSet};

use rakko::{
    Context, Finding, Location, MiseTool, Name, RequiredVersion, ToolVersion, ToolchainRequirement,
    VersionRange,
};
use rakko_tool::{ResolveRequirementError, ResolveToolchainError, Tool};

use super::check_error::CheckToolPinsError;
use super::{Declaration, ToolchainDeclaration};

/// Returns a finding for each toolchain that a mounted action requires and
/// that the project does not pin in a version that the action accepts
///
/// An action requires a toolchain in two forms. A requirement of a toolchain
/// gives the versions that the action accepts. The check takes the
/// requirements of one action for one toolchain together, as a run of the
/// action does. A tool requirement with a range gives a range of its own. An
/// older action declares such a requirement among its tools.
///
/// A toolchain is a decision of the project, so `doctor --fix` leaves it. The
/// finding says which pin to change. A toolchain that the project pins in a
/// version that the action accepts passes, also when mise did not install that
/// version yet. An install of mise repairs that, and the condition is about the
/// pin.
///
/// # Errors
///
/// Returns [`CheckToolPinsError::UnresolvedToolchain`] or
/// [`CheckToolPinsError::UncheckedRequirement`] when the check of a toolchain
/// fails for a reason that no pin of the project repairs. Examples are mise
/// that does not run, and a range that the action wrote wrong.
#[bon::builder]
pub(crate) async fn check(
    declarations: &BTreeSet<Declaration>,
    toolchains: &BTreeSet<ToolchainDeclaration>,
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

    for ((action, toolchain), requirements) in required(toolchains) {
        let result = Tool::select_toolchain()
            .toolchain(&toolchain)
            .requirements(&requirements)
            .context(context)
            .call()
            .await;

        findings.extend(
            requirement_finding()
                .action(&action)
                .requirements(&requirements)
                .result(result)
                .call()?,
        );
    }

    Ok(findings)
}

/// Returns the requirements of each action for each toolchain
fn required(
    toolchains: &BTreeSet<ToolchainDeclaration>,
) -> BTreeMap<(Name, MiseTool), Vec<ToolchainRequirement>> {
    let mut required: BTreeMap<(Name, MiseTool), Vec<ToolchainRequirement>> = BTreeMap::new();

    for declaration in toolchains {
        required
            .entry((
                declaration.action().clone(),
                declaration.requirement().toolchain().clone(),
            ))
            .or_default()
            .push(declaration.requirement().clone());
    }

    required
}

/// Returns the finding of the requirements of one action for one toolchain
///
/// The answer is `None` when the project pins a version that they accept.
///
/// # Errors
///
/// Returns [`CheckToolPinsError::UncheckedRequirement`] for an error of the
/// check that no pin of the project repairs.
#[bon::builder]
fn requirement_finding(
    action: &Name,
    requirements: &[ToolchainRequirement],
    result: Result<ToolVersion, ResolveToolchainError>,
) -> Result<Option<Finding>, CheckToolPinsError> {
    let message = match result {
        Ok(_) => return Ok(None),
        Err(ResolveToolchainError::UnmetRequirement {
            toolchain,
            version,
            range,
        }) => format!(
            "the project pins `{toolchain}` {version}, which the requirement `{range}` of the \
             action `{action}` does not accept, so change the pin of `{toolchain}` to a version \
             that it accepts"
        ),
        Err(ResolveToolchainError::UnpinnedToolchain { toolchain, details }) => format!(
            "the project pins no version of `{toolchain}` that the action `{action}` accepts, so \
             pin a version that {}: {details}",
            accepted(requirements)
        ),
        Err(ResolveToolchainError::UnreadableVersion {
            toolchain, version, ..
        }) => format!(
            "the project pins `{toolchain}` {version}, which has no version number to compare \
             with {} of the action `{action}`, so pin a version with a number",
            ranges(requirements)
        ),
        Err(source) => {
            return Err(CheckToolPinsError::UncheckedRequirement {
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

/// Returns the words that say which versions the requirements accept
fn accepted(requirements: &[ToolchainRequirement]) -> String {
    match requirements {
        [requirement] => format!("the requirement `{}` accepts", requirement.versions()),
        _ => format!("the requirements {} all accept", quoted(requirements)),
    }
}

/// Returns the words that give the ranges of the requirements
fn ranges(requirements: &[ToolchainRequirement]) -> String {
    match requirements {
        [requirement] => format!("the requirement `{}`", requirement.versions()),
        _ => format!("the requirements {}", quoted(requirements)),
    }
}

/// Returns the range of each requirement in backticks, joined by commas
fn quoted(requirements: &[ToolchainRequirement]) -> String {
    requirements
        .iter()
        .map(|requirement| format!("`{}`", requirement.versions()))
        .collect::<Vec<_>>()
        .join(", ")
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

    use rakko::Executable;

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

    /// Returns the message that the check of the requirements gives, or
    /// `None` when it gives no finding
    fn requirement_message(ranges: &[&str], error: ResolveToolchainError) -> Option<String> {
        let action: Name = "check-msrv".parse().expect("the test names an action");
        let requirements: Vec<ToolchainRequirement> = ranges
            .iter()
            .map(|range| {
                ToolchainRequirement::builder()
                    .toolchain("rust")
                    .versions(VersionRange::new(*range))
                    .build()
            })
            .collect();

        requirement_finding()
            .action(&action)
            .requirements(&requirements)
            .result(Err(error))
            .call()
            .expect("the check gives a finding or nothing")
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

    #[test]
    fn required_holds_the_requirements_of_one_action_for_one_toolchain_together() {
        let declaration = |action: &str, toolchain: &str, range: &str| {
            ToolchainDeclaration::builder()
                .action(action.parse().expect("the test names an action"))
                .requirement(
                    ToolchainRequirement::builder()
                        .toolchain(toolchain)
                        .versions(VersionRange::new(range))
                        .build(),
                )
                .build()
        };
        let toolchains = BTreeSet::from([
            declaration("check-msrv", "rust", ">=1.85"),
            declaration("check-msrv", "rust", "<2"),
            declaration("check-msrv", "node", ">=22"),
            declaration("test-typescript", "node", ">=22.18"),
        ]);

        let sizes: Vec<usize> = required(&toolchains).values().map(Vec::len).collect();

        assert_eq!(sizes, [1, 2, 1]);
    }

    #[test]
    fn requirement_finding_with_a_pin_that_a_requirement_refuses_gives_the_pin_and_the_requirement()
    {
        let message = requirement_message(
            &[">=1.85"],
            ResolveToolchainError::UnmetRequirement {
                toolchain: MiseTool::new("rust"),
                version: ToolVersion::new("1.80.0"),
                range: VersionRange::new(">=1.85"),
            },
        );

        assert_eq!(
            message.as_deref(),
            Some(
                "the project pins `rust` 1.80.0, which the requirement `>=1.85` of the action \
                 `check-msrv` does not accept, so change the pin of `rust` to a version that it \
                 accepts"
            )
        );
    }

    #[test]
    fn requirement_finding_with_a_pin_without_a_number_gives_the_pin() {
        let source = semver::Version::parse("stable").expect_err("the text is no version");

        let message = requirement_message(
            &[">=1.85"],
            ResolveToolchainError::UnreadableVersion {
                toolchain: MiseTool::new("rust"),
                version: ToolVersion::new("stable"),
                source,
            },
        );

        assert!(
            message
                .as_deref()
                .is_some_and(|message| message.contains("`rust` stable, which has no version")),
            "got {message:?}"
        );
    }

    #[test]
    fn requirement_finding_with_an_error_that_no_pin_repairs_reports_it() {
        let action: Name = "check-msrv".parse().expect("the test names an action");
        let error = ResolveToolchainError::UnrecognizedReport {
            toolchain: MiseTool::new("rust"),
            source: serde_json::from_str::<u8>("x").expect_err("the text is no JSON"),
        };

        let result = requirement_finding()
            .action(&action)
            .requirements(&[])
            .result(Err(error))
            .call();

        assert!(
            matches!(result, Err(CheckToolPinsError::UncheckedRequirement { .. })),
            "expected an unchecked requirement, got {result:?}"
        );
    }

    #[test]
    fn requirement_finding_with_no_pin_of_two_requirements_gives_both() {
        let message = requirement_message(
            &[">=1.85", "<2"],
            ResolveToolchainError::UnpinnedToolchain {
                toolchain: MiseTool::new("rust"),
                details: "no configuration inside the project pins a version".to_owned(),
            },
        );

        assert!(
            message.as_deref().is_some_and(
                |message| message.contains("the requirements `>=1.85`, `<2` all accept")
            ),
            "got {message:?}"
        );
    }

    #[test]
    fn requirement_finding_with_the_pin_in_the_range_gives_nothing() {
        let action: Name = "check-msrv".parse().expect("the test names an action");

        let result = requirement_finding()
            .action(&action)
            .requirements(&[])
            .result(Ok(ToolVersion::new("1.99.1")))
            .call();

        assert!(matches!(result, Ok(None)), "got {result:?}");
    }
}
