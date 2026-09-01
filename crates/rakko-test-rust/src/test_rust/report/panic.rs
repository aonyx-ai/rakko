use std::path::PathBuf;

use getset::{CopyGetters, Getters};

/// Where and why a test panicked
///
/// The test harness of Rust writes the location of a panic and its message
/// to the output of the test, and nextest keeps that output for a test that
/// failed. The path stands as the compiler embedded it, which is relative to
/// the root that cargo built, so a caller asks the root for the path
/// relative to the project.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct Panic {
    /// The file that the test panicked in, as the compiler embedded it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The line where the test panicked
    #[getset(get_copy = "pub")]
    line: u32,

    /// The column where the test panicked
    #[getset(get_copy = "pub")]
    column: u32,

    /// The message of the panic, or nothing when the test gave none
    #[getset(get = "pub")]
    message: Option<String>,
}

impl Panic {
    /// Creates a panic from the location and the message that the test
    /// harness wrote
    pub fn new(path: PathBuf, line: u32, column: u32, message: Option<String>) -> Self {
        Self {
            path,
            line,
            column,
            message,
        }
    }
}
