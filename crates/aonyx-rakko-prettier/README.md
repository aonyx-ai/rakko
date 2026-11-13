# Aonyx Rakko Prettier

[Rakko] turns project maintenance into versioned Rust crates, and an action is a
crate that does one maintenance task. `aonyx-rakko-prettier` holds the code that
the actions which wrap [prettier] share. Prettier formats many languages, and
one action wraps each set of files that a project keeps apart, such as the YAML
files. Each of these actions asks the same questions. Does the project hold
files of the set? Which prettier does it run? What did that prettier report?
This crate answers them. So an action gives the operation and the files that it
wants, and reads the answer as data.

The crate pins the version of prettier that every such action runs. The
version that the project pins does not change what runs.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each operation and
its report.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

[docs]: https://docs.rs/aonyx-rakko-prettier
[prettier]: https://prettier.io
[rakko]: https://github.com/aonyx-ai/rakko
[support]: https://github.com/aonyx-ai/rakko#support
