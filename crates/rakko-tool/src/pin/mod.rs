//! The pin of a tool in a mise file of a crate
//!
//! A crate that wraps a tool keeps the version of that tool in a mise file of
//! its own. When the tool has no build for some platforms, the file also
//! lists the platforms where it has one. The crate embeds the file when it
//! compiles. This module reads the pin from the text of that file, in a
//! constant context. So a file without the pin fails the build of the crate,
//! and not a run of an action. A platform that Rakko does not read fails the
//! build too. Rakko reads one name for each operating system and each
//! architecture, such as `arm64`. It does not read an alias that mise also
//! accepts, such as `aarch64`.

mod pinned_platforms;

use rakko::{Architecture, OperatingSystem, Platform};

pub use self::pinned_platforms::PinnedPlatforms;

/// A byte position in the text of a mise file
#[derive(Copy, Clone)]
struct ByteStart(usize);

/// A byte position after the last byte to read
#[derive(Copy, Clone)]
struct ByteEnd(usize);

/// The header of the table that holds the pins of a mise file
const TOOLS: &[u8] = b"[tools]";

/// The key of the version in the table of a pin
const VERSION: &[u8] = b"version";

/// The key of the platforms in the table of a pin
const OS: &[u8] = b"os";

/// The parts of the pin of a tool, as positions in the text of the file
#[derive(Copy, Clone)]
struct Entry {
    /// The first byte of the version, after its opening quotation mark
    version: ByteStart,

    /// The closing quotation mark of the version
    version_end: ByteEnd,

    /// The first byte after the opening bracket of the list of platforms, or
    /// `None` when the pin lists no platforms
    platforms: Option<ByteStart>,
}

/// One element of a list of platforms, as the reader of the list finds it
enum Element {
    /// A platform, and the position where the next element starts
    Platform(Platform, ByteStart),
    /// The closing bracket of the list
    End,
    /// Text that is not a platform that Rakko reads, or a list without its
    /// closing bracket
    Invalid,
}

/// The text of a mise file
///
/// A crate that wraps a tool embeds its mise file as text.
/// This type wraps that text to read the tool pin in a constant context.
///
/// # Examples
///
/// ```
/// use rakko_tool::MiseFile;
///
/// const FILE: MiseFile = MiseFile::new("[tools]\ntaplo = \"0.10.0\"\n");
/// const VERSION: &str = FILE.pinned_version("taplo");
///
/// assert_eq!(VERSION, "0.10.0");
/// ```
#[derive(Copy, Clone, Debug)]
pub struct MiseFile<'a>(&'a str);

