# Rakko Check Mise Lock

`rakko-check-mise-lock` provides the action that checks whether the lockfile
of [mise] is complete. A lockfile of mise records the version of each tool
that the project pins, and for a tool that mise downloads, the address and the
checksum of the file for each platform. An install in locked mode stops when
the lockfile has no entry for the platform of the machine. Locked mode
examines only that one platform, so a gap stays hidden while every machine
that runs the install has the tool already. The gap shows when a new machine,
a new platform, or a new contributor installs the tool.

Mise has no operation that only checks the lockfile. The action therefore runs
`mise lock`, which writes every entry that the lockfile needs, and compares the
lockfile before and after the run. A lockfile that `mise lock` leaves as it is
passes. Every entry that `mise lock` adds, changes, or removes is a finding.

The question is a binary one: is the lockfile complete for the platforms that
the project names. The name of the action therefore starts with `check`.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checkmiselock[name]
The action MUST identify itself as `check-mise-lock`.

## Arguments

The action reads one argument. A run reports by default, and `fix` lets mise
write the lockfile of the project. Reporting is the safe default, because a run
that was started in order to look must not change the tree that its caller
holds. With `fix`, a contributor repairs the lockfile with one command.

checkmiselock[args.fix]
The action MUST declare one argument: `fix`, holding a value that is true or
false, with documentation.

checkmiselock[args.value]
A value for `fix` that is not true or false MUST fail the construction of the
arguments, and the failure MUST report the argument.

## Tool

The tool of the action is mise itself. The canonical way to start a harness
enters the environment of mise first, so a run that reaches the action reaches
mise as well, and the operating system finds it by its name. The project pins
no version of mise, so the action resolves nothing through mise.

A mise that does not start, and a mise that ends without success, stop the
action. Mise writes what went wrong, and the error holds those words. A
configuration of mise that the action cannot read, a configuration that needs
trust and that nobody trusted, and a lockfile that mise cannot read all end a
run of mise without success.

checkmiselock[tool.mise]
A run MUST start the program that the operating system finds by the name
`mise`.

The environment can answer every question of mise in advance with
`MISE_YES`, which the setup action of mise for CI sets. Mise then trusts a
configuration that nobody trusted, and it records that trust for good. The
action takes the variable out of the environment of each run of mise, so that
it never trusts a project in the place of whoever owns it. A project that a
job of CI checks stays trusted through the directories that the environment
names in `MISE_TRUSTED_CONFIG_PATHS`.

checkmiselock[tool.unanswered]
A run MUST start every run of mise without the variable `MISE_YES` of its
environment.

Mise also trusts every configuration, without a question and without a record,
when the environment holds `CI` or `GITHUB_ACTIONS`, which every job of GitHub
Actions sets. The action takes both variables out of the environment of each
run of mise as well, so that a project that nobody trusted stops the run on a
job of CI too.

checkmiselock[tool.unattended]
A run MUST start every run of mise without the variables `CI` and
`GITHUB_ACTIONS` of its environment.

checkmiselock[tool.failed]
A run whose mise does not start or ends without success MUST stop, and the
outcome MUST hold what mise wrote.

## Lockfiles

Mise writes a lockfile next to each configuration file of the root of the
project. A project can keep its configuration in `mise.toml`, in
`.config/mise.toml`, or in another place that mise reads, and a project can
use more than one of them. Mise names the lockfiles that it keeps on request,
as data, so the run asks mise and does not search the project itself. The run
asks before and after `mise lock`, because `mise lock` creates a lockfile that
does not exist yet.

The run compares the lockfile of the project before the run with the same
lockfile after the run. A lockfile that only one of the two lists counts as an
empty lockfile on the other side.

Mise keeps the dependency graph of some tools, such as those of npm, in a
directory beside the lockfile, and the entry of the tool records the digest of
the graph. A change of the digest is a change of the entry, and it gives a
finding. The action compares the lockfiles and not the directories of the
graphs, so a graph that is missing while its entry stays the same gives no
finding.

