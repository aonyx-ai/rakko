# ADR-032: Pull Requests

## Status

Accepted

## Context

The [vision] says that CI runs the same commands as a developer, and gets
the same answers. Today the answer of a check of Rakko does not depend on
whether the run is in a pull request. One exception exists: [ADR-028] skips
an action on a platform where its tool has no build, and CI can still run
it.

Otto, the release tool of Aonyx, builds the changelog of a release from
changesets. A changeset is a file in the `.changeset/` directory of a
release unit, such as a crate of this repository. Nothing makes sure that a
pull request adds one. A planned action, the changeset check, fails a pull
request that changes a release unit and adds no changeset to it. The author
can opt a unit out with a line in the body of the pull request that gives a
reason.

This check needs facts that exist only in a CI run for a pull request. It
needs three facts of the pull request:

- the body, which holds the lines that opt a unit out,
- the base and the head commit, which mark the change that the check
  judges, and
- the head branch, which marks a release pull request of Otto. In such a
  pull request, Otto writes the changelog and the versions of a release.

The check is the first Rakko action that needs a pull request. A local run
has no pull request, and a change that nobody committed has no commit to
judge. So the action has nothing to check outside a pull request.

GitHub Actions writes the event that started a job to a JSON file, and the
variable `GITHUB_EVENT_PATH` gives its path. For an event of a pull request,
the file holds the pull request with its body, its base, and its head. The
API of GitHub holds the same facts, but a call to it needs the network, and
the workflow must give Rakko a token.

A project mounts a check so that it runs on every pull request. A way to
skip any check for one pull request breaks that rule. The changeset check
still needs an opt-out, because a change that does not reach users, such as
a refactor, needs no changeset.

[ADR-013] says that an action examines nothing before it runs its tool, and
that a skip comes out of the run of the tool. The look that ADR-013 removed
mirrored the discovery of the tool, so the two can disagree. ADR-028 adds
a skip that the machinery decides from a declaration, before the action
starts. No ADR says how an action learns about a pull request, or what it
does without one.

## Decision

An action can depend on a GitHub pull request. The pull request comes from
the event file of GitHub Actions, and the action skips outside a pull
request.

1. **An action can depend on a pull request.** Such an action gives an
   answer only for a pull request. Its result depends on the body, the
   commits, and the branches of the pull request, and not only on the files
   of the project.

2. **The pull request comes from the event file of GitHub Actions.** Rakko
   reads the facts of the pull request from that file, with no call over the
   network and no token. A job for a pull request from a fork gets no
   secrets, but it gets the event file. So such a job gets the same facts as
   any other job.

3. **Outside a pull request, the action skips.** The run ends as skipped, and
   the reason says that the run has no GitHub pull request. It does not
   fail. A run on the machine of a contributor and a CI run that a person
   starts by hand are examples. A skip does not fail the run, but it shows
   that the check did not run.

   When the run is in a pull request and Rakko cannot read its event file,
   the action stops with an error. A skip there hides a check that has
   something to judge.

4. **Rakko has no generic way to skip a CI check for one pull request.** The
   machinery reads no label and no line in the body to turn off a check that
   a project mounts. An action that judges what a pull request contains can
   offer an opt-out, and the opt-out belongs to that action alone. The
   planned changeset check is the only such action. Its opt-out is part of its
   judgment, because the author tells it why a unit needs no changeset, and
   its output shows that reason.

5. **A bundle exports no such action.** Such an action enforces a rule on
   the pull requests of a project, such as a changeset for each change. A
   bundle mounts its actions in projects that did not choose each one. Such
   an action also needs a CI workflow that runs it again when the author
   edits the body, and each project writes that workflow. So a project
   mounts the action itself, and adds the workflow on purpose.

6. **A later ADR decides about other forges.** A forge is a service that
   hosts Git repositories and their pull requests, such as GitHub or GitLab.
   This decision covers only GitHub and GitHub Actions. A run on another
   forge finds no
   GitHub pull request, so the action skips under point 3. A merge request
   of GitLab can have something to judge, so on another forge this skip can
   hide a check. A later ADR decides how such an action reads the merge
   request of another forge.

This supersedes points 1 and 2 of ADR-013 for an action that depends on a
pull request. It does so only for the question whether the run is in a pull
request. Rakko can read the event file before the tool of such an action
runs. Such an action can skip without a run of its tool. The event file
mirrors no discovery of a tool, and no tool can tell whether the run is in a
pull request.

