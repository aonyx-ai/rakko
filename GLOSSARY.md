# Glossary

This glossary defines the terms of the Rakko design. One term has one
meaning, and one concept has one term. The Retired Terms section maps words
from the design discussions to the terms that replaced them.

## Terms

- **Access** — The declaration of whether a run of an action reads or writes
  the project. An action declares it after its arguments are bound and before
  it runs. The declaration is a function of the arguments and the context, so
  a `--fix` flag changes it, and it does no I/O. A declaration covers the
  whole project: a write is exclusive access to it, and a read is shared
  access. The default declaration is a write. The scheduler runs the writers
  of a group one at a time and then all its readers in parallel.
- **Action** — The unit of maintenance work. An action is a library crate
  that implements the `Action` trait. An action depends on the contract
  crate and on the helper crates that it uses. It never depends on the
  projection or on Clawless.
- **Bound action** — An erased action together with the arguments of one
  run. The machinery binds the values of a run to the arguments of the
  action once. Then the bound action declares its access and runs, and both
  see the same arguments.
- **Bundle** — A meta-crate that exports a list of actions. A bundle defines
  what a word such as "recommended" means, as a dependency. Bundles can
  contain other bundles. A bundle release starts a rollout.
- **Check** — An informal word for an action that examines the project and
  reports findings. A check is not a formal category: the system has one
  `Action` trait and no subtypes.
- **Clawless** — Aonyx's framework for command-line applications, and the
  layer that Rakko builds its command line on. [Clawless] turns mounted
  actions into commands, with help text and shared output flags. Actions do
  not depend on Clawless.
- **Command** — One thing that a user of the command line of a harness can
  run. The projection derives one command from each mounted action and from
  each mounted group, and a harness writes the others, for a maintenance
  activity of the project that fits neither. A derived command is not the
  action or the group itself. A written command is an ordinary Clawless
  command, and it knows nothing about actions.
- **Context** — The data that is relevant when an action runs. Today this is
  the project root and the tool output. Later it can include user variables
  and loaded configuration. Clawless uses the same name for a
  similar concept. This parallel is intentional. An action never sees the
  Clawless type, and a command receives both.
- **Contract crate** — The `rakko` crate. It contains the `Action` trait,
  the access types, and the other shared types. The contract crate stays
  small, because every action and every harness depends on it. The contract
  crate also contains the trait of a group, which uses only its types. The
  `rakko-macros` crate holds its derive macros. Rakko publishes that crate
  together with the contract crate, because the contract crate depends on
  it. The contract crate exports the macros again, so no other crate depends
  on `rakko-macros`.
- **`doctor`** — The command that every harness has. The harness writes no
  code for it. It checks the precondition on the tools. The generated mise
  file `.config/mise/conf.d/rakko.toml` must match the tools that the mounted
  actions declare. The check compares the values of the file, so a formatter
  can change its layout. No pin of the project can override a declared tool. The
  project must pin each toolchain that an action requires in its range. The
  check needs no network. `doctor --fix` writes the generated file, removes
  each pin that overrides a declared tool, and locks the tools with mise. It
  does not change the pin of a toolchain, and it says which pin to change.
- **Erased action** — The internal, object-safe view of an action. Bundles
  export erased actions, and the registry in the harness holds them. Action
  authors do not see this type. A harness that only mounts does not write it
  either, because the builder of the command line, or the bundle that holds a
  list, erases each action.
- **Finding** — One problem that an action found in the project, with its
  location. Findings travel in the outcome of an action run.
- **Fleet** — All Aonyx projects that Rakko maintains.
- **Group** — A named list of actions, its members, that the command line
  runs as one command, such as pre-commit. A group is a concept of the public
  API, next to the action and the harness. The projection derives the flags
  of the group from the arguments of its members, and a flag reaches only the
  members that declare its argument. A group has no arguments of its own. A
  group is a type with a name and its members and nothing else, and the
  scheduler runs the members. When the members run, the group answers like
  one action, with one summary and the exit code of the worst outcome of its
  members. The trait of a group lives in the contract crate.
