# ADR-030: Tool Output

## Status

Accepted

## Context

An action runs one or more tools, such as cargo or yamllint, as child
processes. It reads what each tool writes, and it turns that output into an
outcome. The renderer then shows the outcome when the run is over
([ADR-008]). The rest of the output of each tool is lost. An action keeps a
line of a tool only in a few known cases, such as when yamllint refuses the
configuration of the project.

At a terminal, a run shows the verdict and not the output of each tool. But
a developer who investigates a failure needs the output too. In CI, the log
is the only record of a job. When a step fails, the log shows the verdict,
but it can omit the output that explains it. When a step is long, the log
shows nothing until the step ends. A slow step and a step that hangs then
look the same.

The action `bench` of stiles runs for more than 15 minutes for each project.
Its first run on a runner of CodSpeed, a service for benchmarks, was silent
for an hour, until the runner stopped.

The parts that show the output of a tool while it runs already exist.
`kawauso-process` can start a child and give each line that the child writes
as it arrives, together with its stream, stdout or stderr. It also keeps the
capture, which is every byte of both streams. So streaming does not change
the result that the caller gets at the end. Clawless builds its process
wrapper on this. Its terminal presenter, the part of Clawless that writes to
the terminal, shows the lines of a child under `--verbose`.

What does not exist is a path from the action to Clawless. An action
receives a context that holds only the root of the project, and it starts
its tools itself. Point 2 of ADR-008 keeps an action away from every stream.
[ADR-004] keeps Clawless out of the dependency tree of every action. [ADR-014]
and [ADR-024] turned down an output handle in the contract crate that
mirrors the events of Clawless. ADR-008 left open whether an action ever gets
a channel to report progress while it runs.

Point 2 of ADR-024 runs the readers of a group at the same time, and its
writers one at a time. A reader is a member that examines the project
without a change. A writer is a member that can change the project. If every
member streams its output, the lines of parallel readers can mix. So the
projection needs to associate the output with the member that produced it.

Inside one action, the lines cannot mix today. On 2026-11-13, no action in
this repository ran two tools at the same time.

## Decision

A run can show the output of the tools that an action runs. The action
passes what each of its tools writes to the context of its run. The
projection decides what a person sees.

1. The output of a tool belongs to the run. The context of a run carries a
   sink, which receives the output of its tools. Each line of a stream that
   the call shows goes to the sink when `kawauso-process` reads it. The line
   carries its stream, stdout or stderr. The sink keeps the order in which
   `kawauso-process` reads the lines. It cannot recover the exact order of
   writes across the two streams.

   The action still turns the captured output into its outcome as it does
   today. The forwarded lines never enter the outcome or the JSON documents,
   as point 5 of ADR-024 decides for progress. An action can still include a
   diagnostic from its tool in its outcome.

2. The sink carries what the tools write and nothing else. For each tool, it
   carries the start with the command, each line of a shown stream, and the
   end with the exit code. An action can run several tools, or one tool
   several times. For example, lint-yaml runs `yamllint --list-files` and
   then `yamllint`. The command and the exit code show which lines belong to
   which run, and how each run ended.

   The sink carries no message, no detail, and no text that the action
   composes. This is a rule for the author of an action. The contract does
   not enforce it, because the machinery that starts a tool sends to the
   sink through the same interface. The sink uses no type of Clawless, and it
   does not mirror the events that ADR-014 and ADR-024 turned down.

   The projection supplies the sink for each action run. Each member of a
   group gets its own sink, so parallel members never share an unidentified
   buffer. A context that a library caller builds has a sink that discards
   everything by default. The action learns nothing about the person who
   reads the log, and it still renders nothing, as point 2 of ADR-008
   requires.

3. The author of an action writes no loop. The machinery that starts a tool
   passes its start, its lines, and its end to the sink. It gives back the
   same result as a run that only captures. An action changes the call that
   starts its tools once, and its parsing stays the same.

   The output path does not change the arguments, the environment, or the
   working directory of the tool. The machinery continues to read both
   streams to their end when the sink discards their lines.

   An action often asks its tool for a report in a machine format, such as
   JSON, because it parses that report. A person cannot read it, and the
   outcome already shows what it holds. So the call that starts a tool says
   which streams reach the sink. Cargo writes its JSON to stdout and the
   progress of a build to stderr. So a call that asks cargo for JSON shows
   only stderr. A call whose tool writes text to both streams, such as
   taplo, shows both.

4. A flag asks for the lines, and the environment does not. Point 1 of
   ADR-008 sends the question of how much a person sees to the flags. The
   lines of the tools are more of the run, so they appear under `--verbose`,
   and only then. The run does not read the environment for this choice. A CI
   job that wants the lines passes the flag.

   Without `--verbose`, the projection neither presents nor buffers the
   forwarded lines. `--quiet` and `--verbose` remain mutually exclusive.

   Under `--verbose --json`, both source streams go to stderr, and the
   document on stdout does not change. The same rule applies to group
   headers. The projection presents tool output as diagnostic text, with no
   findings or annotations derived from those lines.

5. A single action streams, and each reader of a group waits. Under
   `--verbose`, a single action and each writer of a group show each line
   when `kawauso-process` reads it. The projection buffers the lines of each
   reader of a group.

   When the member finishes, its lines appear just before its report, under
   a header that gives the member. This also applies when the member returns
   findings or stops with an error. Each block of lines stays next to the
   result that it explains, in the order in which the members finish.

   The projection finishes each block before it presents the next one, so
   the lines of two members never mix. This rule applies at a terminal and in
   CI, including under `--json`.

