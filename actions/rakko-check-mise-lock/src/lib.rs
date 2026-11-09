//! The action that checks whether the lockfile of mise is complete
//!
//! A lockfile of [mise] records the version of each tool that a project pins.
//! For a tool that mise downloads, it also records the address and checksum
//! of the file for each platform. An install in locked mode stops when the
//! lockfile has no entry for the platform of the machine. It examines only
//! that one platform, so a gap stays hidden until a new machine, a new
//! platform, or a new contributor installs the tool.
//!
//! Mise has no operation that only checks, so the action runs `mise lock` and
//! compares the lockfiles of the project before and after the run. A lockfile
//! that `mise lock` leaves as it is passes. Each tool, version, and platform
//! that `mise lock` adds, changes, or removes is a finding.
//!
//! A run reports by default, and it locks a disposable copy of the project,
//! so the checkout of a contributor stays as it was. With `fix`, the run locks
//! the project itself, and each difference is a repair. A project that
//! configures no tool to lock skips visibly.
//!
//! `mise lock` targets the platforms that the setting `lockfile_platforms` of
//! the project names, and the platform of the machine that runs it as well. In
//! the default mode of mise, it also removes the entries of every other
//! platform. A project therefore names every platform on which something
//! locks it: its jobs, its contributors, and a bot that moves its pins, such
//! as Renovate. Otherwise the entries that one machine writes are entries that
//! the next run on another machine removes. The lockfile also depends on the
//! version of mise, and when a contributor gets another answer than CI, CI
//! decides.
//!
//! `mise lock` asks the network for the downloads of each platform. When a
//! download fails, mise ends with an error, and the run stops with the words
//! of mise. Some failures only make mise warn and go on. Examples are a
//! failed query for the tags of a release, after which mise guesses the tag.
//! In the mode `merge`, a failed download does the same. The run then stops
//! with
//! each warning, because the comparison would pass over a gap in the entry of
//! that tool. Only a few kinds of warning stop nothing. They say nothing about
//! what mise locked, such as the notice of an older format of the lockfile. A
//! failure of the
//! network is temporary, and a second run answers
//! the question. Other causes last, such as a release without the
//! attestations that mise checks. The words of mise in the error say which
//! of the two it is. The run makes mise show its warnings, also when the
//! environment, the project, or the global configuration of a contributor
//! asks mise to be quiet.
//!
//! The action compares the lockfiles, and not the dependency graphs that mise
//! keeps beside them for tools such as those of npm.
//!
//! # Asynchronous Runtime
//!
//! A run starts mise and git and waits for them. A [Tokio] runtime drives
//! that, and it panics without one.
//!
//! # Examples
//!
//! A harness adds the action as it is to a list, beside the other actions of
//! the project:
//!
//! ```
//! use rakko::Bundle;
//! use rakko_check_mise_lock::CheckMiseLock;
//!
//! let actions = Bundle::default().with(CheckMiseLock);
//!
//! assert_eq!(actions.actions().len(), 1);
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs

mod check_mise_lock;
mod difference;
mod lockfile;
mod mise;
mod part;

pub use self::check_mise_lock::{CheckMiseLock, CheckMiseLockArgs, CheckMiseLockError, Operation};
pub use self::lockfile::ReadLockfileError;