impl<'a> MiseFile<'a> {
    /// Wraps the text of a mise file
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_tool::MiseFile;
    ///
    /// let file = MiseFile::new("[tools]\ntaplo = \"0.10.0\"\n");
    ///
    /// assert_eq!(file.pinned_version("taplo"), "0.10.0");
    /// ```
    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self(text)
    }

    /// Returns the version that a mise file pins for a tool
    ///
    /// The method reads the `[tools]` table of the file. It returns the
    /// version in the entry for the tool, without its quotation marks.
    /// The key of the entry can be bare, as in `taplo = "0.10.0"`, or quoted,
    /// as in `"npm:markdownlint-cli" = "0.50.0"`.
    ///
    /// The method reads an entry on one line in one of two forms. The plain
    /// form has a version in double quotation marks. The table form gives the
    /// version and the platforms of the tool, as in
    /// `taplo = { version = "0.10.0", os = ["linux", "macos/arm64"] }`, and
    /// [`pinned_platforms`][platforms] reads the platforms. The mise manager
    /// of Renovate updates the version in both forms. The method does not
    /// read an entry whose value is an array, or a table of its own such as
    /// `[tools.taplo]`.
    ///
    /// Call the method to give a value to a constant, with the file that
    /// [`include_str!`] embeds. The compiler then evaluates it, and a file that
    /// pins no version of the tool fails the build.
    ///
    /// # Panics
    ///
    /// Panics when the `[tools]` table of the file holds no entry for the tool
    /// in one of the two forms. Panics when the table of the entry gives no
    /// version, or holds a key other than `version` and `os`. Panics when the
    /// table lists no platform, or a platform that Rakko does not read. In a
    /// constant, the panic is an error of the build.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_tool::MiseFile;
    ///
    /// const PINS: &str = "[tools]\ntaplo = \"0.10.0\"\n";
    /// const VERSION: &str = MiseFile::new(PINS).pinned_version("taplo");
    ///
    /// assert_eq!(VERSION, "0.10.0");
    /// ```
    ///
    /// A file without the pin fails the build:
    ///
    /// ```compile_fail
    /// use rakko_tool::MiseFile;
    ///
    /// const VERSION: &str = MiseFile::new("[tools]\n").pinned_version("taplo");
    /// ```
    ///
    /// [platforms]: MiseFile::pinned_platforms
    pub const fn pinned_version(&self, tool: &str) -> &'a str {
        let Entry {
            version: ByteStart(first),
            version_end: ByteEnd(end),
            ..
        } = self.entry(tool);

        let (_, rest) = self.0.split_at(first);
        let (version, _) = rest.split_at(end - first);

        version
    }

    /// Returns the platforms that a mise file lists for a tool, or `None` when
    /// the pin of the tool lists none
    ///
    /// The method reads the entry of the tool as
    /// [`pinned_version`][version] does. In the table form, the `os` key holds
    /// the platforms where the tool has a build. They are in the form of the
    /// `os` option of mise, such as `linux` or `macos/arm64`. Mise installs the tool
    /// only on those platforms. A pin without the key needs the tool on every
    /// platform.
    ///
    /// Rakko reads one name for each operating system and each architecture,
    /// such as `arm64`, and not an alias that mise also accepts, such as
    /// `aarch64`. Mise matches no platform with a value that it does not know,
    /// and it gives no warning. So call the method to give a value to a constant, as
    /// for the version. A misspelled platform then fails the build, and not
    /// the install of the tool.
    ///
    /// # Panics
    ///
    /// Panics as [`pinned_version`][version] does. In a constant, the panic is
    /// an error of the build.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// use rakko::{Architecture, OperatingSystem, Platform};
    /// use rakko_tool::{MiseFile, PinnedPlatforms};
    ///
    /// const PINS: MiseFile =
    ///     MiseFile::new("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\", \"macos/arm64\"] }\n");
    /// const PLATFORMS: Option<PinnedPlatforms> = PINS.pinned_platforms("stiles");
    ///
    /// assert_eq!(
    ///     PLATFORMS.map(BTreeSet::from),
    ///     Some(BTreeSet::from([
    ///         Platform::new(OperatingSystem::Linux),
    ///         Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64),
    ///     ]))
    /// );
    /// ```
    ///
    /// A misspelled platform fails the build:
    ///
    /// ```compile_fail
    /// use rakko_tool::{MiseFile, PinnedPlatforms};
    ///
    /// const PLATFORMS: Option<PinnedPlatforms> =
    ///     MiseFile::new("[tools]\nstiles = { version = \"0.6.0\", os = [\"macos-arm64\"] }\n")
    ///         .pinned_platforms("stiles");
    /// ```
    ///
    /// [version]: MiseFile::pinned_version
    pub const fn pinned_platforms(&self, tool: &str) -> Option<PinnedPlatforms<'a>> {
        let Some(platforms) = self.entry(tool).platforms else {
            return None;
        };

        let (_, list) = self.0.split_at(platforms.0);

        Some(PinnedPlatforms::new(list))
    }

    /// Returns the parts of the entry of the tool in the `[tools]` table
    ///
    /// # Panics
    ///
    /// Panics as [`pinned_version`][version] does.
    ///
    /// [version]: MiseFile::pinned_version
    const fn entry(&self, tool: &str) -> Entry {
        let text = self.0.as_bytes();
        let mut start = 0;
        let mut in_tools = false;

        while start < text.len() {
            let mut end = start;
            while end < text.len() && text[end] != b'\n' {
                end += 1;
            }

            let (first, last) = trim(text, ByteStart(start), ByteEnd(end));

            if first < last && text[first] == b'[' {
                in_tools = header_is_tools(text, ByteStart(first), ByteEnd(last));
            } else if in_tools
                && let Some(entry) = entry(text, ByteStart(first), ByteEnd(last), tool.as_bytes())
            {
                return entry;
            }

            start = end + 1;
        }

        panic!("the mise file holds no pin of the tool that Rakko reads in its `[tools]` table");
    }
}

