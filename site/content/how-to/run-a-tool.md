+++
title = "Run a Tool From an Action"
description = "Declare an external tool, such as a linter, and run it at the version that your action pins."
weight = 20
+++

Run an external tool from your action and convert its report into an
outcome. Pin the tool version so that projects use the version your action's
tests cover.

This guide builds `check-toml`, an action that runs [taplo] on a project's
TOML files. You need to know how to [build an action][build-an-action]. The
last step also needs a harness. If you need one, follow
[Get Started][get-started].

## Add the Helper Crate

[`rakko-tool`][rakko-tool] finds a tool through [mise] and runs it. Add it next
to `rakko`:

```sh
cargo add rakko rakko-tool
```

## Pin the Tool

Keep the tool's exact version, its pin, in `mise.rakko.toml` at your crate's
root:

{{ <example crate="check-toml" file="mise.rakko.toml" /> }}

The key is the [mise] tool name. Include the backend prefix when needed, such
as `"npm:markdownlint-cli"`. [Mise][mise] reads this file only with the `rakko`
environment active, so a plain `mise install` ignores it. [Renovate] can
update the pin through its [mise] manager.

To install the tool for the tests of your crate, run this in the root of the
crate:

```sh
MISE_ENV=rakko mise install
```

### Limit the Tool to Its Platforms

If a tool supports only certain platforms, list them in the pin. Zizmor,
for example, has no build for Windows on ARM:

```toml,name=mise.rakko.toml
[tools]
zizmor = { version = "1.32.0", os = ["linux", "macos", "windows/x64"] }
```

A platform identifies an operating system and, optionally, an architecture.
Use `linux`, `macos`, or `windows`, optionally followed by `/x64` or
`/arm64`. For example, `windows/x64` limits the tool to Windows on x64.
Rakko rejects aliases such as `aarch64`. An invalid platform name fails the
crate's build.

[Mise][mise] reads a pin with platforms only from a file that you trust.
Trust the file once:

```sh
mise trust mise.rakko.toml
```

## Read the Pin When the Crate Compiles

Read the version and platforms from `mise.rakko.toml` at compile time. This
keeps the action's declaration and the [mise] pin in one place:

{{ <example crate="check-toml" file="src/lib.rs" region="pin" /> }}

`include_str!` embeds the file in the crate. `MiseFile` reads it in a
constant, so a missing taplo pin fails the build before anyone runs the
action.

## Declare the Tool

The action declares each tool that it runs as a `ToolRequirement`. The
requirement holds the name of the tool in [mise], the program that the action
starts, and the version that the action accepts:

{{ <example crate="check-toml" file="src/lib.rs" region="requirement" /> }}

When the pin lists platforms, `exact_version_on` sets the version and the
platforms together. Otherwise, the requirement takes the exact version alone.

The action returns its requirements from `tools`:

{{ <example crate="check-toml" file="src/lib.rs" region="tools" /> }}

`tools` takes no arguments or context. Projects need the tool declarations
before they can install tools or start a run. Return a requirement for every
tool the action can use, regardless of its arguments.

### Declare a Toolchain

A toolchain is a compiler or runtime for the project, such as Rust or Node.js.
The project pins its toolchains. Declare the versions your action accepts
with `RequiredVersion::Range` and a `VersionRange` such as `>=22`. The action
uses the project's pinned version if it falls within that range. Toolchains
have no platform list.

Some tools start their toolchain by name, such as a package for Node.js that
starts `node`. Resolve the toolchain as well, and give it to the tool with
`Tool::with_toolchain`. The tool then runs on the toolchain that your action
resolved.

## Run the Tool

The run resolves the tool and starts it:

{{ <example crate="check-toml" file="src/lib.rs" region="run" /> }}

- `Tool::resolve_requirement` asks [mise] for the required program and version.
  A project pin cannot override an exact requirement. If resolution fails,
  the action returns `Outcome::Errored`.
- `invocation` describes the command and starts it from the project root.
  No shell interprets the arguments, so the tool receives them as written.
- `launch` starts the tool and waits for it to finish. On Windows, it retries
  when the operating system temporarily refuses the tool's launcher.
- A nonzero exit status is not necessarily an execution error. It can mean
  that the tool found problems. Use the status and output to choose the
  action's outcome.

A run needs a [Tokio] runtime. The harness supplies it, so your action does
not need to start one.

### Runs Outside the Platforms

If a requirement lists platforms, Rakko skips the action on any other
platform. The skip names the tool and the platform. Your action needs no
platform detection code. The same rule applies when the action belongs to a
group.

## Use the Action in a Project

After you mount the action in a project's harness, generate its tool pins
and install the tools. Run these commands from the project root:

```sh
mise run rakko -- doctor --fix
mise install
```

Until the generated file matches the mounted actions, the harness stops
actions, groups, and custom commands. Help, the command list, and `doctor`
remain available. Repeat this step when an action update changes a tool pin.
[Keep a Harness Current][keep-a-harness-current] shows how to automate it.

[Tools and Mise][tools-and-mise] explains why the action pins its tool and the
project pins its toolchains.

## Next Steps

[Test an Action][test-an-action] covers tool declarations and runs that need
installed tools. When the action is ready to share, [Publish an
Action][publish-an-action] explains how to include its [mise] file in the
package.

[build-an-action]: @/how-to/build-an-action.md
[get-started]: @/tutorial/get-started.md
[keep-a-harness-current]: @/how-to/keep-a-harness-current.md
[mise]: https://mise.jdx.dev
[publish-an-action]: @/how-to/publish-an-action.md
[rakko-tool]: https://docs.rs/rakko-tool
[renovate]: https://docs.renovatebot.com
[taplo]: https://taplo.tamasfe.dev
[test-an-action]: @/how-to/test-an-action.md
[tokio]: https://tokio.rs
[tools-and-mise]: @/explanation/tools-and-mise.md
