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
  access. The default declaration is a write. The scheduler, which is still
  planned, derives its order from these declarations.
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
  run. The projection derives one command from each mounted action, and a
  harness writes the others, for a maintenance activity of the project that
  does not fit an action. A derived command is not the action itself. A
  written command is an ordinary Clawless command. It knows nothing about
  actions, unless it is a group, which gives the projection the actions that
  it runs.
- **Context** — The data that is relevant when an action runs. Today this is
  the project root and the directory layout. Later it can include user
  variables and loaded configuration. Clawless uses the same name for a
  similar concept. This parallel is intentional. An action never sees the
  Clawless type, and a command receives both.
- **Contract crate** — The `rakko-action` crate. It contains the `Action`
  trait, the access types, and the other shared types. The contract crate
  stays small, because every action and every harness depends on it. The
  `rakko` crate is a placeholder today, and it becomes a facade over the
  contract later.
- **Erased action** — The internal, object-safe view of an action. Bundles
  export erased actions, and the registry in the harness holds them. Action
  authors do not see this type.
- **Finding** — One problem that an action found in the project, with its
  location. Findings travel in the outcome of an action run.
- **Fleet** — All Aonyx projects that Rakko maintains.
- **Group** — A command that runs a list of actions, its members, and gives
  that list to the projection. The projection derives the flags of the group
  from the arguments of its members, and a flag reaches only the members that
  declare its argument. A group has no arguments of its own.
- **Harness** — The small binary crate in each project. The harness mounts
  the bundles and actions that the project uses, writes the commands of the
  project that do not fit an action, and `mise run rakko` runs it. The
  harness is a package of its own, outside the workspace of the project, and
  it is the full adoption surface of a project.
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
  pinned versions, and Renovate updates the pins in `mise.toml`.
- **`mise run rakko`** — The mise task that builds and runs the harness of a
  project. The task lives in the `mise.toml` file of each project. It is the
  canonical form on every platform, and documentation always writes it out,
  because a shorter form exists only where a platform can supply one.
- **Mount** — To register actions, bundles, or written commands in the CLI
  of a harness. The harness names what it mounts, in code.
- **Outcome** — The result of one action run. An outcome tells whether the
  action passed, changed the project by repairing everything that it found,
  failed with findings, skipped because there was nothing to examine, or
  stopped with an error.
- **Precondition** — (Planned.) A condition about the environment of the
  project that a correct run needs, such as a current file of tool pins. The
  projection checks the preconditions before it runs anything, and a failed
  precondition stops the run. The `doctor` command reports them, and
  `doctor --fix` repairs them.
- **Project** — The unit that Rakko maintains: a directory tree with its own
  tools, actions, and harness. A project is usually a Git repository, but
  Rakko does not require one. A project marks its root with
  `.config/rakko.toml`, and a run searches for that file from the directory
  that it starts in.
- **Projection** — The `rakko-cli` crate. It turns the actions that a
  harness mounts into commands, with the help text and the shared output
  flags, and it defines the traits that a written command and a group
  implement. Only a harness depends on it, so Clawless never reaches an
  action.
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
- **Scheduler** — (Planned.) The component that runs actions in parallel.
  The scheduler computes a conflict graph from the access declarations
  before any action runs.
- **Tool** — An external program that an action uses, for example prettier
  or taplo. Mise provisions tools. In Rakko documentation, "tool" always
  means an external program.
- **Toolchain** — A tool that the code of the project builds or runs with,
  such as Rust, Node.js, or TypeScript. The project pins its toolchains.
  (Planned.) An action pins every other tool that it runs, and it accepts a
  range of versions for each toolchain that it needs.
- **Wrapping** — The execution of an external tool as a subprocess of an
  action. Wrapping is the normal integration path, and Rakko makes it a
  polished one.

## Retired Terms

| Retired                                | Replacement      | Reason                                                                    |
| -------------------------------------- | ---------------- | ------------------------------------------------------------------------- |
| `cargo ra`                             | `mise run rakko` | A cargo alias needs the harness in the workspace, which couples the MSRV. |
| chassis                                | Clawless         | A second name for the framework that Clawless already names.              |
| core crate, framework crate, hub crate | contract crate   | Four names were in use for the `rakko-action` crate.                      |
| descriptor, metadata object            | erased action    | Three names were in use for the registry view of an action.               |
| host                                   | harness          | "Host" reads as the machine that programs run on.                         |
| machinery crate                        | projection       | The design documents described the crate but never named it.              |
| mode                                   | (none)           | Removed until an action shows a real need for a shared type.              |
| phase                                  | access           | The fix and verify phases derive from access declarations.                |
| recipe                                 | action           | Justfile vocabulary.                                                      |
| repository                             | project          | A project implies a repository, but Rakko does not require one.           |
| set                                    | bundle           | Both terms were in use for the same concept.                              |
| xtask                                  | harness          | The name of the community pattern, not the name of our crate.             |

[clawless]: https://github.com/aonyx-ai/clawless
[mise]: https://mise.jdx.dev
[renovate]: https://docs.renovatebot.com
