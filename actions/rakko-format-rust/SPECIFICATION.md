# Rakko Format Rust

`rakko-format-rust` provides the action that formats the Rust code of a
project with [rustfmt]. The action wraps the cargo that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
rustfmt bare. Cargo reads the manifests, and rustfmt reads its own
configuration and formats every target. The action selects the operation — a
check, or a fix — and translates what rustfmt reported into an outcome.

Rustfmt reports a run as text, and the action reads that text, because
rustfmt offers nothing structured for a check. The shape of the text belongs
to a version of rustfmt, and the pin softens the risk: a new shape arrives
with a new version, a new version arrives with a pull request, and a report
that the action does not recognize stops the run instead of passing quietly,
so the drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

formatrust[name]
The action MUST identify itself as `format-rust`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look that runs before the tool resolves, so that a
broad bundle stays safe: a project without Rust code and without a cargo skips
visibly instead of stopping over a tool that it has no reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

formatrust[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

formatrust[skip.git]
The examination MUST NOT read the `.git` entry of the project.

formatrust[skip.target]
The examination MUST NOT read a directory named `target`.

formatrust[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads one argument. A run reports by default, and the `fix`
argument lets rustfmt rewrite what it can format. Reporting is the safe
default, because a run that a user started in order to look must not change
the tree that they hold.

formatrust[args.fix]
The action MUST declare one argument: `fix`, holding a value that is true or
false, with documentation.

formatrust[args.value]
A value for `fix` that is not true or false MUST fail the construction of the
arguments, and the failure MUST report the argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A cargo that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

Rustfmt honors the unstable options of its configuration only on the nightly
channel. A stable rustfmt warns and formats without them, and it reports a
diff against code that a nightly rustfmt formatted, so a run on the default
toolchain would fight the editor of every contributor who formats with
nightly. The action therefore runs rustfmt on the nightly toolchain that the
project pins, and a project that pins none stops the action, for the same
reason that a missing cargo does.

formatrust[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

formatrust[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

formatrust[tool.toolchain]
A run MUST run cargo on the toolchain that mise installed for the `nightly`
channel of the project.

formatrust[tool.unpinned]
A run in a project whose `nightly` channel mise does not report as pinned and
installed MUST stop, and the outcome MUST hold the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo formats one workspace at a time. A run
therefore formats every workspace root of the project, and a workspace that
the run cannot discover stops it, because a run that skipped a workspace
would hide every problem of that workspace behind a green result.

formatrust[roots.all]
A run MUST format every workspace root of the project.

formatrust[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Check

Every run starts with a check: rustfmt examines every target of every
package and reports the problems, and nothing rewrites the project. The
action asks cargo for the short report, which lists the files that rustfmt
would rewrite instead of showing a diff of each, because the action reads
the report as data. This selects the presentation of the report and not the
behavior of the tool: what rustfmt does to the project comes from the
configuration of the project alone.

Rustfmt reports two kinds of problems. A file that is not formatted gets a
path and nothing else, and a file that rustfmt cannot parse gets a line, a
column, and a message. Both are problems of the project, so both travel as
findings, each at the level that rustfmt could name.

A configuration that rustfmt does not honor stops the run. Rustfmt warns
about an option that it does not know, and about an option that its channel
does not support, and then it formats without the option. A run without the
option quietly does what the project asked it not to do, so the action
treats the warning as the end of the run.

A report that the action does not recognize stops the run as well. A run
that ended without success and named no problem wrote a report that the
action could not read, and an answer built on such a report would hide every
problem behind a green result. A configuration that rustfmt cannot parse
ends this way, because rustfmt then stops before it looks at a file.

formatrust[check.read]
A run without a true value for `fix` MUST NOT change the project.

formatrust[check.operation]
A run MUST ask rustfmt to report instead of rewriting, and MUST ask cargo for
the short report. It MUST NOT change any other option of rustfmt.

formatrust[check.passed]
A run whose rustfmt reports no problem at any root MUST pass, and the outcome
MUST say how many workspaces the run checked.

formatrust[check.unformatted]
A file that rustfmt reports as not formatted MUST produce a finding that
names that file, with the path relative to the project root.

formatrust[check.invalid]
A file that rustfmt cannot parse MUST produce a finding at the line and the
column that rustfmt reports, with the message of rustfmt.

formatrust[check.configuration]
A run whose rustfmt warns about its configuration MUST stop, and the error
MUST hold what rustfmt reported.

formatrust[check.unrecognized]
A rustfmt run that ends without success and reports no problem that the
action recognizes MUST stop the run, and the error MUST hold what rustfmt
wrote.

## Fix

A run with the fix argument repairs what it can. The check runs first, so
that the action knows every problem of a workspace, and the rewrite follows.
The rewrite lists the files that it rewrote, so the action knows the repairs
from the rewrite and the problems that remain from the check. A file that
rustfmt cannot parse remains, and so does every file of the package that
holds it, because rustfmt rewrites nothing in a package that it cannot read
as a whole. A rewrite repairs formatting, and a syntax error needs a hand.

formatrust[fix.write]
A run with a true value for `fix` MUST let rustfmt rewrite the files that are
not formatted.

formatrust[fix.changed]
A run that repaired every problem that the check found MUST report the
change, and the outcome MUST hold one repair for each file that rustfmt
rewrote.

formatrust[fix.partial]
A run that repaired part of what the check found MUST fail, and the outcome
MUST hold the repairs next to the problems that remain.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[rustfmt]: https://github.com/rust-lang/rustfmt
[tracey]: https://tracey.bearcove.eu/
