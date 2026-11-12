//! The example action of the guide "Run a Tool From an Action"
//!
//! The action runs taplo, an external tool, to check the TOML files of a
//! project. The crate pins taplo in its `mise.rakko.toml`, declares the tool
//! with the version of that pin, and runs it at that version through mise.
//!
//! The site shows the regions of this file between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests the file, so the code on
//! the site always compiles against the current `rakko` and `rakko-tool`
//! crates.

// region: imports
use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, RequiredVersion,
    ToolRequirement, ToolVersion, action_name,
};
use rakko_tool::{Launch, MiseFile, PinnedPlatforms, Tool};
// endregion: imports

// region: pin
/// The name of taplo in the configuration of mise, and of its program
const TAPLO: &str = "taplo";

/// The mise file of the crate, which pins the taplo that the action runs
const PINS: MiseFile = MiseFile::new(include_str!("../mise.rakko.toml"));

/// The version of taplo that the action runs
///
/// A mise file that pins no taplo fails the build.
const VERSION: &str = PINS.pinned_version(TAPLO);

/// The platforms where taplo has a build, or `None` for every platform
///
/// A platform that Rakko does not read fails the build.
const PLATFORMS: Option<PinnedPlatforms> = PINS.pinned_platforms(TAPLO);
// endregion: pin

// region: requirement
/// Returns the requirement of the taplo that the action runs
fn taplo() -> ToolRequirement {
    let version = ToolVersion::new(VERSION);
    let builder = ToolRequirement::builder().tool(TAPLO).executable(TAPLO);

    match PLATFORMS {
        Some(platforms) => builder.exact_version_on(version, platforms).build(),
        None => builder.version(RequiredVersion::Exact(version)).build(),
    }
}
// endregion: requirement

// region: action
/// Checks that the TOML files of the project are valid
///
/// The action runs `taplo check` in the root of the project, at the version
/// that the crate pins. Taplo finds the TOML files and checks them.
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct CheckToml;

impl Action for CheckToml {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-toml")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    // region: tools
    fn tools(&self) -> Vec<ToolRequirement> {
        vec![taplo()]
    }
    // endregion: tools

    // region: run
    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        let tool = match Tool::resolve_requirement()
            .requirement(&taplo())
            .context(context)
            .call()
            .await
        {
            Ok(tool) => tool,
            Err(error) => {
                return Outcome::Errored {
                    source: Box::new(error),
                };
            }
        };

        let execution = match tool.invocation().arg("check").launch().await {
            Ok(execution) => execution,
            Err(error) => {
                return Outcome::Errored {
                    source: Box::new(error),
                };
            }
        };

        if execution.status().success() {
            return Outcome::Passed { summary: None };
        }

        // This example reports every other exit as a problem of the project,
        // also a crash of taplo. An action for real projects reads the report
        // of the tool, and gives one finding for each problem. It returns
        // `Outcome::Errored` when the tool could not examine the project.
        let report = execution.stderr().to_string_lossy();

        Outcome::Failed {
            findings: vec![
                Finding::builder()
                    .message(report.trim())
                    .location(Location::Project)
                    .build(),
            ],
            repairs: Vec::new(),
        }
    }
    // endregion: run
}
// endregion: action