- **Harness** — The small binary crate in each project. The harness mounts
  the bundles, actions, and groups that the project uses, writes the
  commands of the project that do not fit an action, and `mise run rakko`
  runs it. The harness is a package of its own, outside the workspace of the
  project, and it is the full adoption surface of a project.
- **Harness API** — Everything that the code of a harness and the workflows
  of a project depend on. It has four parts. The first part is the builder of
  the projection. The second part is the items of the catalog that a harness
  uses, with the types that they expose. The third part is the contract
  crate, the helper crates, and `rakko-test-utils`, for a harness that writes
  its own actions or groups. The fourth part is the command line that the
  workflows of a project call. The document that a command writes with
  `--json` is not part of the harness API. [ADR-026] records what Rakko
  promises about the harness API from one version to the next.
- **Helper crate** — A crate that carries machinery that many actions need.
  `rakko-tool` is the first one: it finds an external tool through mise and
  runs it. An action depends on the helper crates that it uses next to the
  contract crate. A helper stays out of the contract crate because the
  contract reaches every action, while a helper reaches only the actions
  that use it.
- **Isolation** — (Planned.) A run in a copy of the project that the machinery
  removes after the run. Nothing that the run changes reaches the project.
  Isolation does not protect a user from an action that they do not trust.
- **Linking** — The use of an external tool as a Rust library inside an
  action. Linking is an optimization for one action at a time, taken when
  the library carries the behavior of the tool, not only its engine.
- **mise** — The provisioning layer. [Mise] installs the external tools at
  pinned versions. Renovate updates the pins in the mise files of the
  project. It also updates the mise file of each crate that pins its tool,
  such as `aonyx-rakko-taplo`.
- **`mise run rakko`** — The mise task that builds and runs the harness of a
  project. The task lives in the `mise.toml` file of each project. It is the
  canonical form on every platform, and documentation always writes it out,
  because a shorter form exists only where a platform can supply one.
- **Mount** — To register actions, bundles, groups, or written commands in
  the CLI of a harness. The harness names what it mounts, in code.
- **Outcome** — The result of one action run. An outcome tells whether the
  action passed, changed the project by repairing everything that it found,
  failed with findings, skipped, or stopped with an error. An action skips
  when there was nothing to examine, or when a tool of the action has no
  build for the platform of the run.
- **Platform** — An operating system, or an operating system with an
  architecture, such as Linux, or macOS on ARM. The pin of a tool can list
  the platforms where the tool has a build. The list uses the form of the
  `os` option of mise, such as `linux` or `macos/arm64`. Mise then installs
  the tool only on those platforms. On another platform, the machinery
  skips a run of an action that pins the tool. A pin without such a list
  needs the tool on every platform.
- **Precondition** — A condition about the environment of the project that a
  correct run needs, such as a current file of tool pins. The projection
  checks the preconditions before it runs an action, a command, or a group,
  and a failed precondition stops the run, also in CI. The help, the list of
  the commands, and the `doctor` command still run. `doctor` reports the
  preconditions, and `doctor --fix` repairs them.
- **Project** — The unit that Rakko maintains: a directory tree with its own
  tools, actions, and harness. A project is usually a Git repository, but
  Rakko does not require one. A project marks its root with
  `.config/rakko.toml`, and a run searches for that file from the directory
  that it starts in.
- **Projection** — The `rakko-cli` crate. It turns the actions and the groups
  that a harness mounts into commands, with the help text and the shared
  output flags, and it defines the trait that a written command implements.
  Only a harness, and a crate that provides a written command, depends on
  it, so Clawless never reaches an action or a crate that provides a group.
  Its scheduler runs the members of every group.
- **Rakko** — This project: a toolkit that turns project maintenance into
  versioned Rust crates. The rakko (ラッコ) is the Japanese sea otter, the
  otter that keeps a pebble as its tool.
