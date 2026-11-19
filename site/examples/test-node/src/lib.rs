//! The example action of the guide "Use a Toolchain From an Action"
//!
//! The action runs the test runner of Node, a toolchain of the project. The
//! project pins Node, and the action runs on that pin. The action requires a
//! release of Node that runs test files in TypeScript, because an older
//! release skips them and passes.
//!
//! The site shows the regions of this file between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests the file, so the code on
//! the site always compiles against the current `rakko` and `rakko-tool`
//! crates.

// region: imports
use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, ToolchainRequirement,
    VersionRange, action_name,
};
use rakko_tool::{Launch, ShownStreams, Tool};
// endregion: imports

// region: action
/// Runs the tests of the project with the test runner of Node
///
/// The action runs `node --test` in the root of the project, on the Node that
/// the project pins. Node finds the test files and runs them.
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct TestNode;

impl Action for TestNode {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("test-node")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    // region: toolchains
    fn toolchains(&self) -> Vec<ToolchainRequirement> {
        // Node runs test files in TypeScript from 22.18 and 23.6 on. An older
        // release skips these files and reports no problem, so the run passes
        // without a test.
        vec![
            ToolchainRequirement::builder()
                .toolchain("node")
                .versions(VersionRange::new("^22.18 || >=23.6"))
                .build(),
        ]
    }
    // endregion: toolchains

    // region: run
    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        let node = match Tool::resolve_toolchain()
            .action(self)
            .toolchain("node")
            .executable("node")
            .context(context)
            .call()
            .await
        {
            Ok(node) => node,
            Err(error) => {
                return Outcome::Errored {
                    source: Box::new(error),
                };
            }
        };

        let execution = match node
            .invocation()
            .arg("--test")
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

        // This example reports every other exit as a failed test, also a
        // crash of Node. An action for real projects reads the report of the
        // test runner, and gives one finding for each test that failed.
        let report = execution.stdout().to_string_lossy();

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
