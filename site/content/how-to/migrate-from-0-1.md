+++
title = "Migrate From 0.1"
description = "Move an action, a helper crate, or a harness from Rakko 0.1 to the next release. Its actions take their tools from a mise file and declare toolchain requirements only when they need them."
weight = 80
+++

Move your action or your harness from Rakko 0.1 to the next release. An action
of the new release states its name, arguments, access, and run. Its tools come
from its `mise.rakko.toml`, and a toolchain requirement is optional.

The release breaks the API of 0.1 once, with no deprecated items in between.
Each crate that changes takes a new 0.x version, such as 0.2. Cargo treats that
as a break, so update every crate of Rakko in one step. The crates go to 1.0
after every harness of Aonyx runs a release for four weeks without a break.
This release starts those four weeks again.

## Changes of the Release

In 0.1, an action built a `ToolRequirement` for each program that it ran. An
exact version marked a tool that the action pins. A range marked a toolchain
that the project pins, such as Node.js. Every range was also a condition that
stopped every command of the harness.

The new release separates these:

- **Tools come from the mise file.** `bundled_tools!` reads every pin of the
  `mise.rakko.toml` of the crate when it compiles. No code builds a
  `ToolRequirement`.
- **The run gives the executable.** A declaration holds no program.
  `Tool::resolve` takes the tool and the executable to start.
- **Toolchains are separate and optional.** `Tool::resolve_toolchain` runs the
  program on the pin of the project. An action declares a
  `ToolchainRequirement` only for a need of its own. For example, an older
  release gives a wrong result. Or the error of the toolchain does not say
  that its pin is the cause.
- **An unmet requirement stops only its action.** In 0.1, it was a
  precondition: a condition that the harness checks before it runs any
  command. Now it stops no other action. `doctor` still reports it.

## Update the Dependencies

Change each crate of Rakko in the `Cargo.toml` of your action and of your
harness to the new release, together:

```toml,name=Cargo.toml
[dependencies]
rakko = "0.2"
rakko-tool = "0.2"

[dev-dependencies]
rakko-test-utils = "0.2"
```

Update `rakko-cli` and the actions and bundles of Aonyx, such as
`aonyx-rakko-baseline`, in the harness at the same time. Renovate opens one
pull request for the group, because the preset of Rakko groups its crates.
`rakko-macros` does not change, and `rakko` depends on it.

Then write the tool pins of the project again:

```sh
mise run rakko -- doctor --fix
mise install
```

## Declare Your Own Pins

An action that wraps a tool read its pin with `MiseFile` and built its
requirement by hand in 0.1:

```rust,name=src/lib.rs (0.1)
const PINS: MiseFile = MiseFile::new(include_str!("../mise.rakko.toml"));
const VERSION: &str = PINS.pinned_version("taplo");

fn tools(&self) -> Vec<ToolRequirement> {
    vec![
        ToolRequirement::builder()
            .tool("taplo")
            .executable("taplo")
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build(),
    ]
}
```

Remove the constants, the builder, and `tools`. Return the pins of the file
from `bundled_tools` instead:

{{ <example crate="check-toml" file="src/lib.rs" region="tools" /> }}

The pins keep their versions and their platforms, and the file stays where it
is. `bundled_tools!` declares every pin of the file for each action that calls
it. So a crate with several actions gives each of them every tool of the file.
Keep in the file only the tools that the actions of the crate run.

A pin with platforms needs no code either. The action still skips on another
platform, as in 0.1.

## Use the Pins of a Helper Crate

A helper crate that wraps a tool offered a function that built its requirement
in 0.1. It now keeps the pin in its own `mise.rakko.toml`, and it offers the
pins of that file as a constant with the value `rakko::bundled_tools!()`. For
example, `aonyx-rakko-taplo` offers `Taplo::TOOLS`.

An action that runs the tool through the helper returns that constant from
`additional_tools`, such as `vec![Taplo::TOOLS]`. An action can use both
`bundled_tools` and `additional_tools`. `tools` combines the two and lists each
pin once, so an action does not implement it.

The lookup of the helper is now a builder that takes the action, such as
`Taplo::resolve().context(context).action(self).call()`. `Tool::resolve` reads
the tools of the action that runs, so the helper passes the action on to it.

An action that does not return the constant of the helper still compiles. Its
run then stops with an error whose cause is `UndeclaredTool`, because the
action does not declare the tool.

## Give the Executable

In 0.1, the requirement held the program, and `Tool::resolve_requirement`
took the requirement. Now the run gives the declared tool and its program:

{{ <example crate="check-toml" file="src/lib.rs" region="run" /> }}

The lookup takes the version from the declarations of the action, and it
refuses a tool that the action does not declare. One tool can provide several
programs, such as `renovate-config-validator` of `npm:renovate`. One pin
covers each of them.

## Resolve a Toolchain