The action reads a lockfile as TOML. A lockfile holds a list of entries for
each tool, and each entry names its version. An entry holds a table for each
platform that it records, and fields that every platform shares, such as the
backend of the tool. A lockfile that is not TOML, or that does not have this
shape, stops the run, because an answer built on a misread lockfile would hide
gaps behind a green result. The same is true for a list of lockfiles that the
action cannot read, and for a lockfile that mise names outside the project.

checkmiselock[lockfiles.listed]
A run MUST compare every lockfile that mise names for the directory where the
run locks, the project with `fix` and the copy without it, before and after
`mise lock`.

checkmiselock[lockfiles.unreadable]
A run whose list of lockfiles, or one of whose lockfiles, the action cannot
read MUST stop, and the outcome MUST hold the error.

## Copy

Without `fix`, the run must not change the checkout of the contributor. The run
therefore runs `mise lock` in a disposable copy of the project. The copy holds
the commit that the project is on, with the changed files of the checkout over
it, so a contributor who adds a tool to the configuration and runs the check
before they commit reads the answer for the new tool. The copy is a working
tree of the repository of the project, so a project that is no repository has
no copy, and such a run stops instead of writing where the contributor works.

The copy holds no file that git ignores. A configuration that git ignores, such
as `mise.local.toml`, therefore does not reach the run, and neither does its
lockfile. A lockfile that git ignores is missing from the copy, as it is from
every other checkout of the repository. The run reads the lockfiles before
`mise lock` in the copy as well, so that it compares what `mise lock` changed
there, and a file that only the checkout holds gives no finding. The question
for the lockfiles in the project still makes mise read every lockfile that the
project keeps, the ones that git ignores among them, so a lockfile there that
mise cannot read stops the run. The copy also lies in the temporary directory
of the system, so a configuration of mise in a directory above the project
reaches a run with `fix` and not a run without it.

Mise reads a configuration that needs trust, such as one with settings, an
environment, or hooks, only after somebody trusted the directory that
holds it, and the copy is a directory that nobody has seen before. Before it
creates the copy, the run asks mise for the lockfiles of the project, in the
project. Mise refuses that question when such a configuration of the project
has nobody's trust, so the project stops there. The copy holds the same
configuration, so the run then trusts the copy for each run of mise in the
copy, and for no other run. The trust lives in the environment of that run of
mise, and no record of it remains. Recent versions of mise also trust a working
tree of a repository whose configuration somebody trusted, and the trust of the
run keeps the copy independent of that.

checkmiselock[copy.disposable]
A run without `fix` MUST run `mise lock` in a copy of the project, and MUST
leave the working tree of the project as it was, also when the run fails or
stops.

checkmiselock[copy.uncommitted]
A run without `fix` MUST answer for the working tree of the project, with the
changes that are not committed.

checkmiselock[copy.trust]
A run without `fix` MUST ask mise for the lockfiles of the project in the
project before it creates the copy, and MUST trust the copy only for the runs
of mise in the copy. Such a run MUST NOT read the lockfiles of the checkout.

checkmiselock[copy.compared]
A run without `fix` MUST compare the lockfiles that the copy holds before
`mise lock` with the lockfiles that it holds after.

checkmiselock[copy.unavailable]
A run without `fix` in a project that is not a git repository, and a run whose
copy cannot be created, MUST stop, and the outcome MUST hold the error.

## Lock

The run passes no option to `mise lock`. Mise targets the platforms that the
setting `lockfile_platforms` of the project names, and without the setting, the
platforms that the lockfile records already. So the configuration of the
project decides which platforms count. Mise also adds the platform of the
machine that runs it to the platforms of the setting. A run on a machine whose
platform the setting does not name therefore gets a finding for each tool that
has no entry for that platform. A project names every platform that its jobs
and its contributors use, and then the platform of the machine adds nothing.

