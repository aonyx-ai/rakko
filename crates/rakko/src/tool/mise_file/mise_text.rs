//! The text of a mise file that a crate embeds

use super::{ToolPin, count_pins, read_pins};

/// The text of a mise file that a crate embeds
///
/// The reader of the file works in a constant context, so that a file that it
/// refuses fails the build of the crate. [`pins`][pins] reads every pin of the
/// file for [`bundled_tools!`][bundled].
///
/// [bundled]: crate::bundled_tools
/// [pins]: MiseText::pins
///
/// # Examples
///
/// ```
/// use rakko::__private::MiseText;
///
/// const TEXT: MiseText<'static> = MiseText::new("[tools]\ntaplo = \"0.10.0\"\n");
///
/// assert_eq!(TEXT.pin_count(), 1);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct MiseText<'a>(&'a str);

impl<'a> MiseText<'a> {
    /// Wraps the text of a mise file
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::__private::MiseText;
    ///
    /// const TEXT: MiseText<'static> = MiseText::new("[tools]\ntaplo = \"0.10.0\"\n");
    ///
    /// assert_eq!(TEXT.pin_count(), 1);
    /// ```
    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self(text)
    }
}

impl MiseText<'static> {
    /// Returns the number of pins in the file
    ///
    /// # Panics
    ///
    /// Panics as [`pins`][pins] does for a line that the reader does not
    /// read.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::__private::MiseText;
    ///
    /// const COUNT: usize = MiseText::new("[tools]\ntaplo = \"0.10.0\"\n# a comment\n").pin_count();
    ///
    /// assert_eq!(COUNT, 1);
    /// ```
    ///
    /// [pins]: MiseText::pins
    #[must_use]
    pub const fn pin_count(&self) -> usize {
        count_pins(self.0)
    }

    /// Returns every pin of the file, in the order of the file
    ///
    /// The reader reads the `[tools]` table of the file. Each line of the
    /// table is a pin, a comment, or empty. A pin has one of two forms, on
    /// one line:
    ///
    /// - a version in double quotation marks, as in `taplo = "0.10.0"`, and
    /// - a table with the version and the platforms of the tool, as in
    ///   `zizmor = { version = "1.32.0", os = ["linux", "macos"] }`.
    ///
    /// The key of a pin can be bare, as `taplo`, or in double quotation
    /// marks, as `"npm:markdownlint-cli"`. A comment can follow a pin. It can
    /// also stand on a line of its own anywhere in the file.
    ///
    /// The version is exact. It has at least three parts that dots separate,
    /// and each part starts with a digit, as `0.10.0` or `1.0.0-rc.1`. A `v`
    /// can come before the first part, and a tag of a release can come before
    /// the version with `@`, as `stiles@0.6.0`. Mise resolves a version that
    /// is not exact, such as `latest` or `3`, to any matching version that it
    /// installed, so the reader refuses it.
    ///
    /// The array `N` must hold as many pins as the file holds, which
    /// [`pin_count`][count] gives.
    ///
    /// # Panics
    ///
    /// Panics when the file holds a line that the reader does not read, so
    /// that no pin goes missing without a sign. Such lines are:
    ///
    /// - a key outside the `[tools]` table,
    /// - a table other than `[tools]`,
    /// - a value in another form,
    /// - an escape in a string,
    /// - text after a pin, and
    /// - a version that is not exact.
    ///
    /// Panics when the file pins no tool, or pins a tool twice. Panics when
    /// the table of a pin holds a key other than `version` and `os`, holds a
    /// key twice, or gives no version. Panics when it lists no platform, or a
    /// platform that Rakko does not read. In a constant, the panic is an
    /// error of the build.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::__private::{MiseText, ToolPin};
    ///
    /// const PINS: [ToolPin; 2] =
    ///     MiseText::new("[tools]\ntaplo = \"0.10.0\"\nzizmor = { version = \"1.32.0\" }\n").pins();
    ///
    /// assert_eq!(rakko::__private::tools(&PINS).requirements().len(), 2);
    /// ```
    ///
    /// [count]: MiseText::pin_count
    #[must_use]
    pub const fn pins<const N: usize>(&self) -> [ToolPin; N] {
        read_pins(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mise_text_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<MiseText<'_>>();
    }
}
