# ADR-018: File Access

## Status

Accepted

## Context

[ADR-017] lets an action declare whether a run reads or writes the project.
In that first version, a declaration covers the whole project. So an action
that writes runs alone, even when no other action reads or writes the files
that it changes. The formatter for Markdown and the formatter for YAML wait
for each other, although one changes only Markdown files and the other only
YAML files. The [vision] expects a scheduler that runs two such formatters at
the same time, because their access cannot overlap. For that, a declaration
has to cover less than the whole project.

ADR-017 left the files that a declaration covers as a question of its own.
[ADR-011] left a related question open: whether the discovery of the files
that a run touches moves out of the actions and into the machinery. A
declaration of files seems to ask for that move. If the machinery knows the
files of a run, it can hand them to the tool as a list, and the tool then
examines exactly the files that the action declared.

The tools do not take a list without a change in what they examine. Every
wrapped tool that selects files by pattern accepts an explicit list of files.
Each tool applies its own ignore rules to a file in that list, such as
`.prettierignore`, `.markdownlintignore`, the `include` and `exclude` keys of
taplo, the `ignore` key of yamllint, and the `ignorePatterns` of oxlint and
oxfmt. The ignore rules of git are different. On 2026-10-21, with prettier
3.9.10, oxfmt 0.74.0, oxlint 1.83.0, zizmor 1.31.1, yamllint 1.38.0,
markdownlint-cli 0.49.1, and taplo 0.10.0, the tools applied them as follows:

| Tool             | Root `.gitignore`        | Nested `.gitignore` and `.git/info/exclude` | A listed file that git ignores       |
| ---------------- | ------------------------ | ------------------------------------------- | ------------------------------------ |
| prettier         | own walk and listed file | never                                       | skipped                              |
| oxfmt            | own walk                 | own walk                                    | processed, and `--write` rewrites it |
| oxlint           | own walk                 | own walk                                    | linted                               |
| zizmor           | own walk                 | own walk                                    | audited                              |
| yamllint         | never                    | never                                       | linted                               |
| markdownlint-cli | never                    | never                                       | linted                               |
| taplo            | never                    | never                                       | checked                              |

No tool applies all the ignore rules of git to a file in a list. Three tools
examine a listed file that their own walk skips. A list from Rakko would
change what these tools examine, unless Rakko applied the ignore rules of
each tool itself. A list would also have to copy the default selection of
each tool, such as the collection of zizmor or the fixed file names of the
Renovate validator, and that copy would then decide what the tool examines.
[ADR-013] removed such a copy from the actions, because a copy that disagrees
with the tool hides a real check.

A declaration of files also meets point 2 of ADR-017. When the answer depends
on the configuration of the project, the action declares the cautious answer.
Some tools let a configuration widen their selection, such as a taplo
`include` that adds `Pipfile`, or the `yaml-files` key of yamllint. For files,
the cautious answer is then the whole project again, for every action that
wraps such a tool. No configuration in the fleet widens a selection today:
every `.taplo.toml` uses only `exclude`, and no yamllint configuration sets
`yaml-files`.

A declaration of files changes the contract crate, which every action and
every harness depends on ([ADR-004]). A change of this size needs a decision
on record before the code.

## Decision

An action declares the files that a run reads and the files that it writes
as glob patterns. The patterns serve only the scheduling of runs. Neither the
action nor its tool receives them, nothing enforces them, and each tool keeps
its own discovery.

1. **Each tool keeps its own discovery.** Rakko hands no file list to a tool.
   The tool selects its files as it does today, from its own configuration
   and from any pattern that the action passes to it, as [ADR-011] and
   [ADR-013] require. The patterns of a declaration do not change what the
   tool examines.

2. **A declaration holds glob patterns relative to the project root.** One
   set of patterns covers the files that a run reads, and one set covers the
   files that it writes. The declaration still sees only the bound arguments
   and the context, and it does no I/O. The whole project stays the default,
   and an action can still declare it.

3. **An action declares the default selection of its tool.** The patterns
   cover the files that the tool selects when the project configures no
   selection, such as `**/*.toml` for taplo. A configuration that widens the
   selection is outside the guarantee. Rakko trusts the declaration, because
   it cannot foresee every configuration. It gives sensible defaults and
   strong machinery, and it does not protect a project from each
   configuration that the project writes.

4. **The configuration files of a tool count as reads.** A declaration covers
   the files that the tool loads as its configuration, such as
   `.prettierrc.json`, `.prettierignore`, `.editorconfig`, `.taplo.toml`, and
   `.yamllint.yaml`, and not only the files that the tool examines. A
   declaration covers the action and its tool, as point 4 of [ADR-017] says,
   so it also covers the files that the action reads to resolve its tool,
   such as `mise.toml`. The formatter for JSON writes `.prettierrc.json`,
   which the other actions that wrap prettier read, and the formatter for TOML
   formats `.taplo.toml` and `mise.toml`. Without this rule, two declarations
   that do not overlap can still race on a configuration file.

