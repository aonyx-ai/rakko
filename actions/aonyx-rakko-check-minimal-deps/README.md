# `aonyx-rakko-check-minimal-deps`

This crate provides `check-minimal-deps`, a Rakko action that tests Rust
projects with their lowest dependency versions.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

A manifest states a floor for each dependency, and a floor is a promise: that
version of the dependency works. An ordinary build reaches for the newest
version that the floor allows and never touches the floor itself. So the action
resolves the floors and runs the tests against them.

[Cargo] resolves and [nextest] tests. Nextest runs at the version that the
action declares, which [mise] installed. The resolution runs on the nightly
toolchain that the project pins, because the option that asks for the floors is
unstable. The tests run on the toolchain that the project builds with. The whole
run happens in a disposable copy of the project. A resolution rewrites a
lockfile, so the checkout of a contributor is not the place for it.

A run only reports, and it takes no argument. An update that cargo did not
finish becomes a finding at the manifest of its workspace. A test that failed
becomes a finding at the position where it panicked. A build that does not
finish becomes findings from the diagnostics of the compiler. A project whose
cargo discovers no workspace skips visibly, and says so.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add
`aonyx-rakko-check-minimal-deps` to the harness of the project. Then mount the
action in the `main` of the harness:

```rust
use aonyx_rakko_check_minimal_deps::CheckMinimalDeps;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckMinimalDeps).run()
}
```

The action becomes the `check-minimal-deps` command of the harness.

## Documentation

The [documentation of `CheckMinimalDeps`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

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

[cargo]: https://doc.rust-lang.org/cargo/
[docs]: https://docs.rs/aonyx-rakko-check-minimal-deps/latest/aonyx_rakko_check_minimal_deps/struct.CheckMinimalDeps.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
