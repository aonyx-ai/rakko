# Aonyx Rakko Cargo

[Rakko] turns project maintenance into versioned Rust crates, and an action is
a crate that does one maintenance task. `aonyx-rakko-cargo` holds the code that
the actions which wrap [cargo] share. Each of these actions wraps one job of
cargo, such as a run of clippy, and each asks the same questions. Does the
project hold Rust code? Which cargo does it run? Which workspaces make up the
project? Which toolchain does the job need? What did cargo report about the
build? This crate answers them, so an action writes the command line of its
job and reads the answer as data.

## Documentation

The [documentation of the crate][docs] on docs.rs describes each question and
its answer.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

[cargo]: https://doc.rust-lang.org/cargo/
[docs]: https://docs.rs/aonyx-rakko-cargo
[rakko]: https://github.com/aonyx-ai/rakko
[support]: https://github.com/aonyx-ai/rakko#support
