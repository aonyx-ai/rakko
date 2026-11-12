//! The command that removes the build output of the project

// region: imports
use rakko::{Context, Documented, Name, action_name};
use rakko_cli::Command;
use rakko_cli::clawless::CommandResult;
use rakko_cli::clawless::context::Context as ClawlessContext;
// endregion: imports

// region: command
/// Removes the build output of the project
///
/// Cargo writes its build output into the directory `target` in the root of
/// the project. The command removes that directory, and it says so when there
/// is nothing to remove.
#[derive(Debug, Documented)]
pub(crate) struct Clean;

impl Command for Clean {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("clean")
    }

    async fn run(
        &self,
        project: &Context,
        clawless: &ClawlessContext,
        _args: &Self::Args,
    ) -> CommandResult {
        let target = project.root().get().join("target");

        if !target.try_exists()? {
            clawless.output().message("nothing to remove").await?;
            return Ok(());
        }

        std::fs::remove_dir_all(&target)?;
        clawless.output().message("removed target").await?;

        Ok(())
    }
}
// endregion: command
