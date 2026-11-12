+++
title = "Keep a Harness Current"
description = "Update harness dependencies and tool pins with Renovate, Dependabot, or Cargo."
weight = 60
+++

Adopt new releases through your usual dependency updates. This guide also
keeps tool pins in the same pull request, so you can review and test the
complete update together.

Keep these three parts current:

- The direct dependencies in the harness's `Cargo.toml`, such as
  `rakko-cli` and bundles.
- The actions inside bundles, whose resolved versions appear in the
  harness's `Cargo.lock`.
- The tool pins in `.config/mise/conf.d/rakko.toml`, which follow the actions.

You need a working harness. If you do not have one yet, follow
[Get Started][get-started] first. The examples keep the harness in
`tools/rakko`. If yours is in another directory, change the paths.

## Take New Releases of the Crates of the Harness

[Renovate] finds every `Cargo.toml` in the repository, the manifest of the
harness included, and opens a pull request for each new release. Extend the
preset of Rakko in your configuration of Renovate:

```json,name=renovate.json
{
  "extends": ["github>aonyx-ai/rakko"]
}
```

The preset groups Rakko crate updates in one pull request. It also excludes
the generated tool file, which `doctor --fix` manages. If you use the
`rakko-update` workflow described below, use that workflow's preset instead.

[Dependabot] looks only in the directories that its configuration gives. Add
the directory of the harness:

```yaml,name=.github/dependabot.yml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /tools/rakko
    schedule:
      interval: weekly
```

## Refresh the Lock File of the Harness

A bundle can accept a new action release without releasing a new version
itself. Updating only direct dependencies can therefore leave those actions
at the versions in `Cargo.lock`. Refresh the harness lock file to adopt
compatible action updates.

### With Renovate

Renovate calls this lock file maintenance. Once enabled, its default schedule
is early on Monday. This configuration limits the refresh to the harness
lock file:

```json,name=renovate.json
{
  "extends": ["github>aonyx-ai/rakko"],
  "lockFileMaintenance": { "enabled": true },
  "packageRules": [
    {
      "description": "Refresh only the lock file of the harness",
      "matchUpdateTypes": ["lockFileMaintenance"],
      "matchFileNames": ["!tools/rakko/**"],
      "enabled": false
    }
  ]
}
```

To refresh all project lock files, including workspace and npm lock files,
use Renovate's weekly maintenance preset instead:

```json,name=renovate.json
{
  "extends": ["github>aonyx-ai/rakko", ":maintainLockFilesWeekly"]
}
```

### With Dependabot

By default, Dependabot moves only the crates that the manifest lists. Allow
every dependency, and put the updates in one group, so that a refresh arrives
as one pull request:

```yaml,name=.github/dependabot.yml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /tools/rakko
    schedule:
      interval: weekly
    allow:
      - dependency-type: all
    groups:
      harness:
        patterns:
          - "*"
```

### Without a Bot

Refresh the lock file of the harness with Cargo, and write the tool pins
again:

```sh
cargo update --manifest-path tools/rakko/Cargo.toml
mise run rakko -- doctor --fix
mise install
```

Then run your checks, and commit the changed files.

## Keep the Tool Pins Current

An action update can change a tool version or add a tool. Until the generated
pins match, the harness stops actions, groups, and custom commands, including
in CI. Help, the command list, and `doctor` remain available.

Run `mise run rakko -- doctor --fix` on the update branch, or use the GitHub
Action [`aonyx-ai/rakko-update`][rakko-update]. It runs the same command on
pull requests from update bots and commits the changed files to that branch.
The dependency update and its tool pins then stay together.

### Add the Workflow

Commit the result of `doctor --fix` once before you add the workflow. Then
add the workflow, for example as `.github/workflows/rakko-update.yml`:

{% raw %}

```yaml,name=.github/workflows/rakko-update.yml
---
name: Rakko

"on":
  pull_request:

permissions: {}

concurrency:
  group: rakko-update-${{ github.event.pull_request.number }}
  cancel-in-progress: true

jobs:
  update:
    name: Update the tool pins
    runs-on: ubuntu-latest

    permissions:
      contents: write

    steps:
      - name: Update the tool pins
        uses: aonyx-ai/rakko-update@<commit> # main
```

{% endraw %}

Replace `<commit>` with a full commit hash from `aonyx-ai/rakko-update`'s
`main` branch. Keep the `# main` comment so that Renovate can update the pin.

`rakko-update` checks out the branch, installs the tools of the project with
[mise], builds the harness, and runs `doctor --fix`. The job needs `contents: write`
to push. A workflow that Dependabot starts gets a token that can only read,
and the `permissions` key of the job gives that token write access too.

### Let the Bot Continue

Renovate stops updating a branch after another author commits to it. The
`rakko-update` preset lets Renovate continue after the action commits. It
includes the Rakko preset, so replace that preset with:

```json,name=renovate.json
{
  "extends": ["github>aonyx-ai/rakko-update"]
}
```

Dependabot needs no change. Each commit of `rakko-update` tells Dependabot to
continue.

### Choose the Credential

By default, `rakko-update` pushes with the token of the workflow. GitHub then
waits for a person with write access to approve the CI run on that commit,
with one click in the pull request. Each rebase of the branch needs another
click.

To let CI continue without manual approval, give `rakko-update` a GitHub App
token with write access to repository contents. A personal access token also
works. Create the app token in the job and pass it to `rakko-update`:

{% raw %}

```yaml,name=.github/workflows/rakko-update.yml
steps:
  - name: Create the token of the app
    id: app-token
    uses: actions/create-github-app-token@<commit> # v2
    with:
      app-id: ${{ vars.RAKKO_UPDATE_APP_ID }}
      private-key: ${{ secrets.RAKKO_UPDATE_PRIVATE_KEY }}

  - name: Update the tool pins
    uses: aonyx-ai/rakko-update@<commit> # main
    with:
      token: ${{ steps.app-token.outputs.token }}
```

{% endraw %}

If you use Dependabot, also store the app's private key as a Dependabot
secret. Workflows that Dependabot starts cannot read ordinary repository
secrets.

### Behavior of `rakko-update`

`rakko-update` changes a pull request only if an update bot opened it and
made the latest commit. It recognizes bots such as `renovate[bot]` and
`dependabot[bot]`. Use the `bots` input to add other accounts, such as
self-hosted Renovate.

For a pull request opened by a person, run `doctor --fix` and commit the
result yourself.

If an action needs a newer toolchain than your project pins, `rakko-update`
fails without pushing changes. The `doctor` report identifies the pin that
you need to update.

## Keep the Toolchains Current

Your project pins its toolchains, such as Rust, in its own `mise.toml`, and
`doctor --fix` never changes them. Renovate moves these pins like any other
dependency. The actions accept a range of versions for each toolchain, and
`doctor` reports a pin outside the range of an action.

[Tools and Mise][tools-and-mise] explains who pins which tool, and what
`doctor` checks.

[dependabot]: https://docs.github.com/en/code-security/dependabot
[get-started]: @/tutorial/get-started.md
[mise]: https://mise.jdx.dev
[rakko-update]: https://github.com/aonyx-ai/rakko-update
[renovate]: https://docs.renovatebot.com
[tools-and-mise]: @/explanation/tools-and-mise.md
