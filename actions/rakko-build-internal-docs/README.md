# Rakko Build Internal Docs

`rakko-build-internal-docs` provides `build-internal-docs`, an action for
[Rakko] that builds the internal documentation of a project.

[Rustdoc] does the work. Cargo reads the manifests of the project and renders
the documentation of every package, with the private items and with every
feature. So a run of the action agrees with a contributor that runs `cargo doc`
bare. The action starts the cargo that [mise] installed for the project. It uses
the version that the project pinned and translates what cargo reported into the
outcome of the run.

The internal documentation describes the code to whoever maintains it. The
documentation that a project publishes for the people who use it is a different
task, with a tool of its own.

Building the documentation is also the only examination that it gets. Rustdoc
resolves the links between items while it renders them, and a link that names
nothing is a warning that no other tool reports. Every diagnostic becomes a
finding, so a run with a warning fails.

A run takes no argument. The action applies to a project that holds a manifest
of cargo, and it skips visibly otherwise.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-build-internal-docs` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_build_internal_docs::BuildInternalDocs;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(BuildInternalDocs).run()
}
```

The action becomes the `build-internal-docs` command of the harness.

## Documentation

The [documentation of `BuildInternalDocs`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/rakko-build-internal-docs/latest/rakko_build_internal_docs/struct.BuildInternalDocs.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[rustdoc]: https://doc.rust-lang.org/rustdoc/
