# ADR-011: Tool Integration

## Status

Accepted

## Context

An action does the work of an external tool. The [vision] gives each layer
its job: mise provisions the tools at pinned versions ([ADR-003]), and
Rakko runs them against the project. Between those two sits the question
that this ADR settles: how an action executes its tool, and who decides
what the tool does when it runs.

The first action forced both halves at once, and it started on the wrong
foot. The design audit picked taplo first because it is the one tool with
a real Rust library API, and the plan was to link it as a library and keep
subprocesses out of the MVP entirely. The library turned out to carry the
engine of the tool and not its behavior. Formatting is in the library; the
loading of the configuration, the discovery of the files, and the defaults
that apply when a project states nothing live in the tool's command line.
The pinned command line embeds the same engine crates that an action would
link, so the engine cannot disagree — everything that can disagree is glue
that the action would reimplement, and that copy versions with the action
instead of the tool, so its drift is invisible.

The behavior matters more than the engine, because Rakko is not the only
thing that runs these tools. An editor formats on save, a language server
checks while the contributor types, and a contributor runs a tool bare to
see what it does. Every one of them reads the tool's own configuration
file: taplo reads `.taplo.toml`, prettier reads `.prettierrc.json`,
markdownlint and yamllint read a dotfile each, and zizmor reads
`zizmor.yml`. Whatever Rakko does instead of that is a second source of
truth, and two sources of truth disagree quietly about the same file.

The fleet also leaves no room to avoid subprocesses. Most tools of the
justfile publish no stable library API — zizmor states so explicitly — and
the tools that are not Rust never had one. Wrapping arrives with the
second action whatever the first one does, so the real question is which
path gets the polish.

## Decision

Wrapping is the normal path, and the behavior of the tool is
authoritative.

1. **An action wraps its tool as a subprocess.** The binary that the
   editor and CI run does the work, and the action turns what the tool
   reported into an outcome. The machinery of a subprocess — resolving the
   binary, starting it, collecting what it produced — is polished once and
   shared, so an action states what to run. A run starts the tool in the
   project root, so it behaves the same from every directory of the tree
   ([ADR-010]).

2. **The tool that runs is the tool that mise pinned.** Resolution goes
   through the provisioning layer, so the version in `mise.toml` answers,
   no matter which shell or environment started the harness. A tool that
   is absent stops the action with an error that names it: provisioning
   stays the job of mise, and Rakko installs nothing.

3. **No shell stands between the action and the tool.** The action starts
   the process with an argument vector, and nothing splits, quotes, or
   expands on the way, so an argument arrives as the action wrote it, on
   every platform.

4. **The configuration of the tool is the source of truth.** An action
   selects the operation of the tool — a check, a fix — and changes none
   of its options: no override flags, no keys added, and none ignored. A
   project that never configured the tool gets the defaults of the tool,
   and a configuration that the tool rejects stops the action, because a
   run that falls back to the defaults quietly does what the project asked
   it not to do. A subprocess grants all of this by construction, because
   the tool loads its own configuration on the way in.

5. **Linking stays an optimization behind a bar.** An action may consume
   its tool as a library only when the library carries the behavior of the
   tool, and not merely its engine: the configuration reading, the file
   discovery, and the defaults. A library that hands the action an
   operation and leaves the behavior to be rebuilt does not qualify,
   because the rebuilt copy does not version with the tool.

6. **The action prefers the structured output of its tool.** A tool that
   can report as data is asked to, and text is parsed only where nothing
   better exists. Either way the action translates what the tool reported
   into findings and renders nothing ([ADR-008]).

The decision stops at the mechanics. The shape of the subprocess machinery
and the crate that holds it, and how each action reads the output of its
tool, are implementation choices. Whether an action can ship a default
configuration when a project has none, or merge fleet-wide options into
one that exists, is a later decision that would supersede part of this
one. Whether the discovery of the files that a run touches ever moves out
of the actions and into the machinery stays open with the access
declarations.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Linking as the Normal Path