`mise lock` keeps the versions that the lockfile holds, so a run changes no
version that the project pinned. A tool without a build for a platform gets no
entry for that platform, and `mise lock` does not fail.

Mise also ends with success when it could not resolve a tool for a platform, for
example because a download or a query over the network failed. It then leaves
the entry of that platform as the lockfile had it, or it writes an entry that
holds no address of a download, such as one that holds only the provenance of
the download. A missing entry that mise could not resolve stays missing before
and after the run, so the comparison would pass over a real gap.

Mise warns that it failed to resolve the tool, that it failed to fetch what it
needs, or that an API limited the rate of its requests, and it gives no such
warning for a tool without a build. A run whose `mise lock` warns therefore
stops, and the error holds each warning as a whole, with the lines that say how
to resolve it. A few kinds of warning say nothing about what mise locked, and
they stop nothing: that a lockfile keeps an older format, that the registry
moved a tool to another backend, that a backend is deprecated, that a request
failed and is tried again or elsewhere, and that a newer version of mise exists.
A warning of another kind stops the run, because a new version of mise can give
a new warning for an unresolved tool, and a run that passed over it would hide a
gap.

An install in locked mode refuses a platform whose entry holds no address, and
mise also writes such an entry without a warning, for example for a release that
does not exist. A run whose lockfile holds such an entry after `mise lock`, for
a platform that mise targeted for that lockfile, therefore stops as well,
whatever mise wrote and whether or not the run changed the entry, and the error
names each such part. `mise lock` leaves the entries of a platform that it does
not target as they were, so an old entry without an address for such a platform
gives nothing.

Mise names the platforms that it targets for each lockfile in its report of the
run. A lockfile with platforms that the report does not name stops the run,
because the run then cannot tell an entry of a targeted platform from an old
one, and a report in another shape belongs to a version of mise that the action
does not know. A report that names a lockfile outside the directory where the
run locks stops the run as well. Mise writes such a lockfile when the
configuration of the tools lies in a directory above the project, and a run that
went on would compare no lockfile of the project, and with `fix` it would write
outside the project.

A failure of the network is temporary, and a second run answers the question.
Other causes last, such as a release without the attestations that mise checks,
or a plugin that is not installed, and the words of mise in the error of a
warning say which of the two it is. An entry without an address that a second
run does not repair belongs to a release or a platform without a download, such
as a platform that the tool has no build for. A contributor then checks that the
release and its download exist, removes the table of that platform, or removes
the platform from `lockfile_platforms`.

The warnings are a sign of such a failure, so the run of `mise lock` makes mise
show them. It sets `MISE_QUIET` and `MISE_SILENT` to `false` and
`MISE_LOG_LEVEL` to `warn` in the environment of that run. A variable of the
environment beats the same setting in a configuration, so the warnings show also
when the environment, the project, or the global configuration of the
contributor silences mise, with the setting `quiet`, `silent`, or a `log_level`
that hides warnings. Each of these settings selects what mise shows, and not
what it locks, and each silences mise on its own, so the run sets all three.

The lockfile depends on the version of mise as well. A contributor whose mise
is older or newer than the one in CI can get a lockfile in another shape, and
then a finding that CI does not report. CI decides in that case.

One known behavior of mise gives a finding that a second run does not give.
For a tool that sets `os`, and whose entry in the registry of mise carries
options, the second `mise lock` adds a second entry for the same version, with
those options, and the third run changes nothing. `hk` with
`os = ["linux", "macos"]` does this. A lockfile that `mise use` wrote one time
can therefore give a finding, and a run with `fix` repairs it.

checkmiselock[lock.operation]
A run MUST ask mise to lock the project with `mise lock` and with no other
option.

checkmiselock[lock.unresolved]
A run whose `mise lock` gives a warning of another kind than the harmless kinds
that this section names MUST stop, and the outcome MUST hold each such warning
as mise wrote it.

