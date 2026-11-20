# `aonyx-rakko-require-changesets`

This crate provides `require-changesets`, a Rakko action that fails a pull
request that changes a release unit of Otto without a changeset.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Otto] builds the changelog of a release from changesets. A changeset is one
file for each change in the `.changeset` directory of a release unit. A
release unit is a crate or a workspace that Otto releases with one version,
such as the crate in `crates/rakko` of the Rakko repository. A change that a
pull request does not describe is missing from the changelog and from the
release notes. The action fails a pull request that changes a unit and adds
no changeset to it. The author can opt a unit out with a line in the body of
the pull request that gives a reason:

```text
No changeset for <unit>: <reason>
```

One line opts out every unit of the pull request, so that a refactor across
many crates needs no line per crate:

```text
No changeset: <reason>
```

A line of one of these two forms without a reason fails the check. A line for
a unit that does not exist fails the check as well. A passing check shows each
unit that it accepted without a changeset, with its reason.

Otto opens the pull request of a release from the branch `release/<unit>`.
Such a pull request passes when its branch is in the repository of the pull
request and not in a fork. A project where Otto lists no release unit has
nothing to check, so the run skips.

Otto finds the release units, and the action runs `otto list-units`. A unit
counts as changed when the pull request changes a file that the unit owns.
Otto gives the rule: the manifest of a unit belongs to that unit. Any other
file belongs to the unit of the nearest crate directory above it. A fixture
below a crate, which Otto lists as an unowned directory, belongs to no unit.
For example, `crates/rakko/src/lib.rs` belongs to the unit `rakko`, and
`Cargo.lock` at the top of the repository belongs to no unit. So a change to
`.github/` alone changes no unit, when no crate sits at the top of the
repository.

The action pins the version of Otto that it runs, and mise builds Otto from
source. The pin leaves Windows out, so a run on Windows skips.

The action reads the pull request from the event file of GitHub Actions. A run
outside a pull request, such as one on the machine of a contributor, skips
with a reason.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add
`aonyx-rakko-require-changesets` to the harness of the project. Then mount the
action in the `main` of the harness:

```rust
use aonyx_rakko_require_changesets::RequireChangesets;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(RequireChangesets).run()
}
```

The action becomes the `require-changesets` command of the harness. A bundle
does not export it, because a project opts into the check.

The check needs a workflow of its own. A job for the event of a pull request
reads the body as it was at the time of the event, also in a re-run.
So the workflow listens for the `edited` event as well, and an edit of the body
runs the check again. The checkout needs the history of the base and of the
head, which `fetch-depth: 0` of `actions/checkout` gives:

```yaml
name: Changesets

"on":
  pull_request:
    types: [opened, edited, reopened, synchronize]

permissions: {}

concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number }}
  cancel-in-progress: true

jobs:
  require-changesets:
    name: require-changesets
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: jdx/mise-action@90f833d396d0cc3674b631d2d225e306476c303a # v5.2.10
      - run: mise run rakko -- require-changesets
```

The opt-out lines must stay out of the history. So the squash merge of the
repository must build the commit message from the commits and not from the
body of the pull request. On GitHub, this is the value `COMMIT_MESSAGES` of
the setting `squash_merge_commit_message`.

## Asynchronous Runtime

A run starts programs and waits for them. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `RequireChangesets`][docs] on docs.rs describes what a
run checks, what it reports, and when it stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/aonyx-rakko-require-changesets/latest/aonyx_rakko_require_changesets/struct.RequireChangesets.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[otto]: https://github.com/aonyx-ai/otto
[rakko]: https://rakko.aonyx.ai
[tokio]: https://tokio.rs
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