In 0.1, an action declared a toolchain as a `ToolRequirement` with a range,
and it resolved it with `Tool::resolve_requirement`:

```rust,name=src/lib.rs (0.1)
ToolRequirement::builder()
    .tool("node")
    .executable("node")
    .version(RequiredVersion::Range(VersionRange::new(">=20")))
    .build()
```

Remove that requirement from `tools`. Resolve the toolchain in the run with
`Tool::resolve_toolchain`. Without a requirement, it takes the first pin of
the project:

{{ <example crate="test-node" file="src/lib.rs" region="run" /> }}

Then decide whether the range stays. If the action needs it, keep it. A pass
without a test on an older release is such a need. If the range only copies
the `engines` field of an npm package, remove it. The tool reports an
unsupported release with its own error. Move a range that stays to
`toolchains`:

{{ <example crate="test-node" file="src/lib.rs" region="toolchains" /> }}

An action without a requirement still runs on the pin of the project. The
lookup never takes a toolchain from the `PATH` of the environment.

Some runs select a pin by a fact that only the run knows, such as the
`rust-version` of a manifest. Give that range to `resolve_toolchain` with
`accepting`. It applies to that lookup alone, and `doctor` does not report it.

A tool that starts its toolchain by name, such as a package for Node.js,
still gets the toolchain from `Tool::with_toolchain`. First resolve the tool
with `Tool::resolve`. Then resolve the toolchain with
`Tool::resolve_toolchain`. [Use a
Toolchain From an Action][use-a-toolchain] explains each step.

## Update the Errors

`ResolveRequirementError` is gone. `Tool::resolve` returns a
`ResolveToolError`, and `Tool::resolve_toolchain` and `Tool::with_toolchain`
return a `ResolveToolchainError`. An error of your action that wrapped
`ResolveRequirementError` gets a variant for each lookup that its run makes.

In 0.1, a pin outside a range stopped every command before any run. Now the
lookup returns the error during the run of the action, which returns it as
`Outcome::Errored`. The action stops, and it does not skip.

## Update the Tests

A test that read the tools of an action still calls `tools`. Each
`ToolRequirement` holds the tool, the exact version, and the platforms. It has
no `executable`, and `version` returns a `ToolVersion` and no longer a
`RequiredVersion`:

{{ <example crate="check-toml" file="tests/check_toml.rs" region="tools" /> }}

A test of a toolchain requirement reads `toolchains` instead. [Use a Toolchain
From an Action][use-a-toolchain] shows one.

## Migrate a Harness

A harness that only mounts actions needs the new releases and a run of
`doctor --fix`. Some behavior of the command line changes:

- An unmet toolchain requirement no longer stops every command. Only the
  action that declares it stops, and a group that holds it fails. The other
  actions, groups, and custom commands run.
- `doctor` reports only the toolchain requirements that the mounted actions
  declare. Some actions run on a toolchain without a requirement, such as
  `format-terraform` on Terraform or `lint-rust` on Rust. They do not make
  `doctor` report a missing pin. Such an action stops with an error when it
  runs.
  So `doctor` no longer reports a project that pins no stable release of
  Rust.
- `doctor` and `doctor --fix` still exit with code 1 while a requirement is
  unmet. So the `rakko-update` workflow still fails, and pushes nothing, until
  you change the pin.
- The actions of Aonyx accept more toolchain pins. The Rust actions take
  every pinned stable release, and the actions that need a nightly take every
  dated nightly. The actions that run a package for Node.js take every pinned
  release of Node.js, and `check-typescript` takes every release of
  TypeScript. `format-terraform` takes every release of Terraform. Only
  `test-typescript` still requires a release of Node.js, `^22.18 || >=23.6`.

A harness that matches the errors of the actions of Aonyx, such as in a custom
command, also follows the changes in the next section.

## Every Removed or Changed Item

The harness API is everything that the code of a harness and the workflows
of a project depend on. It holds the crates of Rakko, the actions, bundles,
and commands of Aonyx, and the command line. These lists hold every item of
the harness API that the release removes or changes, crate by crate. An item
that the release only adds is not in them.

### The Releases

Every crate of Rakko takes a breaking release, except `rakko-macros`, which
does not change. Each other crate shows a type of `rakko` in its API, such as
`ProjectRoot` or `ErasedAction`. So it changes with `rakko`, even when its own
items stay. `aonyx-rakko-node` is new. It holds the lookups of Node.js that the
actions for Node.js share.

### `rakko`