5. **Only the actions whose tool examines just the files that it selects
   narrow.** Such a tool selects its files by pattern, or by fixed file names
   as the Renovate validator does, and it follows no reference from one file
   to another. Examples are the actions that wrap taplo, prettier, yamllint,
   or the Renovate validator. The other actions keep the whole project, such
   as the compilers, the test runners, the formatter for Rust, and the check
   of the mise lockfile. Their tools follow references from one file to
   another, through imports, `include_str!`, build scripts, module
   declarations, or the files that a mise configuration points to, so they
   can read any file of the project. When they only read, they share the
   project with the other readers, so the cost is less parallelism with the
   writers. A narrower declaration for one of them is work for that action.

6. **The machinery expands the patterns into the files that exist.** When
   Rakko runs, the machinery turns the patterns of a bound action into a list
   of the files that the run reads and a list of the files that it writes.
   The lists come from the files that are in the project at that time, so a
   consumer of the lists compares sets of files, and it never has to prove
   that two patterns cannot match the same file. The lists serve only the
   scheduling of runs. Neither the action nor its tool receives them.

7. **The walk applies no git filter.** The expansion sees the project as it
   is, with `.git/` and with the files that git ignores. Rakko does not
   decide what a project ignores. taplo, markdownlint, and yamllint never
   apply the ignore rules of git, so they read and write files that git
   ignores, and lists without those files would hide that access from the
   scheduling. If a filter is ever necessary, it is opt-in.

8. **Nothing enforces a declaration.** A run sees the whole project,
   whatever its declaration covers. When Rakko runs an action, it gives the
   action no view that holds only the declared files, and it does not check
   what the run read or wrote. A declaration changes only when a run can
   start, and not what the run can see or change. Enforcement, such as by a
   sandbox, needs a decision of its own.

This answers the question that [ADR-011] left open. The discovery of the
files that a run touches stays with the tool. The expansion of the machinery
is not a second discovery for the run, because the tool never sees it. It
serves only the scheduling. The decision keeps
[ADR-013]: the declaration still examines nothing, and the expansion does not
decide whether an action applies. The patterns copy the default selection of
the tool too, but the copy decides only which runs can share the project, and
never what the tool examines. A copy that drifts from the tool costs
parallelism or lets two runs race, and it hides no check.

The decision extends [ADR-017]. Point 3 of ADR-017 limits a declaration to
the whole project in its first version, and this decision allows patterns
narrower than that. It also supersedes point 2 of ADR-017 for the files that
a declaration covers: there, the default selection of the tool replaces the
cautious answer. The cautious answer still decides whether a run writes.
ADR-017 stays Accepted and unchanged, and the rest of it stands, such as the
writes that do not count and the directories that a tool locks itself.

The decision records the direction and stops there. The types of the
declaration, the names of its methods, the syntax of the glob patterns, and
how the machinery walks the project within point 7 are implementation
choices. Three questions stay open for the implementation: the cost of the
walk in a large tree, whether the walk enters a nested project, and how a
pattern covers a file that does not exist yet, such as a file that a run
creates. Their answers keep points 3 and 6 true. The decision says nothing
about how the scheduler works, a sandbox, an ignore for the whole project,
or runs on the staged files. Each of them gets work of its own.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### File Lists for the Tools

The machinery can hand the expanded list to the tool, so that the tool
examines exactly the files that the action declared. A declaration could then
never miss a file that the tool examines, and an ignore for the whole project
could reach every tool through the list.

The table in the context shows the cost. A list changes what oxfmt, oxlint,
and zizmor examine, because they skip the files that git ignores only on
their own walk, and oxfmt rewrites such a file when a list hands it over. To
avoid that, Rakko would have to apply the ignore rules of each tool itself. A
list must also copy the default selection of each tool, and the copy would
decide what the tool examines, which is the mirrored discovery that [ADR-013]
removed. The patterns of this decision copy the same selection, but only to
order the runs. And a tool that examines a list from Rakko behaves
differently than the same tool that runs bare, which [ADR-011] is there to
prevent.

### Cautious Patterns

The rule of point 2 of [ADR-017] can hold for patterns too. Wherever a
configuration can widen the selection of the tool, the action declares the
whole project.

The actions that wrap taplo and yamllint would then declare the whole project
again, and the formatter for TOML would write all of it with `fix`. Every
action author would also have to know each setting of a tool that can widen
its selection, in every version of the tool. An exact answer needs a look at
the configuration, which ADR-017 and [ADR-013] rule out. No configuration in
the fleet widens a selection today. A declaration that misses a file costs a
race between two runs that the scheduling let share the project, and it
never changes what a run examines, because the run still sees the whole
project.

