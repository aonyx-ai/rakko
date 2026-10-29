# Hosted Renovate and Regenerated Files

Research for [rakko#448][issue-448] on the map [rakko#442][map-442], done on
2026-10-29. The Renovate documentation was read on its `main` branch, and the
Renovate source at the tag [44.195.1][renovate-tag], the latest release on
that day.

The question: a Renovate pull request moves the Git revision of Rakko in a
harness. When the new revision changes the version of a tool, the same pull
request must regenerate `.config/mise/conf.d/rakko.toml` and
`.config/mise/mise.lock`. That takes the harness (Rust) and `mise lock`
(network). Can the hosted Renovate app run that, and if not, what else can?

## Answer in Brief

- The hosted app has `postUpgradeTasks`, but it runs no command unless
  `allowedCommands` permits it, and only Mend sets that list. On the free
  Community plan, a repository can neither set nor request it. On the
  Community (OSS) plan, a trusted open source project can ask Mend to allow
  one command. On the paid Enterprise Cloud and Mend AppSec plans, a
  repository or an organization sets `RENOVATE_ALLOWED_COMMANDS` itself.
- `allowedCommands` is the current name. It was `allowedPostUpgradeCommands`
  until Renovate [PR 32657][pr-32657] renamed it in January 2025.
- A workflow that commits onto a Renovate branch needs a GitHub App token
  (or a personal access token), so that CI runs on its commit without a
  human approval. It needs `gitIgnoredAuthors`, or Renovate stops updating
  the branch. It also needs a check that fails while the files are stale,
  or auto-merge can merge the first commit before the workflow pushes.
- A self-hosted Renovate on the persistent self-hosted runners runs
  `postUpgradeTasks` with our own `allowedCommands`. It needs its own GitHub
  App, the hosted app turned off on those repositories, and someone to run
  and upgrade it.
- Renovate's own managers already run `cargo` and `mise lock` on the hosted
  app. The mise manager reads `.config/mise/conf.d/*.toml` and writes
  `.config/mise/mise.lock`, the two files of the design. So a version change
  that Renovate itself makes in that file needs no custom command. Renovate
  cannot add or remove a tool there, though.

## Post-Upgrade Tasks on the Hosted App

The [`postUpgradeTasks`][opt-post-upgrade] option holds commands that Renovate
runs after it updates a dependency and before it commits. Each command must
match a pattern of [`allowedCommands`][opt-allowed-commands], a global option
that only the admin of the Renovate instance sets. An empty list runs
nothing. The option docs add that on the Mend-hosted apps commands "remain
blocked by default".

The [Mend-hosted FAQ][mend-faq] gives the rule per plan:

| Plan                             | Can a repository run a post-upgrade command?                                              |
| -------------------------------- | ----------------------------------------------------------------------------------------- |
| Community (Free)                 | No. Users "cannot modify nor request arbitrary commands".                                 |
| Community (OSS)                  | Only after a Mend Hosted Request on GitHub Discussions, at Mend's discretion.             |
| Enterprise Cloud and Mend AppSec | Yes, with `RENOVATE_ALLOWED_COMMANDS`, set by a repository admin or for the organization. |

The [environment variables page][mend-env] confirms that only paying
customers get `RENOVATE_ALLOWED_COMMANDS`, next to
`RENOVATE_ALLOW_SHELL_EXECUTOR_FOR_POST_UPGRADE_COMMANDS` and
`RENOVATE_ALLOWED_UNSAFE_EXECUTIONS`. Free and OSS users can set only
repository configuration this way.

Facts that limit the OSS route for this fleet:

- The OSS plan is for projects under an OSI-approved license
  ([overview][mend-overview]). At least eight of the 13 repositories with a
  harness are private (den, skills, otto, aonyx, orchestrator, repository,
  aonyx.ai, and feature-kit, from `gh repo list aonyx-ai`).
- The plan that aonyx-ai is on was not checked. Nothing in the repositories
  shows it.

Facts that limit any hosted run of the harness, even with a command allowed:

- A hosted job gets 1 vCPU, 3 GB of memory, and a 30 minute timeout on the
  free plan, and 1 vCPU, 6 GB, and 60 minutes on the OSS plan
  ([overview][mend-overview]). The command would build the harness from
  nothing in each job.
- Tools for a command come from
  [`postUpgradeTasks.installTools`][opt-install-tools], which installs
  Containerbase tools. The harness needs Rust, mise, and the network for
  `mise lock`.

## What Renovate Already Regenerates on the Hosted App

Renovate's managers run their own lock commands, outside of
`postUpgradeTasks`. The fleet already relies on two of them:

- The cargo manager updates `Cargo.lock` when the Rakko revision moves.
- The mise manager runs `mise lock <tools>` after it changes a mise file
  ([`artifacts.ts`][src-mise-artifacts]). Without `mise` in
  `allowedUnsafeExecutions`, it runs mise in safe mode (`MISE_SAFE=1`), so
  the hosted app can run it. The den commit [89be07a][den-89be07a] by
  `renovate[bot]` shows it: Renovate moved Node.js in `mise.toml` and wrote
  new checksums into `mise.lock`.

The mise manager matches `**/.config/mise/conf.d/*.toml` by default
([`index.ts`][src-mise-index]). For a file in `conf.d`, it writes the lock
file of the parent directory ([`lockfile.ts`][src-mise-lockfile]), so
`.config/mise/conf.d/rakko.toml` maps to `.config/mise/mise.lock`.

Two consequences:

- Unless the preset stops it, the mise manager moves each tool in the
  generated file to its newest upstream release, apart from Rakko. That
  goes against the decision of [rakko#445][issue-445] that the action owns
  the version.
- When the manager writes the version that Rakko wants, the hosted app
  regenerates both files with no custom command. See Alternative 3.

## How Renovate Treats a Commit by Someone Else

[`isBranchModified`][src-is-modified] collects the author and the committer
email of every commit on the branch since the base branch. The branch counts
as modified if any email is not Renovate's own `gitAuthor`, not in
`gitIgnoredAuthors`, and not `noreply@github.com`, which the GitHub platform
ignores ([`github/index.ts`][src-gh-ignored]). The hosted app commits as
`29139614+renovate[bot]@users.noreply.github.com` ([rakko PR 441][pr-441]).

On a modified branch, Renovate ([`gitIgnoredAuthors`][opt-ignored-authors],
[`handle-existing.ts`][src-handle-modified]):

- makes no further commits, not for a conflict and not for a newer version;
- comments that it "will not automatically rebase this PR", and that a
  manual rebase through the checkbox loses the custom changes;
- refuses its own automerge with the reason `BranchModified`
  ([`automerge.ts`][src-pr-automerge]). GitHub's own auto-merge does not
  look at this check, see Automerge in the Fleet below.

With the other bot's email in `gitIgnoredAuthors`, the branch counts as
unmodified, so Renovate keeps working on it:

- `rebaseWhen` defaults to `auto`, which becomes `behind-base-branch` when
  `automerge` is on ([`reuse.ts`][src-reuse], [`rebaseWhen`][opt-rebase-when]).
  The shared preset sets `"automerge": true`. So Renovate rebases the branch
  each time `main` moves.
- Each new Renovate commit starts from `origin/<base>` and holds only the
  files that Renovate changed ([`prepareCommit`][src-prepare-commit]), and
  Renovate force-pushes it. The other bot's commit is gone after every
  rebase or version bump, and the bot has to commit again.
- The [`gitAuthor`][opt-git-author] docs warn about exactly this: Renovate
  "will mistake the branch as unmodified and potentially force push over the
  changes".

`gitIgnoredAuthors` is repository configuration, so the shared preset can
carry it for the whole fleet.

## Automerge in the Fleet

- The preset sets `automerge: true`, and `platformAutomerge` defaults to
  `true`. Renovate turns on GitHub's auto-merge when it opens the pull
  request ([rakko PR 441][pr-441]: by `app/renovate`, squash), and turns it on
  again after each rebase ([`platformAutomerge`][opt-platform-automerge]).
- GitHub merges "after all required reviews and status checks pass", and
  turns auto-merge off when "someone without write permissions pushes new
  changes to the head branch" ([GitHub auto-merge][gh-auto-merge]). A GitHub
  App with write access to contents keeps it on.
- The ruleset on rakko `main` requires the check `All checks succeeded` and
  zero approvals, and does not require the branch to be up to date
  (`strict_required_status_checks_policy: false`). The other fleet
  repositories were not checked.

## Alternative 1: A Workflow That Commits on Renovate Branches

What it needs:

- **A token that starts CI.** A push with `GITHUB_TOKEN` starts no `push`
  run. Its `pull_request` `synchronize` run waits in an approval-required
  state until a user with write access approves it
  ([GitHub triggers][gh-trigger], [changelog][gh-changelog-approval]). That
  stops auto-merge until a human acts. A GitHub App installation token, or a
  personal access token, starts the runs with no approval. The App needs
  write access to contents, and the workflow needs the App ID and the
  private key as secrets.
- **`gitIgnoredAuthors`** with the email that the workflow commits as. For a
  GitHub App, that is `<user id>+<slug>[bot]@users.noreply.github.com`, where
  the user id comes from `api.github.com/users/<slug>[bot]`
  ([Renovate GitHub docs][platform-github]). Without it, the first commit of
  the workflow freezes the pull request at its revision: no rebase and no
  newer Rakko revision until it merges or a human ticks the checkbox.
- **A check that fails while the files are stale.** Renovate's commit starts
  CI and the workflow at the same time. If `All checks succeeded` passes on
  Renovate's commit before the workflow pushes, GitHub merges the stale
  commit. A failing check on the first commit, or the regeneration as a job
  that the required check waits on, closes that gap. The `concurrency`
  group of `ci.yml` cancels the run on the old commit once the new one
  arrives, but that alone does not close it.
- **A workflow that stops when nothing changes.** Its own App-token push
  starts it again, and the second run must find nothing to commit.
- **A runner.** The workflow can run on the persistent self-hosted runners,
  which already carry mise and Rust and reach the network.

What happens to automerge: it stays on, because the App can write. The pull
request merges once `All checks succeeded` passes on the regenerated
commit. With `gitIgnoredAuthors`, every rebase drops the regenerated commit
and the workflow adds it back, so each move of `main` costs one more CI run.

## Alternative 2: Self-Hosted Renovate on the Persistent Runners

What it needs:

- **A GitHub App** with the repository permissions that the
  [Renovate GitHub docs][platform-github] list (among them Contents, Pull
  requests, Checks, Commit statuses, Issues, and Workflows read and write),
  or a personal access token. An installation token expires after one hour,
  so each run creates one, for example with `actions/create-github-app-token`
  ([github-action README][renovate-action]).
- **Global configuration** that only the self-hosted instance has:
  `allowedCommands` with a regular expression per command
  ([`allowedCommands`][opt-allowed-commands]), and `mise` in
  `allowedUnsafeExecutions` if mise has to run outside safe mode
  ([`allowedUnsafeExecutions`][opt-unsafe]). To use the tools of the runner
  instead of Containerbase downloads, `binarySource` is `global`
  ([`binarySource`][opt-binary-source]).
- **Repository configuration**: `postUpgradeTasks` with the commands,
  `fileFilters` for the two files, and `executionMode: branch`, so it runs
  once per branch ([`postUpgradeTasks`][opt-post-upgrade]).
- **A way to run it.** `renovatebot/github-action` runs Renovate in a
  container image, so the runner needs Docker
  ([github-action README][renovate-action]). The fleet already pins
  `npm:renovate` in `mise.toml`, so the CLI can also run directly on the
  runner. Whether the runners have Docker was not checked.
- **One Renovate per repository.** The github-action README asks for a
  different `branchPrefix` so that a self-hosted Renovate does not interfere
  with the Renovate app. With both bots on one repository, each would still
  open its own pull request for the same update, so the hosted app would
  have to leave those repositories.

What it changes:

- Renovate makes the regenerated commit itself, so `gitIgnoredAuthors` is
  not needed, and rebases keep the files.
- The App token starts CI. Automerge works as it does today.
- Mend no longer keeps the tokens safe and the Renovate version current
  ([Renovate GitHub docs][platform-github]). The hosted app also starts jobs
  from webhooks, such as a ticked rebase checkbox ([overview][mend-overview]).
  A self-hosted Renovate on a schedule reacts at its next run.
- A post-upgrade command builds and runs code from the updated dependency.
  Renovate's docs call this the risk of an "outsider attack"
  ([security][security-outsider]). Here the updated dependency is Rakko
  itself.

## Alternative 3: Files That Need No Command

Option A: Renovate writes the new versions itself.

- The mise manager can look a tool up somewhere else with
  `packageRules` and [`overrideDatasource`][opt-override-datasource], as the
  preset already does for oxlint. A [custom datasource][custom-datasource]
  reads `html`, `json`, `plain`, `toml`, or `yaml` from a URL, as the preset
  already does for the Rust nightly.
- If Rakko published the version of each tool at a URL, a rule for
  `.config/mise/conf.d/rakko.toml` could read the versions from there. The
  mise manager would then run `mise lock` and write `.config/mise/mise.lock`
  on the hosted app. A `groupName` of `rakko` puts these updates in the same
  pull request as the revision.
- Limits: Renovate changes only the version of a tool that is already in the
  file. A new or removed tool still needs the harness and a human. The URL
  points to a branch of Rakko, not to the pinned revision, so the two only
  match when Renovate looks both up in the same run. The text that Renovate
  writes must match what the harness generates. This path was not tried end
  to end.

Option B: the file is not committed. The harness writes it at run time, and
a Rakko update then changes only `Cargo.toml` and `Cargo.lock`, which the
hosted app already handles. Nothing in the project then locks those tools,
unless Rakko carries the checksums itself. This is a design consequence, not
something this research checked.

## Facts for the Decision

- The free hosted app cannot run the harness or `mise lock` as a custom
  command, and no setting changes that. A paid plan, or Mend's consent per
  open source project, can.
- Keeping the hosted app and adding a workflow costs a GitHub App, a
  `gitIgnoredAuthors` entry in the preset, a stale-files check in CI, and one
  extra CI run per rebase.
- A self-hosted Renovate costs a GitHub App, a schedule on the runners, and
  the upkeep that Mend does today, and moves all of the fleet's updates off
  the hosted app.
- The hosted app already regenerates `.config/mise/mise.lock` when a Renovate
  manager changes a version in `conf.d`. That route covers version changes
  only, not added or removed tools.

[custom-datasource]: https://docs.renovatebot.com/modules/datasource/custom/
[den-89be07a]: https://github.com/aonyx-ai/den/commit/89be07a
[gh-auto-merge]: https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/automatically-merging-a-pull-request
[gh-changelog-approval]: https://github.blog/changelog/2026-06-11-bot-created-pull-requests-can-run-workflows-if-approved/
[gh-trigger]: https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow#triggering-a-workflow-from-a-workflow
[issue-445]: https://github.com/aonyx-ai/rakko/issues/445
[issue-448]: https://github.com/aonyx-ai/rakko/issues/448
[map-442]: https://github.com/aonyx-ai/rakko/issues/442
[mend-env]: https://docs.renovatebot.com/mend-hosted/environment-variables/
[mend-faq]: https://docs.renovatebot.com/mend-hosted/faq/#how-can-i-run-arbitrary-commands-through-postupgradetasks
[mend-overview]: https://docs.renovatebot.com/mend-hosted/overview/
[opt-allowed-commands]: https://docs.renovatebot.com/self-hosted-configuration/#allowedcommands
[opt-binary-source]: https://docs.renovatebot.com/self-hosted-configuration/#binarysource
[opt-git-author]: https://docs.renovatebot.com/configuration-options/#gitauthor
[opt-ignored-authors]: https://docs.renovatebot.com/configuration-options/#gitignoredauthors
[opt-install-tools]: https://docs.renovatebot.com/configuration-options/#postupgradetasksinstalltools
[opt-override-datasource]: https://docs.renovatebot.com/configuration-options/#packagerulesoverridedatasource
[opt-platform-automerge]: https://docs.renovatebot.com/configuration-options/#platformautomerge
[opt-post-upgrade]: https://docs.renovatebot.com/configuration-options/#postupgradetasks
[opt-rebase-when]: https://docs.renovatebot.com/configuration-options/#rebasewhen
[opt-unsafe]: https://docs.renovatebot.com/self-hosted-configuration/#allowedunsafeexecutions
[platform-github]: https://docs.renovatebot.com/modules/platform/github/
[pr-32657]: https://github.com/renovatebot/renovate/pull/32657
[pr-441]: https://github.com/aonyx-ai/rakko/pull/441
[renovate-action]: https://github.com/renovatebot/github-action#readme
[renovate-tag]: https://github.com/renovatebot/renovate/releases/tag/44.195.1
[security-outsider]: https://docs.renovatebot.com/security-and-permissions/#execution-of-code-outsider-attack
[src-gh-ignored]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/modules/platform/github/index.ts#L272
[src-handle-modified]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/workers/repository/update/branch/handle-existing.ts#L53-L95
[src-is-modified]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/util/git/index.ts#L983-L1060
[src-mise-artifacts]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/modules/manager/mise/artifacts.ts
[src-mise-index]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/modules/manager/mise/index.ts#L35-L46
[src-mise-lockfile]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/modules/manager/mise/lockfile.ts#L26-L48
[src-pr-automerge]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/workers/repository/update/pr/automerge.ts#L96-L101
[src-prepare-commit]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/util/git/index.ts#L1407-L1424
[src-reuse]: https://github.com/renovatebot/renovate/blob/44.195.1/lib/workers/repository/update/branch/reuse.ts#L121-L150
