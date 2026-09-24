# Rakko Check Renovate Config

`rakko-check-renovate-config` provides the action that checks the [Renovate]
configuration of a project with `renovate-config-validator`, the validator that
Renovate itself ships. The action wraps the validator that mise pinned for the
project, so a run agrees with a contributor that runs the validator bare. The
validator finds the configurations of the project, reads each of them, and
reports every option that Renovate would refuse. The action starts it in the
project root and translates what it reported into an outcome.

The question is a binary one: would Renovate accept the configuration at all.
A configuration that Renovate refuses on the default branch stops Renovate
from opening pull requests until someone fixes it, and nothing tells a pull
request that edits the configuration that it will. The action asks the same
question before the configuration merges.

The validator reports a warning as well as an error, and it reports an option
that Renovate still reads but that a later release renamed or replaced. That
option needs a migration. The action reports all three, and a run with any of
them fails. A project fixes its configuration when it adopts the action, and a
migration that nobody reports stays until a release of Renovate removes the old
option.

The validator writes its log as JSON on request, one record per line, and this
action reads those records. The shape of a record belongs to a version of
Renovate, and the pin softens the risk: a new shape arrives with a new version,
a new version arrives with a pull request, and a record that the action cannot
read stops the run instead of passing quietly, so the drift shows as a red pull
request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.
The action checks and does not lint, because its question is whether Renovate
accepts the configuration, and not whether the configuration obeys a style.

checkrenovateconfig[name]
The action MUST identify itself as `check-renovate-config`.

## Arguments

The action reads no argument. A run only reports. The validator shows the
migration that an option needs, and it writes nothing, so a run of this action
leaves every change to a contributor.

checkrenovateconfig[args.none]
The action MUST declare no argument.

## Tool

The action runs the validator that mise installed for the project, at the
version that the project pinned, so a run reaches the same program as the
terminal of a contributor. The version matters here, because every release of
Renovate knows the options of that release: a version that is older than the
configuration refuses an option that Renovate added since. A validator that
mise does not report stops the action, because provisioning is the job of mise,
and the action installs nothing.

checkrenovateconfig[tool.validator]
A run MUST resolve `renovate-config-validator` through mise for the project of
the run, and MUST run the program that mise reports.

checkrenovateconfig[tool.missing]
A run whose validator mise does not report MUST stop, and the outcome MUST hold
the error.

## Runs

The validator finds the configurations of a project itself when a run names
none. It looks for the file names that Renovate reads, in the root of the
project and in the directories of the code hosts, and it reads the `renovate`
key of `package.json` and the presets in its `renovate-config` key. It also
reads the global configuration that a self-hosted Renovate starts with: the
file `config.js` in the root, or the file that `RENOVATE_CONFIG_FILE` names.
It checks every configuration that it finds, and not only the first one. The
action therefore names no file, so the discovery stays with the tool, and a
run covers what a contributor covers when they start the validator bare in the
root of their checkout.

A consequence of the global configuration is that a `config.js` in the root
that has nothing to do with Renovate is validated as one. Its options are
unknown to Renovate, so the run fails with a finding for each of them, where
the project has no other configuration it would skip. That is the answer of
the tool, and a contributor who runs the validator bare gets the same one.

The validator migrates the global configuration before it validates it, and it
reports a needed migration of that configuration only as part of its log. It
fails no run for it, even with strict validation, so the action passes such a
configuration as the validator does. A global configuration that the
environment holds in a variable instead of a file is only part of the log: the
validator fails no run for any problem of it, an error and a warning included,
and the action passes as the validator does.

A run asks for strict validation. The validator reports a needed migration
either way, but without the option it still ends with success. With it, the
status of the validator agrees with the outcome of the action, and a
contributor who runs the validator with the same option gets the same verdict.
The option changes no option of the configuration.

A run asks the validator to write its log as JSON records, at the level that
announces each configuration that the validator reads. The validator writes
text for a reader by default, and each JSON record carries the file, the topic,
and the message in fields instead of in a sentence that a reader has to take
apart. A record that reports a needed migration names no file, and the record
that announces the configuration before it is what names the file. The level
therefore has to include those announcements, whatever level the environment
of the run asks for. Both select the presentation of the log, and not what the
validator checks. The validator also writes its log to a file when the
environment names one, and a relative name puts that file in the project, so a
run removes that setting and the log goes to the action alone.

A run asks the validator to use the regular expressions of JavaScript. The
validator prefers RE2, an engine that a native module of its package provides,
and it falls back to the regular expressions of JavaScript when the module is
not built, which is where the install of the package leaves it on many
machines. The fallback also writes a warning with a stack trace into every run,
and that warning is about the machine, not about the project. A run that asks
for the fallback gets the same engine on every machine, and no warning about
it.

