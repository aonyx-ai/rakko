# Aonyx Rakko Shfmt

[Rakko] turns project maintenance into versioned Rust crates, and an action is a
crate that does one maintenance task. `aonyx-rakko-shfmt` holds the code that
the actions which run [shfmt] share. Shfmt formats shell scripts, and it also
finds the shell scripts of a project, by their extension and by their shebang.
An action that formats the scripts and an action that examines them both ask
the same questions. Which files are shell scripts? Which shfmt does the project
run? What did that shfmt report? This crate answers them, so an action gives
the operation that it wants and reads the answer as data.

The crate pins the version of shfmt that every such action runs. The version
that the project pins does not change what runs.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each operation and
its report.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

[docs]: https://docs.rs/aonyx-rakko-shfmt
[rakko]: https://github.com/aonyx-ai/rakko
[shfmt]: https://github.com/mvdan/sh
[support]: https://github.com/aonyx-ai/rakko#support