/// Returns the bounds of a line without the spaces at its ends
///
/// The end of a line on Windows carries a carriage return before the line
/// feed. The readers of a header and of an entry stop at a closing mark, so
/// they never reach it.
const fn trim(text: &[u8], first: ByteStart, last: ByteEnd) -> (usize, usize) {
    let mut first = first.0;
    let mut last = last.0;
    while first < last && is_space(text[first]) {
        first += 1;
    }

    while last > first && is_space(text[last - 1]) {
        last -= 1;
    }

    (first, last)
}

/// Returns whether a byte is a space that TOML ignores
const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// Returns whether the header of a table on a line is `[tools]`
///
/// A comment can follow the header, so only the text up to the first
/// closing bracket counts.
const fn header_is_tools(text: &[u8], first: ByteStart, last: ByteEnd) -> bool {
    let first = first.0;
    let last = last.0;
    let mut close = first;
    while close < last && text[close] != b']' {
        close += 1;
    }

    close < last && equals(text, ByteStart(first), ByteEnd(close + 1), TOOLS)
}

/// Returns the parts of an entry, when the entry is the one of the tool
///
/// A line that is not an entry of the tool answers `None`. Such lines are:
///
/// - an empty line,
/// - a comment, which starts with `#`, so that its key is never the name of
///   a tool,
/// - an entry of another tool, and
/// - an entry whose value is neither a string in double quotation marks nor a
///   table on the same line.
///
/// # Panics
///
/// Panics when the entry of the tool holds a table that Rakko does not read,
/// as [`table`] describes.
const fn entry(text: &[u8], first: ByteStart, last: ByteEnd, tool: &[u8]) -> Option<Entry> {
    let Some(cursor) = key(text, first, last, tool) else {
        return None;
    };

    let cursor = skip_spaces(text, ByteStart(cursor), last);
    if cursor >= last.0 || text[cursor] != b'=' {
        return None;
    }

    let cursor = skip_spaces(text, ByteStart(cursor + 1), last);
    if cursor >= last.0 {
        return None;
    }

    match text[cursor] {
        b'"' => {
            let version = cursor + 1;
            let Some(close) = find(text, ByteStart(version), last, b'"') else {
                return None;
            };

            Some(Entry {
                version: ByteStart(version),
                version_end: ByteEnd(close),
                platforms: None,
            })
        }
        b'{' => Some(table(text, ByteStart(cursor + 1), last)),
        _ => None,
    }
}

/// Returns the position after the key at the start of a line, when the key
/// is the expected one
///
/// The key can be bare, or in double quotation marks.
const fn key(text: &[u8], first: ByteStart, last: ByteEnd, expected: &[u8]) -> Option<usize> {
    let first = first.0;
    if first >= last.0 {
        return None;
    }

    if text[first] == b'"' {
        let key = first + 1;
        let Some(close) = find(text, ByteStart(key), last, b'"') else {
            return None;
        };
        if !equals(text, ByteStart(key), ByteEnd(close), expected) {
            return None;
        }

        return Some(close + 1);
    }

    let mut cursor = first;
    while cursor < last.0 && text[cursor] != b'=' && !is_space(text[cursor]) {
        cursor += 1;
    }
    if !equals(text, ByteStart(first), ByteEnd(cursor), expected) {
        return None;
    }

    Some(cursor)
}

