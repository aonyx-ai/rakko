# ADR-023: Rakko Updates

## Status

Accepted

## Context

[ADR-021] makes each action pin its tools. Rakko writes the pins of the
mounted actions into a mise file and its lockfile, and the project commits
both. [ADR-022] adds a precondition: when the file does not match the
declarations, every run stops, also in CI, and `doctor --fix` writes the
file again. Both ADRs leave open how an update of Rakko changes the file in
each project.

A project receives Rakko through its update bot, which opens a pull request
for each new revision. In the fleet, the bot is the hosted app of Renovate,
and the pull request merges itself when CI passes. In 18 days of October
2026, one project of the fleet merged 34 of these pull requests, almost all
of them on the day that they opened. In 30 days, Rakko moved a tool of an
action 16 times. Each move changes the generated file in every project that
mounts the action. A bundle release can also add an action, and with it a
tool that the file does not hold yet.

Such a pull request fails until something writes the file again. Only the
harness of the project knows the mounted actions, so the write needs the
harness. The lockfile also needs the network. The hosted app of Renovate
runs a custom command only when Mend allows it, and the free plan allows
none. The mise manager of Renovate can change a version in the file. But it
cannot learn the versions of a new revision of Rakko, and it cannot add or
remove a tool.

GitHub sets a second limit. A workflow can push a commit with its own token,
`GITHUB_TOKEN`. But CI on that commit then waits until a person with write
access approves the run. A run that the workflow starts itself does not
satisfy a required check. So an update that needs no person needs a
credential from outside the workflow.

Rakko prepares a public launch. The way that an update reaches the generated
file must work for a project outside Aonyx, and not only for the fleet. A
project that adopts Rakko must be able to update it without a repair by
hand, and with as little setup as possible.

## Decision

A workflow in the project repairs the pull request of the update bot. It
runs `doctor --fix` on the branch of the pull request and commits the files
that change. The workflow is an action with a repository of its own, next to
a preset for Renovate. The credential that the project gives to the action
decides whether a person approves the repair.

1. **The repair runs on the branch of the bot.** The action runs on a pull
   request that a bot opened, such as Renovate or Dependabot. It runs the
   harness of the new revision with `doctor --fix`, and it commits the
   changed files to the same branch. So the update and its generated files
   arrive in one pull request. On the pull request of a person, the action
   does nothing, and the precondition tells the person to run
   `doctor --fix`.

2. **The action has a repository of its own.** A project calls the action
   and does not copy it. The action has its own versions and its own
   documentation, so a change to Rakko or to one of its actions does not
   make a new version of it. Its contract with Rakko is
   `mise run rakko -- doctor --fix`: every harness runs as `mise run rakko`
   ([ADR-009]), and `doctor` is a reserved command ([ADR-022]). The action
   is an addition to Rakko, not a part of it. A project without the action
   repairs each update by hand, as the precondition says.

3. **The credential decides who approves.** The only input of the action is
   the credential, and the token of the workflow is its default. With that
   token, the repair needs no setup, but GitHub holds CI on the commit until
   a person with write access approves the run with one click. A credential
   from outside the workflow, such as the token of a GitHub App or a
   personal access token, starts CI without a person. GitHub does not let
   the token of the workflow do this, so an update that needs no person
   always needs this setup.

4. **The bot keeps its pull request.** The commit of the action does not
   stop the bot from updating its branch. When the bot rebases the branch or
   moves it to a newer revision, the action repairs the branch again. Rakko
   publishes a preset for Renovate that keeps the mise manager of Renovate
   away from the generated file. The repository of the action publishes a
   preset that extends the preset of Rakko and lets Renovate continue on its
   branch after a commit of the action. A project extends one of the two
   presets.

5. **The route can run without a person.** With a credential from outside
   the workflow, neither the repair nor the merge waits for a review. A
   project whose updates merge themselves when CI passes keeps that. The
   tests of an action already ran against the new version of its tool, in
   the pull request to Rakko that moved the pin, and CI in the project
   checks the result. A person acts only when CI fails. For example, a new
   version of a linter can report a new problem, or an action can need a
   newer toolchain than the project pins. In the second case, the action has
   nothing to repair.

The decision records the direction and stops there. The name of the action
and of its repository, how it releases its versions, the rules of the
presets, how the action recognizes a bot, the identity of its commits, and
which credential the documentation recommends are implementation choices.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Repair by Hand

