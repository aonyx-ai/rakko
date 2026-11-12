//! The example action of the guide "Build an Action"
//!
//! The site shows the regions of this file between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests the file, so the code on
//! the site always compiles against the current `rakko` crate.

// region: imports
use std::fs;

use rakko::{
    Access, Action, Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, Context, Documented,
    Finding, Location, Name, Outcome, ReadArgsError, Summary, action_name, argument_name,
};
// endregion: imports

// region: args
/// The arguments that a run of check-readme reads
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckReadmeArgs {
    /// Whether the run writes a README when the project has none
    fix: bool,
}

impl Args for CheckReadmeArgs {
    fn schema() -> ArgsSchema {
        ArgsSchema::new([Argument::builder()
            .name(argument_name!("fix"))
            .shape(ArgumentShape::Boolean)
            .documentation("Write a README when the project has none")
            .build()])
    }

    fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
        let name = argument_name!("fix");
        let fix = match values.get(&name) {
            Some(value) => value
                .get()
                .parse()
                .map_err(|_| ReadArgsError::UnreadableValue {
                    name,
                    value: value.clone(),
                })?,
            None => false,
        };

        Ok(Self { fix })
    }
}
// endregion: args

// region: action
/// The text that a repair writes into a new README
const README: &str = "# Project\n";

/// Checks that the project has a README
///
/// A README is the first page that a new contributor reads. The action looks
/// for `README.md` in the root of the project. With `--fix`, it writes one
/// when the project has none.
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct CheckReadme;

impl Action for CheckReadme {
    type Args = CheckReadmeArgs;

    fn name(&self) -> Name {
        action_name!("check-readme")
    }

    // region: access
    fn access(&self, _context: &Context, args: &Self::Args) -> Access {
        if args.fix {
            Access::Write
        } else {
            Access::Read
        }
    }
    // endregion: access

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        let readme = context.root().get().join("README.md");

        match readme.try_exists() {
            Ok(true) => Outcome::Passed {
                summary: Some(Summary::new("found README.md")),
            },
            Ok(false) if args.fix => match fs::write(&readme, README) {
                Ok(()) => Outcome::Changed {
                    repairs: vec![missing_readme()],
                },
                Err(error) => Outcome::Errored {
                    source: Box::new(error),
                },
            },
            Ok(false) => Outcome::Failed {
                findings: vec![missing_readme()],
                repairs: Vec::new(),
            },
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Returns the finding that reports a project without a README
fn missing_readme() -> Finding {
    Finding::builder()
        .message("the project has no README.md")
        .location(Location::Project)
        .build()
}
// endregion: action
