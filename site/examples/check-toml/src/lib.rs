//! The example action of the guide "Run a Tool From an Action"
//!
//! The action runs taplo, an external tool, to check the TOML files of a
//! project. The crate pins taplo in its `mise.rakko.toml`, declares the tools
//! of that file, and runs taplo at the version of its pin through mise.
//!
//! The site shows the regions of this file between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests the file, so the code on
//! the site always compiles against the current `rakko` and `rakko-tool`
//! crates.

// region: imports
use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, Tools, action_name,
};
use rakko_tool::{Launch, ShownStreams, Tool};
// endregion: imports

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
    fn bundled_tools(&self) -> Tools {
        rakko::bundled_tools!()
    }
    // endregion: tools

    // region: run
    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        let tool = match Tool::resolve()
            .action(self)
            .tool("taplo")
            .executable("taplo")
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

        let execution = match tool
            .invocation()
            .arg("check")
            .launch_with(context.tool_output(), ShownStreams::Both)
            .await
        {
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
