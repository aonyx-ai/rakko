//! The action that checks whether the lockfile of mise is complete
//!
//! A lockfile of [mise] records the version of each tool that a project pins,
//! and for a tool that mise downloads, the address and the checksum of the
//! file for each platform. An install in locked mode stops when the lockfile
//! has no entry for the platform of the machine. It examines only that one
//! platform, so a gap stays hidden until a new machine, a new platform, or a
//! new contributor installs the tool.
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
//! download fails in the default mode of mise, mise ends with an error, and
//! the run stops with the words of mise. Mise can also leave an entry as it
//! was and warn, as it does for a failed download in the mode `merge`. The
//! run then stops with each warning, because the comparison would pass over a
//! gap in that entry. Only a few kinds of warning that say nothing about what
//! mise locked, such as the notice of an older format of the lockfile, stop
//! nothing. A failure of the network is temporary, and a second run answers
//! the question. Other causes last, such as a release without the
//! attestations that mise checks, and the words of mise in the error say
//! which of the two it is. The run makes mise show its warnings, also when the
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
//! A harness erases the action and mounts it next to the others of the
//! project:
//!
//! ```
//! use rakko_action::ErasedAction;
//! use rakko_check_mise_lock::CheckMiseLock;
//!
//! let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckMiseLock)];
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [tokio]: https://tokio.rs

/// Types for the action that checks whether the lockfile of mise is complete
pub mod check_mise_lock;
/// Types for one difference between a lockfile before and after `mise lock`
pub mod difference;
/// Types for the entries of one lockfile of mise
pub mod lockfile;
/// Types for the mise that a run asks to lock a project
pub mod mise;
/// Types for one part of a lockfile of mise
pub mod part;

pub use self::check_mise_lock::{CheckMiseLock, CheckMiseLockArgs, CheckMiseLockError};
pub use self::difference::{Change, Difference};
pub use self::lockfile::{Lockfile, ReadLockfileError};
pub use self::mise::{Lock, Mise, Trust};
pub use self::part::{Part, Platform, ToolName, Version};
