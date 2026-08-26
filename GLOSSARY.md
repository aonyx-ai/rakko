# Glossary

This glossary defines the terms of the Rakko design. One term has one
meaning, and one concept has one term. The Retired Terms section maps words
from the design discussions to the terms that replaced them.

## Terms

- **Access** — The declaration of the files that an action reads and writes.
  The declaration is a function of the action's arguments, so a `--fix` flag
  changes it. The default declaration is exclusive access to the full
  project. The scheduler and the sandbox derive their behavior from these
  declarations.
- **Action** — The unit of maintenance work. An action is a library crate
  that implements the `Action` trait. An action depends on the contract
  crate, and on nothing else from Rakko.
- **Applicability** — Whether an action applies to a project. Each action
  detects its own applicability, and it skips with a visible message when it
  does not apply. This behavior keeps broad bundles safe.
- **Bundle** — A meta-crate that exports a list of actions. A bundle defines
  what a word such as "recommended" means, as a dependency. Bundles can
  contain other bundles. A bundle release starts a rollout.
- **`cargo ra`** — The cargo alias that builds and runs the harness of a
  project. The alias lives in the `.cargo/config.toml` file of each project.
- **Check** — An informal word for an action that examines the project and
  reports findings. A check is not a formal category: the system has one
  `Action` trait and no subtypes.
- **Clawless** — Aonyx's framework for command-line applications, and the
  chassis of Rakko. [Clawless] turns mounted actions into commands, with
  help text and shared output flags. Actions do not depend on Clawless.
- **Command** — The CLI projection of an action. Clawless generates one
  command for each mounted action. The command is not the action itself.
- **Context** — The data that is relevant when an action runs. Today this is
  the project root and the directory layout. Later it can include user
  variables and loaded configuration. Clawless uses the same name for a
  similar concept. This parallel is intentional, and consumers of Rakko do
  not see the Clawless type.
- **Contract crate** — The `rakko` crate. It contains the `Action` trait,
  the access types, and the other shared types. The contract crate stays
  small, because every action and every harness depends on it.
- **Erased action** — The internal, object-safe view of an action. Bundles
  export erased actions, and the registry in the harness holds them. Action
  authors do not see this type.
- **Finding** — One problem that an action found in the project, with its
  location. Findings travel in the outcome of an action run.
- **Fleet** — All Aonyx projects that Rakko maintains.
- **Harness** — The small binary crate in each project. The harness mounts
  the bundles and actions that the project uses, and `cargo ra` runs it. The
  harness is the full adoption surface of a project.
- **Linking** — The use of an external tool as a Rust library inside an
  action. Linking is an optimization for one action at a time, taken when
  the tool has a real library API.
- **mise** — The provisioning layer. [Mise] installs the external tools at
  pinned versions, and Renovate updates the pins in `mise.toml`.
- **Mount** — To register actions or bundles in the CLI of a harness. The
  harness names what it mounts, in code.
- **Outcome** — The result of one action run. An outcome tells whether the
  action passed, failed with findings, skipped as not applicable, or stopped
  with an error.
- **Project** — The unit that Rakko maintains: a directory tree with its own
  tools, actions, and harness. A project is usually a Git repository, but
  Rakko does not require one.
- **Rakko** — This project: a toolkit that turns project maintenance into
  versioned Rust crates. The rakko (ラッコ) is the Japanese sea otter, the
  otter that keeps a pebble as its tool.
- **Renovate** — The update and rollout mechanism. [Renovate] opens pull
  requests for new action versions in `Cargo.toml`, and for new tool
  versions in `mise.toml`.
- **Rollout** — The propagation of a bundle release across the fleet. Each
  project receives a Renovate pull request, and the CI result of that pull
  request shows whether the project complies.
- **Sandbox** — (Planned.) Enforcement of access declarations when actions
  run. A sandbox turns a false declaration into a visible error instead of a
  race condition.
- **Scheduler** — (Planned.) The component that runs actions in parallel.
  The scheduler computes a conflict graph from the access declarations
  before any action runs.
- **Tool** — An external program that an action uses, for example prettier
  or tracey. Mise provisions tools. In Rakko documentation, "tool" always
  means an external program.
- **Wrapping** — The execution of an external tool as a subprocess of an
  action. Wrapping is the normal integration path, and Rakko makes it a
  polished one.

## Retired Terms

| Retired                                | Replacement    | Reason                                                          |
| -------------------------------------- | -------------- | --------------------------------------------------------------- |
| core crate, framework crate, hub crate | contract crate | Four names were in use for the `rakko` crate.                   |
| descriptor, metadata object            | erased action  | Three names were in use for the registry view of an action.     |
| host                                   | harness        | "Host" reads as the machine that programs run on.               |
| mode                                   | (none)         | Removed until an action shows a real need for a shared type.    |
| phase                                  | access         | The fix and verify phases derive from access declarations.      |
| recipe                                 | action         | Justfile vocabulary.                                            |
| repository                             | project        | A project implies a repository, but Rakko does not require one. |
| set                                    | bundle         | Both terms were in use for the same concept.                    |
| xtask                                  | harness        | The name of the community pattern, not the name of our crate.   |

[clawless]: https://github.com/aonyx-ai/clawless
[mise]: https://mise.jdx.dev
[renovate]: https://docs.renovatebot.com
