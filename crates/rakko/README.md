# Rakko

`rakko` is the contract of [Rakko], which turns project maintenance into
versioned Rust crates. A maintenance task, such as the check of the TOML files
of a project, is a crate called an action. This crate holds the trait that an
action implements, and the types that a run uses: the arguments, the context,
the findings, and the outcome.

A group is a named list of actions that runs as one command, such as the
actions that run before a commit. A bundle is a crate that exports a list of
actions. This crate also holds the trait of a group and the list type of a
bundle.

The crate of an action or of a group depends on this crate. It also depends
on the helper crates that it uses, such as `rakko-tool`, but not on
`rakko-cli`. This crate builds no command line, so the command-line framework
stays out of the build of an action.

## Harness

A harness is the small binary that a project runs to maintain itself. It
mounts the actions that the project uses. To write a harness, start with
[`rakko-cli`][rakko-cli], which turns the mounted actions into a command line.
A harness also depends on `rakko` in two cases. The first case is a harness
that writes an action or a group of its own. The second case is a harness that
builds a list of its own, such as the list of a group.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each trait and
each type.

## Support

Aonyx uses Rakko internally, and we maintain and support this crate. The
section [Support] in the README of the repository lists the crates that Aonyx
supports for other authors.

[docs]: https://docs.rs/rakko
[rakko]: https://github.com/aonyx-ai/rakko
[rakko-cli]: https://crates.io/crates/rakko-cli
[support]: https://github.com/aonyx-ai/rakko#support
