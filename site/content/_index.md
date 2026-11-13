+++
title = "Rakko"
description = "Rakko turns project maintenance into versioned Rust crates."

[extra]
headline = "Project Maintenance as Rust Crates"
lede = "Turn your maintenance tasks into Rust crates. Run them from one command line, share them across projects, and update them like any other dependency."
needs = "Use [mise] to install Rust and the tools your actions need."

# The dependencies of the harness in the hero. The Rust code under them comes
# from the example crate `examples/landing`, which adds these actions to its
# command line. A test of that crate checks that the names here match it.
harness_manifest = """
[dependencies]
aonyx-rakko-format-rust = "0.1"
aonyx-rakko-lint-rust = "0.1"
aonyx-rakko-test-rust = "0.1"
rakko-cli = "0.1"
"""

# The facts in the strip under the first view of the landing page.
facts = ["Open source, MIT or Apache 2.0", "Written in Rust", "Uses [mise] and [Clawless][clawless]"]

features_title = "How Rakko Works"
features_intro = "You fix a maintenance script in one project, then copy the fix to the others. With Rakko, each task is a versioned crate. Release the fix once, and let projects adopt it through dependency updates."

# The three layers, from the top down: the command line, the actions, and
# the tools.
layers_title = "Rakko, [Mise][mise], and [Clawless][clawless]"
layers_intro = "Rakko defines and runs your maintenance tasks. [Clawless][clawless] provides the command line, and [mise] installs the tools."

[[extra.features]]
title = "Actions"
text = "An action is a maintenance task in a Rust library crate. Use an existing action or write one for your workflow."

[[extra.features]]
title = "The Harness"
text = "A harness is a small binary crate in your project. Add the actions you want, and Rakko gives each one a command."

[[extra.features]]
title = "Bundles"
text = "A bundle packages a set of actions. Add the set to your harness at once, or choose the actions you need."

[[extra.features]]
title = "Updates"
text = "Let your dependency bot open a pull request for a new action release. Review the update and its CI results before you merge."

[[extra.layers]]
name = "Clawless"
url = "https://clawless.rs"
role = "provides the command line, with help text and shared output flags."

[[extra.layers]]
name = "Rakko"
role = "defines actions and groups, and runs the tasks your harness selects."

[[extra.layers]]
name = "mise"
url = "https://mise.jdx.dev"
role = "installs linters, formatters, and other tools at the versions your actions require."
+++

## Your Own Actions

Aonyx uses Rakko internally and builds actions for its own projects. You are
welcome to try them and use the ones that fit your workflow.

We encourage you to [build your own actions][build-an-action] too. They can
reflect your team's conventions or leave more choices to each project. An
action is an ordinary Rust crate that you can test with Rakko's helpers and
publish for others to use.

[build-an-action]: @/how-to/build-an-action.md