| Item                                            | Change                                                                  | Use instead                                                                         |
| ----------------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `RequiredVersion`                               | Removed                                                                 | A pin gives a `ToolVersion`, and a range moves to a `ToolchainRequirement`          |
| `ToolRequirement::builder` and its builder type | Removed                                                                 | `rakko::bundled_tools!` in `Action::bundled_tools`                                  |
| `exact_version_on` of the builder               | Removed                                                                 | The `os` option of the pin in `mise.rakko.toml`                                     |
| `ToolRequirement::executable`                   | Removed                                                                 | The `executable` of `Tool::resolve` or `Tool::resolve_toolchain`                    |
| `ToolRequirement::version`                      | Returns a `ToolVersion` and no longer a `RequiredVersion`               | The version of the pin, as it is                                                    |
| `Action::tools`                                 | The default returns the tools of `bundled_tools` and `additional_tools` | `bundled_tools` and `additional_tools`, and `toolchains` for a range of a toolchain |
| `ErasedAction`                                  | Gains the method `toolchains`                                           | Nothing, because only `rakko` implements the trait                                  |

### `rakko-tool`

| Item                        | Change                                                                      | Use instead                                                                |
| --------------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `MiseFile`                  | Removed                                                                     | `rakko::bundled_tools!`                                                    |
| `PinnedPlatforms`           | Removed                                                                     | The `os` option of the pin in `mise.rakko.toml`                            |
| `Tool::resolve_requirement` | Removed                                                                     | `Tool::resolve` for a tool, and `Tool::resolve_toolchain` for a toolchain  |
| `ResolveRequirementError`   | Removed                                                                     | `ResolveToolError` for a tool, and `ResolveToolchainError` for a toolchain |
| `Tool::with_toolchain`      | Returns a `ResolveToolchainError` and no longer a `ResolveRequirementError` | The new error                                                              |

### `rakko-cli`

The builder and the other items do not change. `doctor` and its exit codes
stay. An unmet toolchain requirement is no longer a precondition of every
command, as [Migrate a Harness][harness] says.

### `rakko-test-utils`

No item changes. A test that reads `ToolRequirement::executable` or a
`RequiredVersion` changes as [Update the Tests][tests] says.

### The Helper Crates

| Crate                  | Item                             | Change                                                                                                                          | Use instead                                                    |
| ---------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `aonyx-rakko-cargo`    | `Cargo::requirement`             | Removed                                                                                                                         | Nothing, because Rust is a toolchain of the project            |
| `aonyx-rakko-cargo`    | `Cargo::resolve`                 | Becomes a builder that takes the action, the context, and an optional `ToolchainSelection`, and returns `ResolveToolchainError` | The new builder and error                                      |
| `aonyx-rakko-cargo`    | `Toolchain::nightly`             | Removed                                                                                                                         | `Toolchain::nightly_requirement` in `Action::toolchains`       |
| `aonyx-rakko-cargo`    | `Toolchain::release`             | Removed                                                                                                                         | `ToolchainSelection::Release` for `Toolchain::resolve`         |
| `aonyx-rakko-cargo`    | `Toolchain::resolve_requirement` | Removed                                                                                                                         | `Toolchain::resolve` with a `ToolchainSelection`               |
| `aonyx-rakko-nextest`  | `Nextest::requirement`           | Removed                                                                                                                         | `Nextest::TOOLS` in `Action::additional_tools`                 |
| `aonyx-rakko-nextest`  | `Nextest::resolve`               | Takes the action, and returns `ResolveToolError`                                                                                | The new argument and error                                     |
| `aonyx-rakko-prettier` | `Prettier::requirement`          | Removed                                                                                                                         | `Prettier::TOOLS` in `Action::additional_tools`                |
| `aonyx-rakko-prettier` | `Prettier::node_requirement`     | Removed                                                                                                                         | Nothing, because the action runs on the Node.js of the project |
| `aonyx-rakko-prettier` | `Prettier::resolve`              | Becomes a builder that takes the action and the context, and returns `aonyx_rakko_node::ResolveNodeToolError`                   | The new builder and error                                      |
| `aonyx-rakko-shfmt`    | `Shfmt::requirement`             | Removed                                                                                                                         | `Shfmt::TOOLS` in `Action::additional_tools`                   |
| `aonyx-rakko-shfmt`    | `Shfmt::resolve`                 | Becomes a builder that takes the action and the context, and returns `ResolveToolError`                                         | The new builder and error                                      |
| `aonyx-rakko-taplo`    | `Taplo::requirement`             | Removed                                                                                                                         | `Taplo::TOOLS` in `Action::additional_tools`                   |
| `aonyx-rakko-taplo`    | `Taplo::resolve`                 | Becomes a builder that takes the action and the context, and returns `ResolveToolError`                                         | The new builder and error                                      |

`aonyx-rakko-worktree` changes no item.

### The Errors of the Actions

Each variant that held a `ResolveRequirementError` now holds the error of its
lookup. The other variants do not change.