An action can consume its tool as a Rust library: no subprocess, no
resolution, a typed API, and an error that arrives as a value instead of a
stream to parse. This was the plan for the first action.

Most tools rule it out on their own: the majority publish no stable
library API, and the tools that are not Rust never had one, so wrapping
must exist and must be polished anyway. The first action then showed that
even a real library clears less than it seems. The taplo crates carry the
formatter, while the behavior around it lives in the command line, so a
linking action rebuilds the behavior, and the copy drifts invisibly
because it versions with the action. A library saves a process start and
costs the one property this decision protects: that the tool behaves the
same with Rakko as without it.

### A Shell Between the Action and the Tool

The justfile runs every tool through a shell today, and handing a command
line to `sh -c` is the shortest port of a recipe. The shell is where the
quoting bugs live: an argument with a space splits, a stray glob expands,
and what `$PATH` resolves depends on initialization files that differ per
contributor. A native Windows shell does not run the same line at all,
which [ADR-009] already refused to accept for the entry point. An argument
vector does the one thing a recipe needs and nothing that it must not.

### A Rakko Configuration per Tool

Rakko can define a configuration of its own for each tool and translate it
into the options of that tool. The fleet would get one configuration
style, one file location, and keys that Rakko documents itself.

This is the second source of truth that the context warns about. The
editor and the bare tool read the tool's file, Rakko reads its own, and
the two drift apart without anyone noticing, because each looks correct to
its reader. The translation also trails the tool permanently: every option
that the tool adds is missing from Rakko until someone maps it, and every
mapping is code that can be wrong.

### Fixed Defaults in the Action

An action can carry the options of the fleet in its code and read no file
at all. This is the strongest form of uniformity: policy lives in the
action, versioned, and Renovate rolls a policy change across the fleet as
a release.

The tool still runs outside Rakko, and there it reads its file, so a
project would be formatted one way by the editor and another way by the
action, on the same afternoon. It also inverts adoption: mounting an
action would change how a project is formatted, while an action that
honors the file changes nothing on arrival. Uniform options are a real
goal, but they belong to the open question of shipping and merging
configuration, where the tool can see the same file.

## Consequences

- Adopting Rakko changes nothing about how a tool behaves. A project keeps
  its configuration files, the check agrees with the editor, and a
  contributor debugs an action by running its tool bare. One file, one
  truth, one mistake instead of two.
- The fleet stays non-uniform for now. Each project's configuration can
  say something different, and an action treats each project the way that
  project asks. Fleet-wide policy waits for the open decision on shipping
  and merging configuration.
- The translation that [ADR-008] named becomes the common case: every
  action parses what its tool reported. The pin softens the cost — output
  changes with a version, and a version changes with a Renovate pull
  request, so a broken parse arrives as a red pull request instead of a
  quiet drift.
- A test needs the tool. A wrapped action tests against the real binary,
  so its tests run where the tools are provisioned, and they are heavier
  than the tests of a linked library. In exchange they exercise the
  behavior that a project observes.
- Every run pays a process start. The cost is small next to the work of
  the tool, and a run that waits on a child yields ([ADR-005]), so
  parallel runs lose nothing.
- The bar on linking means that few tools will ever clear it, and that is
  the point. A library that grows into the behavior of its tool can be
  adopted per action later, and the swap must be invisible to a project,
  which is what the bar protects.
- Applicability stays the judgment of the action. The discovery of the
  tool answers only when the tool runs, so an action decides whether it
  applies with a cheaper look of its own, and the two can disagree at the
  margins. The skip message says what the action looked for, so a reader
  can question it.
- The agreement between an action and an editor holds per version. Mise
  pins what the project runs; an editor plugin can bundle a version of its
  own, and that skew is outside the reach of Rakko.

[adr-003]: 003-mise.md
[adr-005]: 005-asynchronous-actions.md
[adr-008]: 008-renderers.md
[adr-009]: 009-harness-entry-point.md
[adr-010]: 010-project-root.md
[vision]: ../VISION.md