/// Returns the parts of a pin in the table form, from the first byte after
/// its opening brace
///
/// The table holds a `version` key with a string, and an optional `os` key
/// with a list of platforms. The keys can come in any order.
///
/// # Panics
///
/// Panics when the table has no closing brace on its line, or gives no
/// version. Panics when it holds a key other than `version` and `os`, or a
/// value of another kind. Panics when it lists no platform, or a platform
/// that Rakko does not read.
const fn table(text: &[u8], cursor: ByteStart, last: ByteEnd) -> Entry {
    let mut cursor = cursor.0;
    let mut version = None;
    let mut platforms = None;

    loop {
        cursor = skip_spaces(text, ByteStart(cursor), last);
        if cursor >= last.0 {
            panic!("the table of the pin of the tool has no closing brace on its line");
        }
        if text[cursor] == b'}' {
            break;
        }

        let pinned_version = key(text, ByteStart(cursor), last, VERSION);
        let pinned_platforms = key(text, ByteStart(cursor), last, OS);
        let after = match (pinned_version, pinned_platforms) {
            (Some(after), _) | (None, Some(after)) => after,
            (None, None) => {
                panic!("the table of the pin of the tool holds a key other than `version` and `os`")
            }
        };

        cursor = skip_spaces(text, ByteStart(after), last);
        if cursor >= last.0 || text[cursor] != b'=' {
            panic!("a key in the table of the pin of the tool has no value");
        }
        cursor = skip_spaces(text, ByteStart(cursor + 1), last);

        if pinned_version.is_some() {
            if cursor >= last.0 || text[cursor] != b'"' {
                panic!("the version in the table of the pin of the tool is not a string");
            }
            let Some(close) = find(text, ByteStart(cursor + 1), last, b'"') else {
                panic!("the version in the table of the pin of the tool is not a string");
            };
            version = Some((ByteStart(cursor + 1), ByteEnd(close)));
            cursor = close + 1;
        } else {
            if cursor >= last.0 || text[cursor] != b'[' {
                panic!("the platforms in the table of the pin of the tool are not a list");
            }
            platforms = Some(ByteStart(cursor + 1));
            cursor = list_end(text, ByteStart(cursor + 1), last) + 1;
        }

        cursor = skip_spaces(text, ByteStart(cursor), last);
        if cursor < last.0 && text[cursor] == b',' {
            cursor += 1;
        } else if cursor >= last.0 || text[cursor] != b'}' {
            panic!("the table of the pin of the tool has no closing brace on its line");
        }
    }

    let Some((first, end)) = version else {
        panic!("the table of the pin of the tool gives no version");
    };

    Entry {
        version: first,
        version_end: end,
        platforms,
    }
}

/// Returns the position of the closing bracket of a list of platforms, from
/// the first byte after its opening bracket
///
/// # Panics
///
/// Panics when an element of the list is not a platform that Rakko reads.
/// Panics when the list has no closing bracket on its line. Panics when the
/// list is empty, because mise then installs the tool on no platform.
const fn list_end(text: &[u8], cursor: ByteStart, last: ByteEnd) -> usize {
    let mut cursor = cursor;
    let mut empty = true;

    loop {
        match element(text, cursor, last) {
            Element::Platform(_, next) => {
                cursor = next;
                empty = false;
            }
            Element::End if empty => panic!("the pin of the tool lists no platform"),
            Element::End => return skip_spaces(text, cursor, last),
            Element::Invalid => {
                panic!("the pin of the tool lists a platform that Rakko does not read")
            }
        }
    }
}