The precondition already tells a person how to repair a project: run
`doctor --fix`. A person can do this on each pull request, with no action
and no credential. But at the rate of the fleet, that is up to about 30
repairs a week, and every move of a tool turns into a red pull request in
every project that mounts the action. The repair by hand stays the fallback
for a project without the action.

### A Command After the Update

Renovate can run a command after it updates a dependency and before it
commits. The update and the files then arrive in one commit, and nothing
else touches the branch. But the hosted app runs a command only when Mend
allows it. The free plan allows none. An open source project can ask Mend
for one command, but Mend decides for each project, and a paid plan asks
every adopter to pay.

### A Self-Hosted Renovate

A Renovate that runs on the persistent self-hosted runners of the fleet can
run any command after an update. But each adopter would then run a Renovate
of their own: a GitHub App, a schedule, and the upkeep that Mend does today.
That is too much to ask of a project that only wants to adopt a tool.

### Versions From a List

Rakko can publish the versions that its actions declare, and the preset can
point the mise manager of Renovate at that list. Renovate then changes the
versions in the generated file and writes the lockfile, with no command and
no credential. But Renovate can only change a version that the file already
holds. It cannot add the tool of a new action, and it cannot remove a tool.
If Rakko releases its crates to a registry, the version of a tool also
depends on the version of its crate that `Cargo.lock` resolves, and a list
cannot know that. A project would still need the action for the other cases.

### The Token of the Workflow Alone

The action could make the token of the workflow enough for an update that
needs no person. But a run that the action starts with `workflow_dispatch`
does not satisfy a required check of a ruleset. The action could post the
result of CI as a status with the name of the required check, and GitHub
would accept it. But that goes around a gate that GitHub added on purpose,
and GitHub can close the gap. So the token of the workflow stays the
default, with the approval of a person.

### The Action in the Rakko Repository

The action can live in the Rakko repository, next to the preset of Rakko,
and change in the same pull request as Rakko. But then each revision of
Rakko is a new version of the action, and the projects update the action
much more often than it changes. GitHub Marketplace also lists only an
action whose metadata file is at the root of its repository. The action
depends only on `doctor --fix`, so it gains little from a place next to
Rakko.

### A Review of Every Tool Change

The action could wait for a person on every update that changes a tool, also
with a credential that starts CI. But the tests of the action already ran
against the new version in Rakko. A review would add dozens of approvals a
week, and people learn to approve what they do not read.

## Consequences

- An update of Rakko that changes the tools of a project brings the new
  generated files in the same pull request. This includes a bundle release
  that adds an action with a new tool.
- A project that adopts the action extends its preset, adds the workflow,
  and runs `doctor --fix` once. That is enough for updates that a person
  approves with one click. For updates that need no person, the project also
  stores a credential, such as the key of a GitHub App. GitHub does not let
  Rakko remove this step.
- With the token of the workflow, each rebase of the branch needs another
  click. A run that waits for approval expires after 30 days.
- The action works with every update bot on GitHub. A project on another
  platform, or a project without a bot, repairs its updates by hand, as the
  precondition says.
- Each rebase of the branch drops the commit of the action, and the action
  commits again. Each rebase therefore costs one more run of CI.
- The action runs the harness of the new revision with a credential that can
  write to the repository. CI on a pull request usually gets a token that
  can only read. So the code of an update of Rakko, and of its dependencies,
  runs with more rights than before. The launch promises no security
  boundary. A ruleset that requires a pull request for the default branch
  keeps the credential away from that branch.
- The rules of the preset of Rakko become public, and they freeze with the
  launch. The action follows versions of its own, so a change to its input
  does not wait for a release of Rakko.
- Aonyx maintains one more repository, with its own CI and releases. A new
  name for Rakko also reaches that repository.
- The fleet needs one GitHub App for its organization. The action calls
  `doctor`, which older revisions do not have, so a project cannot add the
  action before the update that brings the preconditions. In each project, a
  person repairs that one pull request and adds the action.
- This repository keeps the mise manager of Renovate on its own generated
  file. Here the mise file of a crate and the generated file hold the same
  pin, and both move in one pull request ([ADR-021]).

[adr-009]: 009-harness-entry-point.md
[adr-021]: 021-tool-declarations.md
[adr-022]: 022-preconditions.md