The choice has a cost. RE2 is the engine that Renovate prefers, and it refuses
some patterns that JavaScript accepts, such as a lookahead. A pattern of that
kind passes this action, and a Renovate that runs RE2 can still refuse it.
Where the native module is built, the run gives up that part of the check.

checkrenovateconfig[run.project]
A run MUST start the validator in the root of the project, and MUST name no
configuration to it.

checkrenovateconfig[run.strict]
A run MUST ask the validator for strict validation.

checkrenovateconfig[run.structured]
A run MUST ask the validator to write its log as JSON records, at the level
that announces each configuration that it validates.

checkrenovateconfig[run.regex]
A run MUST ask the validator to use the regular expressions of JavaScript.

## Skipping

The action applies to a project that holds a Renovate configuration, and the
validator is what decides that. The validator reports on its own when it found
no configuration to validate, and the action turns that report into a skip.

checkrenovateconfig[skip.unconfigured]
A run whose validator reports that it found no configuration MUST report that
the action does not apply, and the reason MUST be what the validator wrote.

## Check

The validator reads each configuration that it found and reports what Renovate
would refuse. Nothing of the project changes, whatever the run finds.

The validator reports a list of errors and a list of warnings for each
configuration, and each entry of a list has a topic and a message. Each entry
becomes a finding at the file that the validator named, and the message of the
finding holds the topic and the message, so that a contributor reads the answer
of the validator and not one that Rakko wrote about it. A warning becomes a
finding like an error, because the validator itself fails a run with either.

A configuration that needs a migration becomes one finding at its file. The
validator reports the configuration as it is and the configuration that the
migration makes of it, and it shows the difference as a drawing of the two for
a reader. The message of the finding names each option at the top of the
configuration that the migration changes, so a contributor knows where to look
without the drawing.

A configuration that the validator cannot read becomes a finding at its file.
The finding sits at the line and the column where the validator names one, and
the message holds what the validator wrote about the file.

A configuration in the `renovate` key of `package.json` becomes a finding at
`package.json`, because the key is not a file of its own.

A passing run names how many configurations the validator validated, so a pass
that examined fewer configurations than a reader expects points at a file that
the validator did not look for. The number is the one that the validator
writes at the end of the run, in which each preset of the `renovate-config`
key counts once.

The action stops when the validator does not finish. The validator ends with a
status of its own when it crashes, and it writes a fatal record when it cannot
read the global configuration that a run of Renovate itself would start with.
In both cases it validated less than the project, and an outcome built on such
a run would describe a part of the project as the whole. A run that ended
without success and reported no problem stops the action as well, because the
validator reports what it found, so such a run failed for a reason that it did
not name. A record that the action cannot read stops the run for the same
reason. This includes a warning about the environment of the run, such as one
about an option of Renovate that a variable of the environment sets: the
action cannot tell such a warning from a problem of the project that a new
version of Renovate reports, so it stops rather than passes.

checkrenovateconfig[check.read]
A run MUST NOT change the project.

checkrenovateconfig[check.passed]
A run whose validator reports no problem MUST pass.

checkrenovateconfig[check.summary]
A passing run MUST name how many configurations the validator validated.

checkrenovateconfig[check.error]
An error that the validator reports MUST produce a finding at the file that the
validator named, and the message MUST hold the topic and the message of the
error.

checkrenovateconfig[check.warning]
A warning that the validator reports MUST produce a finding at the file that
the validator named, and the message MUST hold the topic and the message of the
warning.

checkrenovateconfig[check.migration]
A configuration that the validator reports as one that needs a migration MUST
produce a finding at its file, and the message MUST name each option at the top
of the configuration that the migration changes.

checkrenovateconfig[check.unparsable]
A configuration that the validator cannot read MUST produce a finding at its
file, at the line and the column where the validator names them, and the
message MUST hold what the validator wrote about it.

checkrenovateconfig[check.embedded]
A problem of the configuration in the `renovate` key of `package.json` MUST
produce a finding at `package.json`.

checkrenovateconfig[check.aborted]
A run whose validator crashes or reports that it stopped MUST stop, and the
error MUST hold what the validator wrote.

checkrenovateconfig[check.unreported]
A run whose validator ended without success and reported no problem MUST stop,
and the error MUST hold what the validator wrote.

checkrenovateconfig[check.unreadable]
A run whose log the action cannot read MUST stop, and the error MUST hold what
the validator wrote.

[renovate]: https://docs.renovatebot.com
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