/// Returns the element of a list of platforms that starts at a position
///
/// An element is a platform in double quotation marks, with a comma or the
/// closing bracket after it. A comma after the last platform is allowed.
const fn element(text: &[u8], cursor: ByteStart, last: ByteEnd) -> Element {
    let cursor = skip_spaces(text, cursor, last);
    if cursor >= last.0 {
        return Element::Invalid;
    }
    if text[cursor] == b']' {
        return Element::End;
    }
    if text[cursor] != b'"' {
        return Element::Invalid;
    }

    let Some(close) = find(text, ByteStart(cursor + 1), last, b'"') else {
        return Element::Invalid;
    };
    let Some(platform) = platform(text, ByteStart(cursor + 1), ByteEnd(close)) else {
        return Element::Invalid;
    };

    let next = skip_spaces(text, ByteStart(close + 1), last);
    if next < last.0 && text[next] == b',' {
        Element::Platform(platform, ByteStart(next + 1))
    } else if next < last.0 && text[next] == b']' {
        Element::Platform(platform, ByteStart(next))
    } else {
        Element::Invalid
    }
}

/// Returns the platform that a part of the text gives in the form of mise, or
/// `None` when the text is not such a platform
///
/// The form is an operating system with an optional architecture after a
/// slash, such as `linux` or `macos/arm64`.
const fn platform(text: &[u8], first: ByteStart, last: ByteEnd) -> Option<Platform> {
    let slash = match find(text, first, last, b'/') {
        Some(slash) => slash,
        None => last.0,
    };

    let Some(operating_system) = operating_system(text, first, ByteEnd(slash)) else {
        return None;
    };

    if slash == last.0 {
        return Some(Platform::new(operating_system));
    }

    let Some(architecture) = architecture(text, ByteStart(slash + 1), last) else {
        return None;
    };

    Some(Platform::new(operating_system).with_architecture(architecture))
}

/// Returns the operating system that a part of the text gives, or `None`
/// when mise has no such name
const fn operating_system(text: &[u8], first: ByteStart, last: ByteEnd) -> Option<OperatingSystem> {
    if equals(text, first, last, b"linux") {
        Some(OperatingSystem::Linux)
    } else if equals(text, first, last, b"macos") {
        Some(OperatingSystem::Macos)
    } else if equals(text, first, last, b"windows") {
        Some(OperatingSystem::Windows)
    } else {
        None
    }
}

/// Returns the architecture that a part of the text gives, or `None` when
/// mise has no such name
const fn architecture(text: &[u8], first: ByteStart, last: ByteEnd) -> Option<Architecture> {
    if equals(text, first, last, b"x64") {
        Some(Architecture::X64)
    } else if equals(text, first, last, b"arm64") {
        Some(Architecture::Arm64)
    } else {
        None
    }
}

/// Returns the first position at or after a position that holds no space
const fn skip_spaces(text: &[u8], cursor: ByteStart, last: ByteEnd) -> usize {
    let mut cursor = cursor.0;
    let last = last.0;
    while cursor < last && is_space(text[cursor]) {
        cursor += 1;
    }

    cursor
}

/// Returns the first position at or after a position that holds a byte
const fn find(text: &[u8], cursor: ByteStart, last: ByteEnd, byte: u8) -> Option<usize> {
    let mut cursor = cursor.0;
    let last = last.0;
    while cursor < last {
        if text[cursor] == byte {
            return Some(cursor);
        }
        cursor += 1;
    }

    None
}

