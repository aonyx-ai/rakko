+++
title = "Publish an Action"
description = "Publish your action on crates.io so that other projects can find and mount it."
weight = 70
+++

Publish your action on crates.io so that other projects can add it to their
harnesses. You control its releases, and users adopt them through dependency
updates.

This guide publishes `otter-check-readme`, the action of [Build an
Action][build-an-action]. You need a working action with tests. To try it in
a project first, set up a harness with [Get Started][get-started].

## A Crate of Your Own

Keep your action in your own crate and repository. This lets you maintain it
for your workflow and release it on your schedule. The Rakko repository
contains actions for Aonyx's projects and does not accept new actions from
other authors. The [support policy][support] explains this scope.

## Fill In the Manifest

crates.io needs a description and a license for each crate. Add them to the
`[package]` table of your `Cargo.toml`, with the repository of the crate and
the keyword `rakko-action`:

```toml,name=Cargo.toml
[package]
name = "otter-check-readme"
description = "Checks that a project has a README"
license = "MIT OR Apache-2.0"
repository = "https://github.com/otter/otter-check-readme"
keywords = ["rakko-action"]
```

The `rakko-action` keyword helps people find your crate alongside other
Rakko actions on crates.io.

Each library dependency must have a version available on crates.io. Use
`cargo add` to add registry dependencies to the manifest.

## Write a README

crates.io uses your `README.md` as the crate's page. Describe what the action
does, which tools it requires, and how to mount it in a harness. Include the
command name and flags so that readers can run it. [Build an Action][mount]
shows the mount code.

## Check the Package

If your action embeds `mise.rakko.toml`, include that file in the package.
List the package files and make sure that the [mise] file appears:

```sh
cargo package --list
```

Build the package without uploading it:

```sh
cargo publish --dry-run
```

## Publish the Crate

```sh
cargo publish
```

Then give the repository of the crate the GitHub topic `rakko-action`, so that
others find it on GitHub as well. With the GitHub command line, run this in
the repository:

```sh
gh repo edit --add-topic rakko-action
```

You can also add the topic in the settings of the repository on GitHub.

## Release a New Version

Projects adopt new releases through dependency updates, for example with
Renovate. If a release changes a tool pin, users must run
`mise run rakko -- doctor --fix` before their harness commands can run again.
Include that step in the release notes. [Keep a Harness
Current][keep-a-harness-current] explains how projects can automate it.

[build-an-action]: @/how-to/build-an-action.md
[get-started]: @/tutorial/get-started.md
[keep-a-harness-current]: @/how-to/keep-a-harness-current.md
[mise]: https://mise.jdx.dev
[mount]: @/how-to/build-an-action.md#mount-it-in-a-harness
[support]: https://github.com/aonyx-ai/rakko#support