| Crate                               | Item                                         | Change                                                      |
| ----------------------------------- | -------------------------------------------- | ----------------------------------------------------------- |
| `aonyx-rakko-build-internal-docs`   | `BuildInternalDocsError::UnresolvedTool`     | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-crate-owners`    | `CheckCrateOwnersError::UnresolvedCargo`     | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-dependencies`    | `CheckDependenciesError::UnresolvedCargo`    | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-dependencies`    | `CheckDependenciesError::UnresolvedDeny`     | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-check-latest-deps`     | `CheckLatestDepsError::UnresolvedCargo`      | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-latest-deps`     | `CheckLatestDepsError::UnresolvedNextest`    | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-check-minimal-deps`    | `CheckMinimalDepsError::UnresolvedCargo`     | Removed, because the action resolves only the nightly       |
| `aonyx-rakko-check-minimal-deps`    | `CheckMinimalDepsError::UnresolvedNextest`   | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-check-minimal-deps`    | `CheckMinimalDepsError::UnresolvedToolchain` | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-msrv`            | `CheckMsrvError::UnresolvedTool`             | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-msrv`            | `CheckMsrvError::UnresolvedToolchain`        | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-renovate-config` | `CheckRenovateConfigError::UnresolvedTool`   | Holds an `aonyx_rakko_node::ResolveNodeToolError`           |
| `aonyx-rakko-check-typescript`      | `CheckTypeScriptError::UnresolvedTool`       | Removed, and `UnresolvedToolchain` replaces it              |
| `aonyx-rakko-check-typescript`      | `CheckTypeScriptError::UnresolvedToolchain`  | New, holds an `aonyx_rakko_node::ResolveNodeToolchainError` |
| `aonyx-rakko-check-unused-deps`     | `CheckUnusedDepsError::UnresolvedCargo`      | Removed, because the action resolves only the nightly       |
| `aonyx-rakko-check-unused-deps`     | `CheckUnusedDepsError::UnresolvedToolchain`  | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-check-unused-deps`     | `CheckUnusedDepsError::UnresolvedUdeps`      | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-format-json`           | `FormatJsonError::UnresolvedTool`            | Holds an `aonyx_rakko_node::ResolveNodeToolError`           |
| `aonyx-rakko-format-markdown`       | `FormatMarkdownError::UnresolvedTool`        | Holds an `aonyx_rakko_node::ResolveNodeToolError`           |
| `aonyx-rakko-format-rust`           | `FormatRustError::UnresolvedTool`            | Removed, because the action resolves only the nightly       |
| `aonyx-rakko-format-rust`           | `FormatRustError::UnresolvedToolchain`       | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-format-shell`          | `FormatShellError::UnresolvedTool`           | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-format-terraform`      | `FormatTerraformError::UnresolvedTool`       | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-format-toml`           | `FormatTomlError::UnresolvedTool`            | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-format-typescript`     | `FormatTypeScriptError::UnresolvedTool`      | Holds an `aonyx_rakko_node::ResolveNodeToolError`           |
| `aonyx-rakko-format-yaml`           | `FormatYamlError::UnresolvedTool`            | Holds an `aonyx_rakko_node::ResolveNodeToolError`           |
| `aonyx-rakko-lint-github-actions`   | `LintGitHubActionsError::UnresolvedTool`     | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-markdown`         | `LintMarkdownError::UnresolvedTool`          | Holds an `aonyx_rakko_node::ResolveNodeToolError`           |
| `aonyx-rakko-lint-rust`             | `LintRustError::UnresolvedTool`              | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-lint-shell`            | `LintShellError::UnresolvedShellcheck`       | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-shell`            | `LintShellError::UnresolvedShfmt`            | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-stiles`           | `LintStilesError::UnresolvedTool`            | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-terraform`        | `LintTerraformError::UnresolvedTool`         | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-toml`             | `LintTomlError::UnresolvedTool`              | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-typescript`       | `LintTypeScriptError::UnresolvedTool`        | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-lint-yaml`             | `LintYamlError::UnresolvedTool`              | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-test-rust`             | `TestRustError::UnresolvedCargo`             | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-test-rust`             | `TestRustError::UnresolvedNextest`           | Holds a `ResolveToolError`                                  |
| `aonyx-rakko-test-rust-docs`        | `TestRustDocsError::UnresolvedTool`          | Holds a `ResolveToolchainError`                             |
| `aonyx-rakko-test-typescript`       | `TestTypeScriptError::UnresolvedTool`        | Removed, and `UnresolvedToolchain` replaces it              |
| `aonyx-rakko-test-typescript`       | `TestTypeScriptError::UnresolvedToolchain`   | New, holds a `ResolveToolchainError`                        |

The bundles and the commands change no item of their own. Each takes the new
releases of its actions.

[harness]: @/how-to/migrate-from-0-1.md#migrate-a-harness
[tests]: @/how-to/migrate-from-0-1.md#update-the-tests
[use-a-toolchain]: @/how-to/use-a-toolchain.md
