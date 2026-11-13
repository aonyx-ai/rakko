# Aonyx Rakko Nextest

[Rakko] turns project maintenance into versioned Rust crates, and an action is
a crate that does one maintenance task. `aonyx-rakko-nextest` holds the code
that the actions which run the tests of a project with [nextest] share. Each of
these actions asks the same questions. Which command does a run write? What
did nextest and cargo report? This crate answers them, so an action gives the
workspace that it wants tested and reads the answer as data.

The crate pins the version of nextest that every such action runs. The version
that the project pins does not change what runs.

## Documentation

The [documentation of the crate][docs] on docs.rs describes a run and its
report.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

[docs]: https://docs.rs/aonyx-rakko-nextest
[nextest]: https://nexte.st
[rakko]: https://github.com/aonyx-ai/rakko
[support]: https://github.com/aonyx-ai/rakko#support
