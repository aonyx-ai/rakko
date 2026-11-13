# Aonyx Rakko Check Crate Owners

`aonyx-rakko-check-crate-owners` provides `check-crate-owners`, an action for
[Rakko] that checks that each crate of a project has every owner that it needs
on [crates.io].

A crate that one person owns depends on that person. When their account
becomes unavailable, nobody can publish the crate, change its owners, or set up
trusted publishing. The action compares the owners of each crate with a list of
owners, and it fails when a crate misses one of them. With `fix`, it adds each
missing owner.

The action checks each crate of each workspace of the project that cargo can
publish to crates.io, as its manifest says. A manifest without the `publish` key
allows cargo to publish the crate, and so does a list of registries that holds
`crates-io`. The action asks cargo for those crates, and it reads their owners
from the public API of crates.io, which needs no token. So a run in CI needs no
secret. The action skips a crate that crates.io does not have yet. A run that
passes gives the skipped crates in its summary, and a run that skips gives them
in its reason. A new crate stays off crates.io until cargo publishes it for the
first time.

The action reports only a missing owner. An owner that crates.io lists, but that
the list of the action does not hold, is no finding. A repair never removes an
owner.

crates.io adds a team as an owner at once. It sends a user an invitation, and it
lists the user only after they accept it. So a user who did not accept the
invitation yet still counts as missing.

A run that reports gives the `cargo owner --add` command that adds each missing
owner. With `fix`, the run runs that command, with the credentials that cargo
has for the user that starts the run. crates.io has two kinds of owners, users
and teams, and only a user owner of a crate can add owners. For any other user,
crates.io refuses the change, and the report says that a user owner of the crate
must run the fix. So CI reports, and an owner runs the fix on their own machine.

crates.io asks an automated client to send at most one request per second. So
the action waits one second before each lookup and before each run of
`cargo owner --add`, and a run takes about one second per crate.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add
`aonyx-rakko-check-crate-owners` to the harness of the project. Then mount the
action in the `main` of the harness, with the owners that every crate must
have:

```rust
use aonyx_rakko_check_crate_owners::{CheckCrateOwners, CrateOwner};

fn main() -> std::process::ExitCode {
    let owners = [
        CrateOwner::new("jdno"),
        CrateOwner::new("github:aonyx-ai:crates-io"),
    ];

    rakko_cli::builder()
        .mount(CheckCrateOwners::new(owners))
        .run()
}
```

The action becomes the `check-crate-owners` command of the harness. It needs
the network, so a project runs it in a job of CI and not before every commit.

## Asynchronous Runtime

A run starts a program, sends requests, and waits for them. A [Tokio] runtime
drives that. The command line that `rakko-cli` builds starts one, so a harness
needs no runtime of its own. A caller that runs the action outside of that
command line starts a runtime with its timer first, because the run panics
without one.

## Documentation

The [documentation of `CheckCrateOwners`][docs] on docs.rs describes what a run
checks, what it reports, and when it stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[crates.io]: https://crates.io
[docs]: https://docs.rs/aonyx-rakko-check-crate-owners/latest/aonyx_rakko_check_crate_owners/struct.CheckCrateOwners.html
[guide]: https://github.com/aonyx-ai/rakko#support
[rakko]: https://github.com/aonyx-ai/rakko
[tokio]: https://tokio.rs