### Enforced Declarations

The machinery can enforce a declaration when the action runs. It can give
the action a view of the project that holds only the declared files, or it
can check after the run that the action wrote nothing else. A wrong
declaration would then fail, instead of letting two runs race.

Too much is unknown to decide this now, even where a tool accepts the files
that it examines as a list. Each tool applies its own ignore rules to the
files that it is given, as the table in the context shows. An action also
reads files that it does not examine, such as `mise.toml`, and uses
directories that it does not declare, such as the `target/` directory of
cargo. The cost of a copy or an overlay of the project for each run is not
measured either. The scheduling needs none of this, so enforcement waits for
a decision of its own.

### Patterns Without Expansion

A consumer can compare the patterns themselves, and the machinery walks
nothing.

Two patterns can overlap in ways that are not obvious, such as `**/*.json`
and `.config/*`, and a consumer would have to prove for each pair that they
cannot match the same file. A consumer that gets this wrong lets two runs
race. A comparison of two sets of files that exist is simple, and its answer
is exact for the files of the project.

### A Walk That Skips Ignored Files

The expansion can apply the ignore rules of git. The lists then hold fewer
files, and the walk is cheaper in a large tree. On 2026-10-21, the main
checkout of this repository held about 643,000 files under
`.claude/worktrees/`, and taplo alone took about 15 seconds to walk it.

Rakko would then decide what a project ignores, and the lists would miss
files that an action uses. taplo, markdownlint, and yamllint read and write
files that git ignores, so their runs could race with runs whose lists look
disjoint. A filter can come later as an opt-in, if the cost of the walk
demands it.

## Consequences

- Two runs whose lists do not overlap can run at the same time once a
  scheduler exists, such as the formatter for Markdown next to the formatter
  for YAML. Until then, nothing changes for a user: every tool finds its own
  files, and every action examines and reports what it does today.
- A declaration can be wrong, and nothing shows it when Rakko runs. A
  configuration that widens the selection of a tool, or a tool that reads a
  file that the action did not declare, breaks the guarantee without an
  error, and two runs can race. The tests of an action have to check that a
  run writes only the files of its list.
- The lists are wider than the files that a run examines, because the ignore
  rules of a tool only remove files from its default selection, and the walk
  applies no ignore rules. Two actions can conflict on a file that neither of
  them touches. That costs parallelism, and it never lets two runs race.
- A list holds the files that exist when Rakko runs, so it does not show a
  file that a run creates. Two lists that do not overlap thus do not prove
  that the runs cannot race: a writer of the whole project can create a file
  that the other run reads. Points 3 and 5 of [ADR-017] still promise that
  such a writer runs alone, so the work on the scheduler has to keep that
  promise beyond what the lists show.
- An action that writes the configuration of a tool conflicts with every
  action that reads that configuration. With `fix`, the formatter for JSON
  writes every JSON file, dotfiles included, such as `.prettierrc.json` and
  `renovate.json`. The formatter for YAML writes `.yamllint.yaml`,
  `.markdownlint.yaml`, and `zizmor.yml`. So a run of the formatter for JSON
  with `fix` conflicts with the other actions that wrap prettier, and a run
  of either formatter with `fix` conflicts with several of the linters.
- Every action that resolves its tool through mise reads `mise.toml`, and
  the formatter for TOML writes that file with `fix`. So a run of that
  formatter with `fix` conflicts with every such action, as long as the
  actions resolve their tools during their runs.
- The compilers, the test runners, the formatter for Rust, and the check of
  the mise lockfile keep the whole project. A run of them that reads still
  waits for every writer, and a run that writes, such as the formatter for
  Rust with `fix`, still runs alone.
- The walk reads the whole tree, with the ignored files inside it. In a
  large checkout, it can take seconds before any action starts. The
  implementation has to measure that cost before it considers a filter.
- An action whose patterns match no file still runs its tool, and a skip
  still comes out of the run, as [ADR-013] requires. An empty list affects
  the order of the run, and not whether it happens.
- An ignore for the whole project stays apart from this decision. Without
  file lists, taplo, markdownlint, and yamllint never apply the ignore rules
  of git, so such an ignore reaches them only through their own ignore
  files, as long as every run sees the whole project.
- ADR-017 stays as it was accepted, so a reader has to read its points 2 and
  3 together with this ADR.

[adr-004]: 004-actions-as-libraries.md
[adr-011]: 011-tool-integration.md
[adr-013]: 013-applicability.md
[adr-017]: 017-access-declarations.md
[vision]: ../VISION.md
