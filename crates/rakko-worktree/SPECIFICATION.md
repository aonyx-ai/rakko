# Rakko Worktree

`rakko-worktree` gives an action a disposable copy of the project to work in.
An action whose writes are incidental — a lockfile that a resolution rewrites,
a file that a build generates — runs its jobs in that copy, and the checkout of
the contributor stays as they left it.

The copy is a git worktree: a detached checkout of the commit that the HEAD of
the project names, in a directory of the temporary directory of the system,
with the changed files of the checkout synced over it. An action that runs
there answers for the working tree of the contributor, which is what every
other action answers for. A contributor who raises a version floor in a
manifest and runs the check before they commit therefore reads the answer for
the new floor, and a dirty tree is no reason to stop the run.

One rule shapes the crate: nothing touches the checkout of the contributor. No
temporary commit, no stash, no applied patch. The crate writes into the
repository, where a worktree shows up in `git worktree list` while the run
lasts, and it writes into a directory of its own. A failure or an interrupt
leaves at most a directory that the operating system removes and an entry that
`git worktree prune` forgets, and neither of those can dirty the project.

The crate is not the sandbox. It gives an action a place to write, and it
enforces nothing about what the action writes there.

Git is a requirement of every action that uses the crate. Git is infrastructure
and not a tool that an action wraps, so the crate finds it with the rules of
the platform, and a git that does not run stops the caller with an error.

The crate does not sync a submodule. Git reports a submodule whose checkout
moved as one path that names a directory, and the crate leaves such a path at
the revision that the HEAD of the project names. It also copies a symbolic link
as a link on a platform that creates one without a privilege, and as the file
that the link names on every other platform.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key words
MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Repository

A worktree is a checkout of a repository, so a project without one has no copy
that the crate can give. A project marks its root with a file of its own and
needs no repository otherwise, so the failure belongs to the crate and not to
the discovery of the project, and the error names the condition instead of the
step that ran into it.

The root of the project is the top level of the repository. A project below the
top level would need a checkout of the whole repository and a sync of paths
that belong to no project, which is more than the crate promises, so it stops
with an error that names both directories.

worktree[repository.missing]
The crate MUST report an error that names the project root when the project is
not in a git repository.

worktree[repository.toplevel]
The crate MUST report an error that names the project root and the top level
when the project root is not the top level of its repository.

## Worktree

The checkout is detached, because a branch that two working trees hold is a
branch that git refuses in one of them, and the copy needs no name that
outlives it.

The directory lives under the temporary directory of the system and never under
the project. A leftover of an interrupted run is then a directory that the
operating system removes, and it is outside every path that a check of the
project reads.

worktree[worktree.detached]
The crate MUST create a detached checkout of the commit that the HEAD of the
project names.

worktree[worktree.temporary]
The checkout MUST live in a directory that the crate creates under the
temporary directory of the system, and MUST NOT live under the project.

worktree[worktree.unavailable]
A run of git that creates no worktree MUST produce an error that holds what git
wrote.

## Project

The checkout of the contributor is what the crate protects. A run of an action
is a check, and a check that breaks the state that a contributor works in costs
them their afternoon, so every command that the crate starts either reads the
project or writes somewhere else.

A command also has to work on the repository that the caller named, and a bare
git does not promise that. A process that runs inside a git hook, a rebase, or
a bisect inherits variables that name the repository, the working tree, and the
index of the run above it, and those variables beat both the working directory
and the option that names one. A command that kept them would read the index of
that run and write into its repository, which is the one thing that the crate
promises never to do.

The crate therefore drops the whole prefix that git reserves, and not a list of
the names that redirect a command today. A list is a promise that nobody
renewed: git adds a variable, the list stays as it was, and the command it
redirects is the one that writes. The prefix holds whatever git adds, so a name
that nobody has heard of is gone before it can do anything.

worktree[project.untouched]
The crate MUST NOT change the working tree, the index, a reference, or the
stash of the project.

worktree[project.environment]
A command that the crate starts MUST run without every variable of the
environment whose name begins with `GIT_`, so that the command works on the
repository of the directory that the crate named, and so that a variable which
a later version of git reads cannot redirect it.

## Sync

The sync makes the worktree hold what the checkout of the contributor holds.
Git names the paths where the two can differ: a path that it reports as changed
and a path that it reports as untracked. Every other path arrived with the
checkout of the commit.

A path that the project holds is copied over the path in the worktree, and a
path that the project holds nothing at is removed from the worktree. The two
rules together cover a file that changed, a file that arrived, a file that
went, and both halves of a rename.

Git never names a file that it ignores, so a build directory of several
gigabytes is never copied, and the run in the worktree builds from nothing.

The index of the worktree keeps the commit that it checked out, so a change
that the contributor staged and a change that they did not are not told apart.
No build reads that difference, and the tree that a build reads is the tree of
the contributor either way.

worktree[sync.changed]
The crate MUST sync every path that git reports as changed or as untracked in
the project: it MUST copy the path into the worktree when the project holds a
file there, and it MUST remove the path from the worktree when the project
holds nothing there.

worktree[sync.ignored]
The crate MUST NOT sync a file that git ignores.

worktree[sync.modified]
After the sync, git MUST report the worktree as modified when it reports the
project as modified, and as unmodified when it reports the project as
unmodified. A build script that reads the description of the checkout therefore
reads what it reads in the project.

## Paths

An action runs its jobs at directories of the project and reports its findings
at paths of the project, so it needs the name that a path of the project has
inside the worktree. The worktree holds the tree of the project at its own
root, so that name is the same path, relative to the root.

A path can climb through `..`, and a check that asks whether the project holds
such a path answers for the wrong directory, so the crate resolves the climb
first.

worktree[path.inside]
The crate MUST report the path that a path of the project has inside the
worktree, at the same path relative to the root.

worktree[path.foreign]
A path that the project root does not contain MUST get no name inside the
worktree.

worktree[path.parent]
A path that climbs through its parent components MUST be resolved before the
crate decides whether the project root contains it.

## Removal

The value owns the worktree, and a drop removes it. A run that ended with an
answer, a run that stopped with an error, and a run that panicked all reach the
same removal, so a leftover needs an interrupt that no destructor survives.

The removal starts git and waits for it, which holds the thread that drops the
value for as long as git takes. The alternative is a worktree that outlives the
run, and the wait is a fraction of the work that the action did in the copy.

worktree[remove.drop]
A drop of the value MUST remove the worktree of the repository and the
directory that held it.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
