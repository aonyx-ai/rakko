//! Where and why a test panicked

use std::path::PathBuf;

use bon::bon;
use getset::{CopyGetters, Getters};

/// Where and why a test panicked
///
/// The test harness of Rust writes the location of a panic and its message
/// to the output of the test, and nextest keeps that output for a test that
/// failed. The path stands as the compiler embedded it, which is relative to
/// the root that cargo built, so a caller asks the root for the path
/// relative to the project.
///
/// # Examples
///
/// ```
/// use rakko_nextest::Panic;
/// use std::path::PathBuf;
///
/// let panic = Panic::builder()
///     .path(PathBuf::from("src/lib.rs"))
///     .line(10)
///     .column(3)
///     .build();
///
/// assert_eq!(panic.path(), &PathBuf::from("src/lib.rs"));
/// assert_eq!((panic.line(), panic.column()), (10, 3));
/// assert_eq!(panic.message(), &None);
/// ```
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

#[bon]
impl Panic {
    /// Creates a panic from the location and the message that the test
    /// harness wrote
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_nextest::Panic;
    /// use std::path::PathBuf;
    ///
    /// let panic = Panic::builder()
    ///     .path(PathBuf::from("tests/test.rs"))
    ///     .line(42)
    ///     .column(5)
    ///     .message("assertion failed".to_string())
    ///     .build();
    ///
    /// assert_eq!(panic.message().as_deref(), Some("assertion failed"));
    /// ```
    #[builder]
    pub fn new(path: PathBuf, line: u32, column: u32, message: Option<String>) -> Self {
        Self {
            path,
            line,
            column,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Panic>();
    }
}
