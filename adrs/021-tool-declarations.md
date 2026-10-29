# ADR-021: Tool Declarations

## Status

Accepted

## Context

An action wraps an external tool and reads what the tool reports
([ADR-011]). So what an action does depends on the version of its tool.
Today the project pins that version. Point 1 of [ADR-003] lets one file, the
`mise.toml` of the project, pin every tool. Point 2 of ADR-011 resolves the
tool at the version that this file pins.

The action and the version of its tool therefore change apart. An action
cannot know which version a project runs, so it must accept every version
that a project can pin. Its tests cover only the version that this
repository pins. When markdownlint-cli 0.50.0 arrived, the action had to
keep the behavior that 0.49 needed, because other projects still pinned
0.49.

The pins also repeat across the fleet. All 13 projects with a harness pin
the same 14 tools, at the same versions except Node.js and Renovate. Nine of
these tools are tools of actions. A project outside Aonyx that adopts Rakko
has to copy these pins and keep them current.

Not every tool can move to the action. A project builds or runs its own code
with some of them, such as the Rust toolchains and Node.js. Their versions
are decisions of the project. Mise also replaces a tool of one file as a
whole. A `rust` entry in the `mise.toml` of a project replaces every `rust`
entry of another file. So a second file cannot add a nightly next to the
toolchains of the project.

Mise and Renovate supply the parts of a design. Mise merges the files in
`.config/mise/conf.d` with the configuration of the project. It loads a file
such as `mise.rakko.toml` only when the environment `rakko` is active. The
mise manager of Renovate updates the pins in both kinds of file. Mise can
also resolve an exact version of a tool, whatever version the project pins.

## Decision

An action pins the exact version of each tool that it wraps, and the project
pins its toolchains. Rakko writes the pins of the actions into a mise file
that the project commits.

1. **A toolchain stays with the project.** A toolchain is a compiler or
   runtime that the code of the project builds or runs with, such as Rust,
   Node.js, or TypeScript. The project pins it, and an action states the
   range of versions that it accepts. Every Rust toolchain is a toolchain,
   the nightly included, because mise replaces a `rust` entry as a whole.

2. **An action declares its tools.** An action declares each tool that it
   runs, as it declares its access ([ADR-017]). A tool gets an exact
   version, and a toolchain gets the range that the action accepts. The
   declaration depends on neither the arguments nor the context of a run,
   because the project needs the pins before any run starts.

3. **The pin is a mise file in the crate.** The crate that wraps a tool
   keeps the pin in a mise file that mise ignores by default. The mise
   manager of Renovate updates it like any other pin. When several actions
   share a helper crate for a tool, the helper crate keeps the pin. The
   tests of an action run the tool at the version of this file.

4. **Rakko writes the pins into the project.** From the declarations of the
   actions that a harness mounts, Rakko generates a mise file and its
   lockfile, and the project commits both. The project does not edit them.
   Mise merges the generated file with the configuration of the project. So
   an install in CI, the editor, and a bare run of the tool get the version
   that the action declares. The `mise.toml` of the project keeps its
   toolchains and the tools that no action declares.

5. **A pin of the project does not replace a declared one.** Mise lets a pin
   in the configuration of a project override the same tool in the generated
   file. Rakko reports such a pin as an error, and the repair removes the
   pin. A project that must stay on an older version of a tool stays on an
   older revision of Rakko.

6. **A run uses the declared version.** An action resolves its tool through
   mise at the exact version that it declares, and not through the pin of
   the project. If that version is not installed, the run stops with an
   error that gives the tool and the version. An action resolves a toolchain
   at the version that the project pins, and it stops when that version is
   outside its range.

This supersedes point 1 of [ADR-003], "One file pins every tool". The rest
of ADR-003 stands: mise provisions every tool, it pins the Rust toolchains,
and its backends install the other tools. This also supersedes the sentence
of point 2 of [ADR-011] that the version in `mise.toml` answers. Resolution
still goes through mise, and Rakko still installs nothing.

The decision records the direction and stops there. The shape of the
declaration, the names and places of the files, and how an action asks mise
for a version are implementation choices. How Rakko keeps the generated file
current, and how an update of Rakko changes the file in each project, are
decisions of their own. A setting that lets a project run another version of
a declared tool is not part of this decision. It is an addition, and it can
come later.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Project Pins Every Tool

This is the choice of ADR-003. A project keeps the control of every version,
and each pin has one place. But the action must accept every version that a
project pins, and its tests cover one of them. The update to
markdownlint-cli 0.50.0 showed the cost. Every project that adopts Rakko
also copies the same pins.

### A Range for Every Tool

An action can state the range of versions of its tool that it accepts, as it
does for a toolchain. The project then pins a version in that range. The
project keeps its control, and Rakko can report a pin outside the range. But
the action still has to work with every version in the range, and its tests
still cover one. The project also still copies the pins.

### Resolution Without a File

Rakko can ask mise for the declared version at each run, and install the
version when it is missing, with no file in the project. In this design,
Rakko installs tools, which point 2 of ADR-011 rules out. The editor and a
bare run use another version or none, and an install in CI does not see the
version.

### Pins in the Configuration of the Project

Rakko can write the declared pins into the `mise.toml` of the project, so
that one file holds every pin, as ADR-003 wanted. But the file then has two
owners, the project and Rakko. Renovate updates the lines of Rakko apart
from the action, and each write of Rakko has to merge with the edits of the
project.

### A Pin of the Project as an Escape Hatch

Rakko can let a pin of the project win, as mise does by default, and run the
version that the project pins. A project can then hold a tool back, or take
a newer version, without a new revision of Rakko. But the action then runs a
version that its tests never covered, which is the problem that this
decision removes. If the pin wins only for the editor, the editor and the
run disagree. The revision of Rakko is the escape hatch instead.

### The Version in Rust Code

The crate can keep its pin as a constant in Rust, or in the metadata of its
manifest. The pin is then part of the code, with no file to embed. But
Renovate needs a custom manager for each such place. A mise file is the
format that the mise manager of Renovate already reads.

## Consequences

- An action and its tool change together. An update of a tool arrives as a
  pull request to the crate that wraps it. The tests of the action run
  against the new version in that pull request.
- A project copies no pins for the tools of its actions. Its `mise.toml`
  keeps its toolchains and its own tools.
- A project cannot run a declared tool at a version other than the one that
  its revision of Rakko declares. To keep an older tool, it keeps an older
  revision.
- A project holds two configuration files of mise and two lockfiles. Rakko
  generates one of each, and Rakko replaces an edit by hand at the next
  write.
- Each update of Rakko that changes a tool also changes the generated file
  in each project. The hosted app of Renovate runs no custom command, so the
  way that the update changes the file is still open.
- The change breaks the API of every action that wraps a tool, because the
  action resolves its tool through its declaration. The declaration itself
  is an addition to the contract, with a default of no tools. The break
  lands before the public launch.
- A toolchain stays a decision of the project. An action that needs a newer
  toolchain stops with an error until the project moves its pin.
- This repository keeps the tests of every action, also of the actions that
  its harness does not mount. So its harness mounts every action, and the
  generated file installs every tool that a test needs. Its own `mise.toml`
  keeps only the toolchains and the tools of the repository.
- In this repository, the mise file of a crate and the generated file hold
  the same pin. Renovate updates a dependency in every file in one branch,
  so both move in one pull request.

[adr-003]: 003-mise.md
[adr-011]: 011-tool-integration.md
[adr-017]: 017-access-declarations.md