Buffering favors a readable block over live output for a reader of a group.
If a CI job needs live output from a long action, it runs that action
directly with `--verbose`. A reader that hangs inside a group shows no tool
output until it finishes. If the harness or its runner stops first, the
buffered lines are lost. This decision does not solve that case.

Live output also depends on the tool. A tool that holds its output in a
buffer of its own sends a line only when it writes the line out. An
incomplete line can wait until the stream closes.

This answers the question that ADR-008 left open for the output of tools
only. Whether an action can report progress of its own stays open. An
example is the count of files that it checked so far. ADR-008 stays
Accepted, and this ADR supersedes no part of it.

The decision records the direction and stops there. The names and types of
the sink, the crate that holds the machinery of point 3, and the text of the
header are implementation choices. The storage and limits of the buffers
are implementation choices too. If a limit omits lines, the output must say
that it omitted them.

An action that runs two tools at the same time mixes their lines in its
sink. How the projection keeps them apart stays open until an action needs
it. The decision says nothing about a live view of the progress of a group,
which ADR-024 defers.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Capture at the End

The run can show the whole capture of each tool when the action finishes.
Nothing streams, so nothing can mix, and the lines need no path while the
tool runs.

But the capture still has to leave the action, so this needs the same path
as the decision. A step that hangs, or that the job stops at its timeout,
never finishes, and it shows nothing. `kawauso-process` already gives the
lines while it captures them, so streaming needs no second run of the tool.
Forwarding and presenting the lines still add work.

### The Process Wrapper of Clawless in the Action

The context of an action can carry the process wrapper of Clawless. The
action then starts its tools through Clawless, and Clawless shows the lines.

This puts Clawless into the dependency tree of every action, which ADR-004
rules out. It also gives the action a stream of Clawless, which point 2 of
ADR-008 rules out.

### A Hook in kawauso-process

`kawauso-process` can send every line of every run to an observer that the
projection installs once. No action then has to change.

This is hidden global state in a crate that knows nothing about Rakko. The
readers of a group run at the same time, so the observer needs an association
between each tool and its member. A global hook alone does not provide that
association.

### Tool Output in the Outcome

Each outcome can carry the capture of each tool that the action ran. The
output is then a value that a test can assert on, and the renderer shows it.

The outcome gets a field for diagnostic output, and the lines arrive only
when the action finishes, as in the first alternative. The contract crate
then carries presentation data in the outcome. ADR-008 gives the JSON schema
to its renderer, so this field also needs a rule about its representation
there.

### CI Detection From the Environment

The run can show the lines when the variable `CI` is set, with no flag. Every
project in the fleet then gets the lines in CI at once.

But the same command then gives other output on a laptop than in CI, and
the person who reads the log cannot see why. Many systems set `CI`, and some
tools and test setups set it locally to turn off prompts. A wrong guess then
puts the lines of every tool on a terminal where nobody asked for them. This
alternative also needs a rule of precedence against `--quiet` and
`--verbose`. Point 4 of ADR-008 lets the environment select the renderer,
but how much a person sees is a flag under point 1.

### A Prefix on Each Line

The run can stream the lines of every reader of a group at once. Each line
can carry the name of its member as a prefix.

This gives live output for parallel readers, including in CI. But the lines
of two readers still mix, and a person has to sort them. A prefix can also
interfere with paths that a terminal or an editor recognizes. We accept the
delay and possible loss of a buffered block to keep the output of each
member together. A job that needs live output runs the action directly.

### One Block for the Whole Group

The projection can hold the lines of every member, and show them all when
the group finishes.

A slow member, such as `bench`, then holds back the lines of every fast
member. The blocks also lose their place next to the reports, which ADR-024
sends when each member finishes.

## Consequences

- With `--verbose`, a single action or a writer of a group shows tool output
  while it runs. This helps a person investigate a slow or failed step when
  its tools produce lines.
- `kawauso-process` and Clawless need no change.
- The contract crate gets the sink, so the addition reaches every action and
  harness, as ADR-004 describes. Each action that starts tools adopts the
  forwarding call. An action that keeps the old call stays silent, so the
  migration needs review.
- A CI workflow that needs tool output passes `--verbose`. A workflow that
  forgets the flag shows only the existing reports. The fleet can set the
  flag in a reusable workflow or a shared task that CI runs.
- `--verbose` also shows the details of Clawless, which are the extra
  messages that a command sends for that flag. In CI, the log gets these
  details too.
- A reader of a group shows its tool output only when it finishes, in CI as
  well as at a terminal. A timeout or a runner that stops can lose its whole
  block. A later view of group progress alone does not recover those lines.
- Under `--verbose`, buffering adds storage for every active reader of a
  group, alongside the capture that `kawauso-process` keeps. An unlimited
  buffer in memory can hold the output twice. A storage limit must disclose
  omitted lines, and it must leave the capture that the action parses intact.
- Existing library calls stay silent through the default sink. Callers can
  supply a sink without depending on Clawless or on a renderer.
- A developer can copy the command from a CI log to request tool output on a
  laptop, because the flag is explicit.

[adr-004]: 004-actions-as-libraries.md
[adr-008]: 008-renderers.md
[adr-014]: 014-commands.md
[adr-024]: 024-group-runs.md
