# Aonyx Rakko Check Unused Deps

`aonyx-rakko-check-unused-deps` provides `check-unused-deps`, an action for
[Rakko] that finds the dependencies a project declares and never uses.

A dependency that no target uses still has a cost for the project. It is built,
it is audited, and it is updated. It also widens the surface that a reader of
the manifest has to understand. Nothing in the manifest says whether a
dependency is used, so the answer comes from a build.

[Cargo-udeps] does the work. It reads the record that the compiler writes about
the crates each target actually loaded. It holds that record against the
dependencies of the manifest. The action declares the version of cargo-udeps
that it runs. It starts that version, which [mise] installed, on the nightly
toolchain that the project pins, and it installs nothing. The nightly channel is
not a preference: the record comes from an unstable option of the compiler, and
a stable toolchain refuses it.

A run only reports, and it takes no argument. Every unused dependency becomes a
finding at the manifest that declares it. The action applies to a project that
holds a manifest of cargo, and it skips visibly otherwise.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-check-unused-deps`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_check_unused_deps::CheckUnusedDeps;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckUnusedDeps).run()
}
```

The action becomes the `check-unused-deps` command of the harness.

## Documentation

The [documentation of `CheckUnusedDeps`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[cargo-udeps]: https://github.com/est31/cargo-udeps
[docs]: https://docs.rs/aonyx-rakko-check-unused-deps/latest/aonyx_rakko_check_unused_deps/struct.CheckUnusedDeps.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
