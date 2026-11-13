# Aonyx Rakko Check Latest Deps

`aonyx-rakko-check-latest-deps` provides `check-latest-deps`, an action for
[Rakko] that checks a project against the newest versions of its dependencies.

A manifest states a floor for each dependency, and a floor is a promise: every
later version of that dependency works as well. The action resolves the newest
versions that the manifests allow and runs the tests against them, so the
promise is confirmed instead of assumed.

[Cargo] resolves and [nextest] tests. Cargo runs on the stable toolchain that
the project pins, and nextest at the version that the action declares, which
[mise] installed. The whole run happens in a disposable copy of the project. A
resolution rewrites a lockfile, so the checkout of a contributor is not the
place for it.

A run only reports, and it takes no argument. An update that cargo did not
finish becomes a finding at the manifest of its workspace. A test that failed
becomes a finding at the position where it panicked. A build that does not
finish becomes findings from the diagnostics of the compiler. A project whose
cargo discovers no workspace skips visibly, and says so.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-check-latest-deps`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_check_latest_deps::CheckLatestDeps;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckLatestDeps).run()
}
```

The action becomes the `check-latest-deps` command of the harness.

## Documentation

The [documentation of `CheckLatestDeps`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[cargo]: https://doc.rust-lang.org/cargo/
[docs]: https://docs.rs/aonyx-rakko-check-latest-deps/latest/aonyx_rakko_check_latest_deps/struct.CheckLatestDeps.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rakko]: https://github.com/aonyx-ai/rakko