- **Renderer** — The part of the machinery that turns the outcome of a run
  into output. One renderer serves a reader at a terminal, one serves a
  machine that reads JSON, and one serves the pull request of a CI run. A
  renderer reads the outcome and nothing else, so a new output target costs a
  renderer and changes no action. Clawless has a presenter, which is a
  different part: it carries what a renderer produced to a stream.
- **Renovate** — The update and rollout mechanism. [Renovate] opens pull
  requests for new action versions in `Cargo.toml`, and for new tool
  versions in `mise.toml`.
- **Repair** — One problem that an action found in the project and corrected,
  with its location. A repair takes the shape of a finding, because it is the
  problem that the project no longer has. Repairs travel in the outcome of an
  action run, whether the action repaired everything that it found or only
  part of it.
- **Rollout** — The propagation of a bundle release across the fleet. Each
  project receives a Renovate pull request, and the CI result of that pull
  request shows whether the project complies.
- **Sandbox** — (Not planned.) A security boundary that protects a user from an
  action that they do not trust. A Rust action runs in the process of the
  harness, and its build scripts run when the harness compiles, so a copy of the
  project does not contain it.
- **Scheduler** — The part of the projection that runs the members of a
  group. It reads the access declaration of each member before any member
  runs, and a kind of access that it does not know stops the group there.
  Then it runs the writers one at a time, in the order of the list, and then
  all the readers in parallel. Every member runs to its end, also when
  another member fails. The scheduler reports each member when the member
  finishes, and the summary of the group at the end. The scheduler is
  internal to the projection.
- **Tool** — An external program that an action uses, for example prettier
  or taplo. Mise provisions tools. In Rakko documentation, "tool" always
  means an external program.
- **Tool output** — The place on the context where the tools of an action
  send what they write while they run. For each tool, it receives the start,
  the lines of the streams that the call shows, and the end. The projection
  decides what a person sees. Under `--verbose`, it shows the events, and
  otherwise it discards them. The author of an action sends no text of their
  own there, although the contract does not enforce it.
- **Tool requirement** — What an action declares for each tool that it
  runs. A requirement holds the identifier of the tool in the configuration
  of mise and the program that the action starts. It also holds the exact
  version of a tool, or the range of versions of a toolchain. A requirement
  of an exact version can also list the platforms where the tool has a
  build. The declaration takes no arguments and no context, because the
  machinery reads it before any run starts.
- **Toolchain** — A tool that the code of the project builds or runs with,
  such as Rust, Node.js, or TypeScript. The project pins its toolchains. An
  action pins every other tool that it runs through mise, and it accepts a
  range of versions for each toolchain that it needs.
- **Wrapping** — The execution of an external tool as a subprocess of an
  action. Wrapping is the normal integration path, and Rakko makes it a
  polished one.

## Retired Terms

| Retired                                | Replacement      | Reason                                                                    |
| -------------------------------------- | ---------------- | ------------------------------------------------------------------------- |
| `cargo ra`                             | `mise run rakko` | A cargo alias needs the harness in the workspace, which couples the MSRV. |
| chassis                                | Clawless         | A second name for the framework that Clawless already names.              |
| core crate, framework crate, hub crate | contract crate   | Four names were in use for the contract crate.                            |
| descriptor, metadata object            | erased action    | Three names were in use for the registry view of an action.               |
| host                                   | harness          | "Host" reads as the machine that programs run on.                         |
| machinery crate                        | projection       | The design documents described the crate but never named it.              |
| mode                                   | (none)           | Removed until an action shows a real need for a shared type.              |
| phase                                  | access           | The fix and verify phases derive from access declarations.                |
| recipe                                 | action           | Justfile vocabulary.                                                      |
| repository                             | project          | A project implies a repository, but Rakko does not require one.           |
| set                                    | bundle           | Both terms were in use for the same concept.                              |
| xtask                                  | harness          | The name of the community pattern, not the name of our crate.             |

[adr-026]: adrs/026-harness-api-stability.md
[clawless]: https://github.com/aonyx-ai/clawless
[mise]: https://mise.jdx.dev
[renovate]: https://docs.renovatebot.com