For every other question, both points stand. The action still examines
nothing of the project before it runs its tool. A tool that had nothing to
examine still decides its own skip. ADR-013 stays Accepted, and its status
line refers to this ADR.

The absence of a pull request is not a precondition of [ADR-022]. A
precondition stops every command, and the absence stops nothing. Only the
action that needs the pull request skips.

The decision records the direction and stops there. The action can read
the event file itself. The machinery can also read it and skip the action
from a declaration, as ADR-028 does. This choice is an implementation
choice. So are the type that holds the pull request, the fields that Rakko
reads, the words of the reason, and the triggers of the workflow.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The API of GitHub

The action can ask the API of GitHub for the pull request. The API gives the
body as it is now, also after an edit and in a re-run of a job. It also
gives the list of the files that the pull request changes. But a call needs
the network, and the workflow must give Rakko a token. The rate limits of
the API also apply. Git gives the
changed files from the checkout of the job, and the event file gives the
other facts.

### Values That the Workflow Passes

The workflow of the project can pass the body and the branches to the
action, as arguments or as variables of the environment. The action then
knows nothing about GitHub. A variable of the environment also keeps the
body out of the commands of the workflow. In a command, shell code in the
body runs as part of the command, which zizmor reports as a template
injection. But each project writes the same mapping again, and a
project that forgets a value gets a check that judges the wrong facts.

### A Skip by the Machinery Alone

The decision can require that the machinery reads the event file and skips
the action from a declaration, as ADR-028 does for a platform. ADR-013 then
stays as it is. But the declaration is a new item of the harness API, and
only one action needs it today. The decision leaves this way open, and the
implementation can take it.

### A Failure Outside a Pull Request

The action can fail when it finds no pull request. Nobody then mistakes a
local run for a check that ran. But a contributor who runs every check
locally then gets a failure with nothing to repair. A failure must give a
cause that the project can change, and this one gives none.

### A Pass Outside a Pull Request

The action can pass when it finds no pull request. A pass and a skip answer
different questions, as ADR-013 says. A pass says that the action examined
the change and found it correct. Outside a pull request, it examined
nothing.

### A Generic Skip for One Pull Request

The machinery can read a label or a line in the body, and skip any check
that it names. One mechanism then serves every check. But each pull request
can then turn off the lint of Rust or the tests. A mounted check then no
longer runs on every pull request. No check except the planned changeset
check needs an opt-out. Also, only a person with triage access to the
repository can set a label, so the author of a pull request from a fork
cannot opt out.

### One Abstraction for Every Forge

The contract can describe a pull request for any forge, such as a merge
request of GitLab. But every project that uses Rakko today is on GitHub. An
abstraction for a forge that no project uses guesses at what that forge
needs. A later ADR can add a forge when a project needs one.

## Consequences

- Such an action gives a different answer in CI and on the machine of a
  contributor. It is the second exception to the rule of the vision that CI
  gets the same answers as a developer. The first is the platform skip of
  ADR-028. A contributor learns about a missing
  changeset only in the pull request.
- Rakko needs no token and no network to read the pull request. The
  workflow gives Rakko no token, and the rate limits of the API do not
  apply.
- The event file holds no list of changed files. The action compares the
  base and the head with Git, so the workflow must check out the history
  that the comparison needs. A checkout of depth 1, the default of
  `actions/checkout`, holds only the merge commit of the pull request, and
  not its base or its head commit.
- The event file describes the pull request when the event started the job.
  A re-run of a job uses the event of the first run, also after an edit of
  the body. So an edit reaches the action only through a new event, such as
  the `edited` event of a pull request. The workflow of the project must
  listen for that event.
- A project that wants to skip another check for one pull request cannot do
  so with Rakko. It repairs the cause, or it removes the action from its
  harness.
- A project adds such an action to its harness by hand, and it also adds the
  workflow that runs it. No bundle brings the action.
- A project on another forge gets only skips from such an action until a
  later ADR covers that forge.
- A reader of ADR-013 must read this ADR as well.
- The glossary entry for the outcome lists the reasons for a skip. It gains
  the absence of a pull request together with the implementation.

[adr-013]: 013-applicability.md
[adr-022]: 022-preconditions.md
[adr-028]: 028-tool-platforms.md
[vision]: ../VISION.md
