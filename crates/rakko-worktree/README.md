# Rakko Worktree

[Rakko] turns project maintenance into versioned Rust crates, and an action is
a crate that does one maintenance task. `rakko-worktree` gives an action a
disposable copy of a project to write in. Some work writes files that nobody
asked for: a resolution of dependencies rewrites a lockfile, and a build
writes what it generates. A check that does such work in the checkout of a
contributor leaves the tree dirty.

The copy is a Git worktree of the commit at the HEAD of the project. The crate
syncs the changed files of the checkout over it. It also removes from the copy
each file that the checkout removed. So a job that runs in the copy gives the
result for the tree of the contributor, and the checkout does not change. A
drop of the value removes the copy.

## Documentation

The [documentation of the crate][docs] on docs.rs describes the copy and what
it requires of the project.

## Support

Aonyx built this crate for its own actions, and the crate mainly serves them.
You are welcome to use it in your own actions. The crates that Aonyx supports
for other authors are those that the section [Support] in the README of the
repository lists.

[docs]: https://docs.rs/rakko-worktree
[rakko]: https://github.com/aonyx-ai/rakko
[support]: https://github.com/aonyx-ai/rakko#support
