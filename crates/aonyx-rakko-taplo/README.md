# Aonyx Rakko Taplo

[Rakko] turns project maintenance into versioned Rust crates, and an action is a
crate that does one maintenance task. `aonyx-rakko-taplo` holds the code that
the actions which wrap [taplo] share. Taplo formats and lints TOML files, and
one action wraps each of these jobs. Each of these actions asks the same
questions. Does the project hold TOML files? Which taplo does it run? What did
that taplo report? This crate answers them, so an action gives the operation
that it wants and reads the answer as data.

The crate pins the version of taplo that every such action runs. The version
that the project pins does not change what runs.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each operation and
its report.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

[docs]: https://docs.rs/aonyx-rakko-taplo
[rakko]: https://github.com/aonyx-ai/rakko
[support]: https://github.com/aonyx-ai/rakko#support
[taplo]: https://taplo.tamasfe.dev
