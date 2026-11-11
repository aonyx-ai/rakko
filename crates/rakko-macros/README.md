# Rakko Macros

`rakko-macros` holds the derive macros of the contract of [Rakko], such as
`Documented`. An action is a crate that does one maintenance task. A command
and a group are other parts that a harness mounts. `Documented` takes the
sentence that describes an action, a command, or a group from the first
paragraph of its doc comment.

The [`rakko`][rakko-crate] crate exports every macro of this crate again, and
the code that a macro writes refers to items of `rakko`. So an action, a
command, or a group depends on `rakko`, and never on this crate.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each macro.

## Support

Use this crate through `rakko`. The section [Support] in the README of the
repository lists `rakko` among the crates that Aonyx supports for other
authors.

[docs]: https://docs.rs/rakko-macros
[rakko]: https://github.com/aonyx-ai/rakko
[rakko-crate]: https://crates.io/crates/rakko
[support]: https://github.com/aonyx-ai/rakko#support
