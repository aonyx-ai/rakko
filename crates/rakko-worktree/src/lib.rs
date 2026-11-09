//! A disposable copy of a project to work in
//!
//! Some work writes files that nobody asked for. A resolution of dependencies
//! rewrites a lockfile, and a build writes what it generates. A check that does
//! such work in the checkout of a contributor leaves the tree dirty. Every
//! other program that reads the tree at the same time reads what the check
//! wrote: an editor, a language server, a second build.
//!
//! This crate hands out a copy to write in instead. A [`Worktree`] is a
//! detached checkout of the commit that the HEAD of the project names. It sits
//! in a directory of the temporary directory of the system. The changed files
//! of the checkout are synced over it. The copy therefore holds what the
//! contributor holds, byte for byte. So a job that runs in it answers for the
//! tree that the contributor is working on.
//!
//! Nothing in the checkout of the contributor changes. The crate makes no
//! commit, writes no stash, and applies no patch. It writes into the
//! repository, where the worktree shows up in `git worktree list` while the
//! value lives. It also writes into a directory of its own.
//!
//! The value owns the copy. A drop removes the worktree and the directory. So
//! an error and a panic leave nothing behind. An interrupt that no destructor
//! survives leaves a directory that the operating system removes and an entry
//! that `git worktree prune` forgets.
//!
//! The crate enforces nothing. It gives a job a place to write, and it does
//! not stop the job from writing elsewhere.
//!
//! # Requirements
//!
//! The project must be a git repository, and its root must be the top level of
//! that repository. Git must be reachable by name, the way the operating
//! system finds a program.
//!
//! A run inside a git hook, a rebase, or a bisect works as well. Such a run
//! inherits variables that name the repository of the run above it. Those
//! variables beat the working directory of a command. So every command of this
//! crate runs without the variables that git reads for itself, and works on the
//! project that the caller named.
//!
//! # Tools
//!
//! The copy is a directory that nobody set up, and [mise] reads the
//! configuration of a directory only after somebody trusted that directory. A
//! caller therefore resolves its tools and its toolchains at the project,
//! where the configuration is trusted, and starts them in the copy.
//!
//! # Asynchronous Runtime
//!
//! Creating a copy starts git and waits for it, and a [Tokio] runtime drives
//! that work. The drop that removes the copy needs no runtime: it waits for
//! git on the thread that dropped the value.
//!
//! # Examples
//!
//! ```no_run
//! use rakko_worktree::Worktree;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let project = "/home/otter/project";
//!
//! let worktree = Worktree::create(project.into()).await?;
//!
//! // A job that runs here writes into the copy and not into the project.
//! println!("{}", worktree.root());
//!
//! // The worktree and its directory are gone after this.
//! drop(worktree);
//! # Ok(())
//! # }
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs

mod worktree;

pub use self::worktree::{CreateWorktreeError, Worktree};