/// Returns whether a part of the text holds exactly the expected bytes
const fn equals(text: &[u8], first: ByteStart, last: ByteEnd, expected: &[u8]) -> bool {
    let first = first.0;
    let last = last.0;
    if last - first != expected.len() {
        return false;
    }

    let mut index = 0;
    while index < expected.len() {
        if text[first + index] != expected[index] {
            return false;
        }
        index += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::collections::BTreeSet;

    use super::*;

    /// Returns the platforms that the file lists for stiles, as a set
    fn platforms_in(file: &str) -> Option<BTreeSet<Platform>> {
        MiseFile::new(file)
            .pinned_platforms("stiles")
            .map(BTreeSet::from)
    }

    /// Returns a platform without an architecture
    fn every(operating_system: OperatingSystem) -> Platform {
        Platform::new(operating_system)
    }

    #[test]
    fn byte_end_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ByteEnd>();
    }

    #[test]
    fn byte_start_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ByteStart>();
    }

    #[test]
    fn element_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Element>();
    }

    #[test]
    fn entry_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Entry>();
    }

    #[test]
    fn mise_file_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<MiseFile<'_>>();
    }

    #[test]
    fn pinned_platforms_reads_a_list_with_spaces_and_a_trailing_comma() {
        let file = "[tools]\nstiles = {version=\"0.6.0\",os=[ \"linux\" , \"windows\", ]}\n";

        assert_eq!(
            platforms_in(file),
            Some(BTreeSet::from([
                every(OperatingSystem::Linux),
                every(OperatingSystem::Windows)
            ]))
        );
    }

    #[test]
    fn pinned_platforms_reads_each_platform_of_the_list() {
        let file = "[tools]\nstiles = { version = \"0.6.0\", os = [\"linux/x64\", \"macos/arm64\", \"windows\"] }\n";

        assert_eq!(
            platforms_in(file),
            Some(BTreeSet::from([
                Platform::new(OperatingSystem::Linux).with_architecture(Architecture::X64),
                Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64),
                every(OperatingSystem::Windows),
            ]))
        );
    }

    #[test]
    fn pinned_platforms_reads_the_list_before_the_version() {
        let file = "[tools]\nstiles = { os = [\"macos\"], version = \"0.6.0\" }\n";

        assert_eq!(
            platforms_in(file),
            Some(BTreeSet::from([every(OperatingSystem::Macos)]))
        );
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_a_list_at_the_end_of_the_file_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_a_list_without_its_closing_bracket_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\" }\n");
    }

    #[test]
    fn pinned_platforms_with_a_plain_pin_returns_none() {
        let file = "[tools]\nstiles = \"0.6.0\"\n";

        assert_eq!(platforms_in(file), None);
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_a_platform_without_its_closing_mark_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_a_platform_without_its_opening_mark_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [xlinux\"] }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_a_platform_without_quotation_marks_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [linux] }\n");
    }

    #[test]
    #[should_panic(expected = "are not a list")]
    fn pinned_platforms_with_a_string_as_the_list_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = \"linux\" }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_a_target_triple_panics() {
        platforms_in(
            "[tools]\nstiles = { version = \"0.6.0\", os = [\"aarch64-apple-darwin\"] }\n",
        );
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_an_alias_of_an_architecture_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"macos/aarch64\"] }\n");
    }

    #[test]
    #[should_panic(expected = "lists no platform")]
    fn pinned_platforms_with_an_empty_list_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [] }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn pinned_platforms_with_two_platforms_without_a_comma_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\" \"macos\"] }\n");
    }

    #[test]
    fn pinned_platforms_without_the_key_returns_none() {
        let file = "[tools]\nstiles = { version = \"0.6.0\" }\n";

        assert_eq!(platforms_in(file), None);
    }

    #[test]
    fn pinned_version_ignores_a_comment_after_the_header() {
        let file = "[tools] # the pins\ntaplo = \"0.10.0\"\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_ignores_a_quoted_key_of_another_tool() {
        let file = "[tools]\n\"npm:prettier\" = \"3.9.10\"\n\"taplo\" = \"0.10.0\"\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_ignores_an_entry_of_another_table() {
        let file = "[env]\ntaplo = \"0.9.0\"\n\n[tools]\ntaplo = \"0.10.0\"\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_ignores_an_entry_of_another_tool() {
        let file = "[tools]\ntaplo-cli = \"1.0.0\"\ntaplo = \"0.10.0\"\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_a_file_with_the_line_ends_of_windows() {
        let file = "[tools]\r\ntaplo = \"0.10.0\"\r\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_a_quoted_key() {
        let file = "[tools]\n\"npm:markdownlint-cli\" = \"0.50.0\"\n";

        assert_eq!(
            MiseFile::new(file).pinned_version("npm:markdownlint-cli"),
            "0.50.0"
        );
    }

    #[test]
    fn pinned_version_reads_an_entry_after_a_comment() {
        let file = "[tools]\n# taplo = \"0.9.0\"\ntaplo = \"0.10.0\" # the pin\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_an_entry_without_spaces() {
        let file = "[tools]\ntaplo=\"0.10.0\"\n";

        assert_eq!(MiseFile::new(file).pinned_version("taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_the_version_of_a_table() {
        let file = "[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\"] } # the pin\n";

        assert_eq!(MiseFile::new(file).pinned_version("stiles"), "0.6.0");
    }

    #[test]
    fn pinned_version_reads_the_version_of_a_table_with_a_quoted_key() {
        let file = "[tools]\nstiles = { \"os\" = [\"linux\"], \"version\" = \"0.6.0\" }\n";

        assert_eq!(MiseFile::new(file).pinned_version("stiles"), "0.6.0");
    }

    #[test]
    #[should_panic(expected = "has no value")]
    fn pinned_version_with_a_key_without_a_value_in_the_table_panics() {
        MiseFile::new("[tools]\nstiles = { version }\n").pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "holds no pin of the tool")]
    fn pinned_version_with_a_pin_in_another_table_panics() {
        MiseFile::new("[env]\ntaplo = \"0.10.0\"\n").pinned_version("taplo");
    }

    #[test]
    #[should_panic(expected = "has no closing brace")]
    fn pinned_version_with_a_table_that_ends_after_a_comma_panics() {
        MiseFile::new("[tools]\nstiles = { version = \"0.6.0\",\n").pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "gives no version")]
    fn pinned_version_with_a_table_without_a_version_panics() {
        MiseFile::new("[tools]\nstiles = { os = [\"linux\"] }\n").pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "has no closing brace")]
    fn pinned_version_with_a_table_without_its_closing_brace_panics() {
        MiseFile::new("[tools]\nstiles = { version = \"0.6.0\"\n").pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "is not a string")]
    fn pinned_version_with_a_version_that_is_no_string_in_the_table_panics() {
        MiseFile::new("[tools]\nstiles = { version = 6, os = [\"linux\"] }\n")
            .pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "holds no pin of the tool")]
    fn pinned_version_with_a_version_without_its_closing_mark_panics() {
        MiseFile::new("[tools]\ntaplo = \"0.10.0\n").pinned_version("taplo");
    }

    #[test]
    #[should_panic(expected = "holds no pin of the tool")]
    fn pinned_version_with_an_array_as_the_value_panics() {
        MiseFile::new("[tools]\ntaplo = [\"0.10.0\"]\n").pinned_version("taplo");
    }

    #[test]
    #[should_panic(expected = "holds no pin of the tool")]
    fn pinned_version_with_an_entry_without_an_equals_sign_panics() {
        MiseFile::new("[tools]\ntaplo : \"0.10.0\"\n").pinned_version("taplo");
    }

    #[test]
    #[should_panic(expected = "is not a string")]
    fn pinned_version_with_an_unclosed_version_in_the_table_panics() {
        MiseFile::new("[tools]\nstiles = { version = \"0.6.0 }\n").pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "holds a key other than")]
    fn pinned_version_with_another_option_in_the_table_panics() {
        MiseFile::new("[tools]\nstiles = { version = \"0.6.0\", version_prefix = \"stiles@\" }\n")
            .pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "has no closing brace")]
    fn pinned_version_with_two_keys_without_a_comma_panics() {
        MiseFile::new("[tools]\nstiles = { version = \"0.6.0\" os = [\"linux\"] }\n")
            .pinned_version("stiles");
    }

    #[test]
    #[should_panic(expected = "holds no pin of the tool")]
    fn pinned_version_without_the_tool_before_a_blank_last_line_panics() {
        MiseFile::new("[tools]\njq = \"1.8.2\"\n  ").pinned_version("taplo");
    }

    #[test]
    #[should_panic(expected = "holds no pin of the tool")]
    fn pinned_version_without_the_tool_panics() {
        MiseFile::new("[tools]\njq = \"1.8.2\"\n").pinned_version("taplo");
    }
}