checkmiselock[lock.unaddressed]
A run whose lockfile, after `mise lock`, records a platform that mise targeted
for that lockfile without an address of a download MUST stop, and the outcome
MUST name each such part.

checkmiselock[lock.foreign]
A run whose `mise lock` reports that it locks a lockfile outside the directory
where the run locks MUST stop, and the outcome MUST name the lockfile.

checkmiselock[lock.untargeted]
A run whose lockfile, after `mise lock`, records a platform while mise named no
platform that it targeted for that lockfile MUST stop, and the outcome MUST
name the lockfile.

checkmiselock[lock.warnings]
A run MUST start `mise lock` with `MISE_QUIET` and `MISE_SILENT` set to `false`
and `MISE_LOG_LEVEL` set to `warn`.

## Skipping

The action applies to a project that configures tools to lock, and mise is
what decides that. Mise reports on its own when the project configures no
tools to lock, and the action turns that report into a skip.

checkmiselock[skip.unconfigured]
A run whose mise reports that the project configures no tools to lock MUST
report that the action does not apply, and the reason MUST be what mise wrote.

## Comparison

The run compares each tool, version, and platform. An entry of a tool for a
version has one part for each platform that it records, and that part holds
the table of the platform and the fields that the entry shares. An entry that
records no platform has one part without a platform. When `mise lock` writes
the platforms of such an entry, the part without a platform goes away, and the
parts of the platforms name what changed, so that part gives no finding of its
own. A tool can have more than one entry for the same version, and those
entries compare in the order of the lockfile. A part that `mise lock` gives
more entries counts as added, and a part that it gives fewer counts as
removed.

A part that `mise lock` adds, changes, or removes is a finding. A finding names
the tool, the version, and the platform, and it sits at the lockfile. A project
without a lockfile gets a lockfile from `mise lock`, so every part of the new
lockfile is a finding.

A passing run names how many lockfiles it compared, so a pass that compared
fewer lockfiles than a reader expects points at a configuration that mise did
not read.

checkmiselock[compare.passed]
A run whose `mise lock` leaves every lockfile as it was MUST pass, and the
outcome MUST name how many lockfiles the run compared.

checkmiselock[compare.added]
A part that `mise lock` adds MUST produce a finding at its lockfile, and the
message MUST name the tool, the version, and the platform of the part.

checkmiselock[compare.changed]
A part that `mise lock` changes MUST produce a finding at its lockfile, and the
message MUST name the tool, the version, and the platform of the part.

checkmiselock[compare.removed]
A part that `mise lock` removes MUST produce a finding at its lockfile, and the
message MUST name the tool, the version, and the platform of the part.

checkmiselock[compare.unplatformed]
An entry that records no platform MUST compare as one part, and its finding
MUST name the tool and the version. A part without a platform MUST NOT produce
a finding when a part of the same tool and version with a platform produces
one.

checkmiselock[compare.missing]
A run in a project without a lockfile MUST fail with a finding for each part of
the lockfile that `mise lock` creates.

## Fix

With `fix`, the run runs `mise lock` in the checkout, and the result of
`mise lock` stays in the project. The run compares the lockfiles in the same way
as without `fix`, and each difference is a repair. `mise lock` can also rewrite
a lockfile without a change to any part, for example when it writes the header
of the file again. Such a rewrite gives no repair, and the run passes. A run
with `fix` that stops after `mise lock` can leave what mise wrote in the
project, such as a run whose mise failed to resolve a tool for a platform. A
run with `fix` also compares and keeps what `mise lock` writes to a lockfile
that git ignores, such as `mise.local.lock`, which a run without `fix` never
sees.

checkmiselock[fix.write]
A run with a true value for `fix` MUST run `mise lock` in the project, and
MUST leave its result there.

checkmiselock[fix.changed]
A run with `fix` whose `mise lock` changed a part of a lockfile MUST report the
change, and the outcome MUST hold one repair for each part that `mise lock`
added, changed, or removed.

[mise]: https://mise.jdx.dev
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
