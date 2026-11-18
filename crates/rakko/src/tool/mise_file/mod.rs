//! The reader of the mise file that a crate embeds
//!
//! A crate that wraps a tool keeps the pin of that tool in a mise file of its
//! own. When the tool has no build for some platforms, the pin also lists the
//! platforms where it has one. The crate embeds the file when it compiles.
//! This module reads the text of that file in a constant context. So a file
//! that the reader refuses fails the build of the crate. It does not fail a
//! run of an action.
//!
//! [`MiseText::pins`] reads every pin of the file. It refuses every line that
//! it does not read, so that no pin goes missing without a sign. It also
//! refuses a version that is not exact, because mise resolves such a version
//! to whatever matching version it installed.
//!
//! The reader reads one name for each operating system and each
//! architecture, such as `arm64`. It does not read an alias that mise also
//! accepts, such as `aarch64`.

mod mise_text;

use std::collections::BTreeSet;

pub use self::mise_text::MiseText;
use crate::tool::platform::{Architecture, OperatingSystem, Platform};
use crate::tool::{MiseTool, ToolRequirement, ToolVersion};

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

    /// The first byte after the value of the pin
    end: ByteStart,
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

/// The pin of one tool in the mise file that a crate embeds
///
/// A pin holds the identifier of the tool, its exact version, and the
/// platforms where it has a build. It holds no program, because one tool can
/// provide several programs. The action gives the program that it starts
/// when it runs.
///
/// Only the reader of a mise file creates a pin, and [`bundled_tools!`] calls
/// that reader. So each pin is a line of a mise file. The parts of the pin
/// are text of the embedded file, which the reader checked when the crate
/// compiled.
///
/// [`bundled_tools!`]: crate::bundled_tools
///
/// # Examples
///
/// ```
/// use rakko::__private::{MiseText, ToolPin};
///
/// const TEXT: MiseText<'static> = MiseText::new("[tools]\ntaplo = \"0.10.0\"\n");
/// const PINS: [ToolPin; 1] = TEXT.pins::<1>();
///
/// assert_eq!(rakko::__private::tools(&PINS).requirements().len(), 1);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ToolPin {
    /// The key of the pin, without its quotation marks
    tool: &'static str,

    /// The version of the pin, without its quotation marks
    version: &'static str,

    /// The text of the list of platforms after its opening bracket, or
    /// `None` when the pin lists no platforms
    platforms: Option<&'static str>,
}

impl ToolPin {
    /// The value that fills an array before the reader writes the pins into it
    const EMPTY: Self = Self {
        tool: "",
        version: "",
        platforms: None,
    };

    /// Returns whether two pins pin the same tool
    const fn same_tool(&self, other: &Self) -> bool {
        let tool = self.tool.as_bytes();

        equals(
            tool,
            ByteStart(0),
            ByteEnd(tool.len()),
            other.tool.as_bytes(),
        )
    }

    /// Returns the requirement that the pin declares
    ///
    /// The requirement holds the exact version and the platforms of the pin,
    /// and no program.
    pub(crate) fn requirement(&self) -> ToolRequirement {
        ToolRequirement::declared()
            .tool(MiseTool::new(self.tool))
            .version(ToolVersion::new(self.version))
            .maybe_platforms(self.platforms.map(platforms))
            .call()
    }
}

/// Returns every pin of a mise file, in the order of the file
///
/// The array `N` must hold exactly as many pins as the file holds, which
/// [`count_pins`] gives. [`MiseText::pins`] describes the lines that the
/// reader reads.
///
/// # Panics
///
/// Panics as [`MiseText::pins`] does. In a constant, the panic is an error
/// of the build.
const fn read_pins<const N: usize>(text: &'static str) -> [ToolPin; N] {
    if N == 0 {
        panic!("the mise file pins no tool");
    }

    let (pins, count) = walk::<N>(text);
    if count != N {
        panic!("the number of pins does not match the number of pins in the mise file");
    }

    let mut first = 0;
    while first < N {
        let mut second = first + 1;
        while second < N {
            if pins[first].same_tool(&pins[second]) {
                panic!("the mise file pins a tool twice");
            }
            second += 1;
        }
        first += 1;
    }

    pins
}

/// Returns the number of pins in a mise file
///
/// # Panics
///
/// Panics as [`MiseText::pins`] does for a line that the reader does not
/// read.
const fn count_pins(text: &'static str) -> usize {
    walk::<0>(text).1
}

/// Reads every line of a mise file, and returns the first `N` pins with the
/// number of pins in the file
///
/// An empty line and a comment hold no pin, so the reader passes over them.
///
/// # Panics
///
/// Panics as [`MiseText::pins`] does for a line that the reader does not
/// read.
const fn walk<const N: usize>(text: &'static str) -> ([ToolPin; N], usize) {
    let bytes = text.as_bytes();
    let mut pins = [ToolPin::EMPTY; N];
    let mut count = 0;
    let mut in_tools = false;
    let mut seen_tools = false;
    let mut start = 0;

    while start < bytes.len() {
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'\n' {
            end += 1;
        }

        let mut line_end = end;
        if line_end > start && bytes[line_end - 1] == b'\r' {
            line_end -= 1;
        }

        let (first, last) = trim(bytes, ByteStart(start), ByteEnd(line_end));

        if first < last && bytes[first] == b'[' {
            if !header_is_tools(bytes, ByteStart(first), ByteEnd(last)) {
                panic!(
                    "the mise file holds a table other than `[tools]`, which Rakko does not read"
                );
            }
            if seen_tools {
                panic!("the mise file holds the `[tools]` table twice");
            }
            in_tools = true;
            seen_tools = true;
        } else if first < last && bytes[first] != b'#' {
            if !in_tools {
                panic!("the mise file holds a key outside the `[tools]` table");
            }

            let pin = strict_entry(text, ByteStart(first), ByteEnd(last));
            if count < N {
                pins[count] = pin;
            }
            count += 1;
        }

        start = end + 1;
    }

    (pins, count)
}

/// Returns the pin on a line of the `[tools]` table, and refuses a line that
/// is no pin that the reader reads
///
/// # Panics
///
/// Panics when the line is no pin in one of the two forms that
/// [`MiseText::pins`] reads. Panics when text other than a comment follows the
/// pin.
const fn strict_entry(text: &'static str, first: ByteStart, last: ByteEnd) -> ToolPin {
    let bytes = text.as_bytes();
    let (key_first, key_end, after) = strict_key(bytes, first, last);

    let cursor = skip_spaces(bytes, ByteStart(after), last);
    if cursor >= last.0 || bytes[cursor] != b'=' {
        panic!("a line of the `[tools]` table of the mise file is no pin, because it has no `=`");
    }

    let cursor = skip_spaces(bytes, ByteStart(cursor + 1), last);
    if cursor >= last.0 {
        panic!("a pin of the mise file has no value");
    }

    let Entry {
        version,
        version_end,
        platforms,
        end,
    } = match bytes[cursor] {
        b'"' => {
            let version = cursor + 1;
            let Some(close) = find(bytes, ByteStart(version), last, b'"') else {
                panic!("the version of a pin of the mise file has no closing quotation mark");
            };

            Entry {
                version: ByteStart(version),
                version_end: ByteEnd(close),
                platforms: None,
                end: ByteStart(close + 1),
            }
        }
        b'{' => table(bytes, ByteStart(cursor + 1), last),
        _ => panic!(
            "a pin of the mise file is neither a version in double quotation marks nor a table on one line"
        ),
    };

    let rest = skip_spaces(bytes, end, last);
    if rest < last.0 && bytes[rest] != b'#' {
        panic!("a pin of the mise file has text after its value");
    }

    if version.0 == version_end.0 {
        panic!("a pin of the mise file gives an empty version");
    }
    if holds(bytes, version, version_end, b'\\') {
        panic!("the version of a pin of the mise file holds an escape, which Rakko does not read");
    }
    if !is_exact(bytes, version, version_end) {
        panic!("a pin of the mise file gives a version that is not exact, such as `latest` or `3`");
    }

    let platforms = match platforms {
        Some(list) => Some(slice(text, list, ByteEnd(end.0))),
        None => None,
    };

    ToolPin {
        tool: slice(text, key_first, key_end),
        version: slice(text, version, version_end),
        platforms,
    }
}

/// Returns the bounds of the key at the start of a line, and the position
/// after it
///
/// A bare key holds letters, digits, `-`, and `_`, as TOML allows. A key in
/// double quotation marks can hold any character but an escape.
///
/// # Panics
///
/// Panics when the line starts with no key that the reader reads.
const fn strict_key(bytes: &[u8], first: ByteStart, last: ByteEnd) -> (ByteStart, ByteEnd, usize) {
    let first = first.0;

    if bytes[first] == b'"' {
        let key = first + 1;
        let Some(close) = find(bytes, ByteStart(key), last, b'"') else {
            panic!("the key of a pin of the mise file has no closing quotation mark");
        };
        if key == close {
            panic!("a pin of the mise file has an empty key");
        }
        if holds(bytes, ByteStart(key), ByteEnd(close), b'\\') {
            panic!("the key of a pin of the mise file holds an escape, which Rakko does not read");
        }

        return (ByteStart(key), ByteEnd(close), close + 1);
    }

    let mut cursor = first;
    while cursor < last.0 && is_bare(bytes[cursor]) {
        cursor += 1;
    }
    if cursor == first {
        panic!(
            "a line of the `[tools]` table of the mise file starts with no key that Rakko reads"
        );
    }

    (ByteStart(first), ByteEnd(cursor), cursor)
}

/// Returns whether a byte can stand in a bare key of TOML
const fn is_bare(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

/// Returns whether a version is exact
///
/// An exact version has at least three parts that dots separate, and each
/// part starts with a digit. A `v` can come before the first part. A tag of a
/// release can come before the version, up to the last `@`.
const fn is_exact(text: &[u8], first: ByteStart, end: ByteEnd) -> bool {
    let mut cursor = first.0;
    let mut at = first.0;
    while at < end.0 {
        if text[at] == b'@' {
            cursor = at + 1;
        }
        at += 1;
    }

    if cursor < end.0 && text[cursor] == b'v' {
        cursor += 1;
    }

    let mut parts = 0;
    let mut part_start = true;
    while cursor < end.0 {
        if text[cursor] == b'.' {
            if part_start {
                return false;
            }
            part_start = true;
        } else if part_start {
            if !text[cursor].is_ascii_digit() {
                return false;
            }
            parts += 1;
            part_start = false;
        }
        cursor += 1;
    }

    !part_start && parts >= 3
}

/// Returns each platform of a list that the reader checked
///
/// The text starts after the opening bracket of the list. Each platform
/// counts once.
pub(crate) fn platforms(list: &str) -> BTreeSet<Platform> {
    let text = list.as_bytes();
    let last = ByteEnd(text.len());
    let mut cursor = ByteStart(0);
    let mut platforms = BTreeSet::new();

    while let Element::Platform(platform, next) = element(text, cursor, last) {
        platforms.insert(platform);
        cursor = next;
    }

    platforms
}

/// Returns a part of a text between two byte positions
const fn slice(text: &str, first: ByteStart, end: ByteEnd) -> &str {
    let (_, rest) = text.split_at(first.0);
    let (part, _) = rest.split_at(end.0 - first.0);

    part
}

/// Returns the bounds of a line without the spaces at its ends
///
/// The end of a line on Windows carries a carriage return before the line
/// feed. [`walk`] removes it before it trims the line.
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
/// version. Panics when it holds a key other than `version` and `os`, a key
/// twice, or a value of another kind. Panics when it lists no platform, or a platform
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
            if version.is_some() {
                panic!("the table of the pin of the tool gives the key `version` twice");
            }
            if cursor >= last.0 || text[cursor] != b'"' {
                panic!("the version in the table of the pin of the tool is not a string");
            }
            let Some(close) = find(text, ByteStart(cursor + 1), last, b'"') else {
                panic!("the version in the table of the pin of the tool is not a string");
            };
            version = Some((ByteStart(cursor + 1), ByteEnd(close)));
            cursor = close + 1;
        } else {
            if platforms.is_some() {
                panic!("the table of the pin of the tool gives the key `os` twice");
            }
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
        end: ByteStart(cursor + 1),
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

/// Returns whether a part of the text holds a byte
const fn holds(text: &[u8], first: ByteStart, last: ByteEnd, byte: u8) -> bool {
    find(text, first, last, byte).is_some()
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

    /// Returns the platforms that the one pin of a file lists, as a set
    fn platforms_in(file: &'static str) -> Option<BTreeSet<Platform>> {
        read_pins::<1>(file)[0].requirement().platforms().cloned()
    }

    /// Returns the identifiers and versions of every pin of a file
    fn pins_of<const N: usize>(file: &'static str) -> Vec<(&'static str, &'static str)> {
        read_pins::<N>(file)
            .iter()
            .map(|pin| (pin.tool, pin.version))
            .collect()
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
    fn count_pins_counts_each_pin_of_the_table() {
        let file =
            "[tools]\ntaplo = \"0.10.0\"\n# a comment\n\nzizmor = { version = \"1.32.0\" }\n";

        assert_eq!(count_pins(file), 2);
    }

    #[test]
    #[should_panic(expected = "outside the `[tools]` table")]
    fn count_pins_with_a_key_outside_the_table_panics() {
        count_pins("min_version = \"2026.1.0\"\n[tools]\ntaplo = \"0.10.0\"\n");
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
    fn read_pins_keeps_the_platforms_of_a_pin() {
        let pins = read_pins::<1>(
            "[tools]\nzizmor = { version = \"1.32.0\", os = [\"linux\", \"macos/arm64\"] }\n",
        );

        assert_eq!(
            pins[0].requirement().platforms(),
            Some(&BTreeSet::from([
                every(OperatingSystem::Linux),
                Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64),
            ]))
        );
    }

    #[test]
    fn read_pins_of_a_plain_pin_lists_no_platform() {
        let file = "[tools]\nstiles = \"0.6.0\"\n";

        assert_eq!(platforms_in(file), None);
    }

    #[test]
    fn read_pins_of_a_table_without_platforms_lists_none() {
        let file = "[tools]\nstiles = { version = \"0.6.0\" }\n";

        assert_eq!(platforms_in(file), None);
    }

    #[test]
    fn read_pins_reads_a_comment_after_a_pin() {
        let pins = pins_of::<2>(
            "[tools] # pins\ntaplo = \"0.10.0\" # a pin\nzizmor = { version = \"1.32.0\" } # a table\n",
        );

        assert_eq!(pins, [("taplo", "0.10.0"), ("zizmor", "1.32.0")]);
    }

    #[test]
    fn read_pins_reads_a_file_with_the_line_ends_of_windows() {
        let pins = pins_of::<2>(
            "# the pins\r\n[tools]\r\ntaplo = \"0.10.0\"\r\nzizmor = { version = \"1.32.0\" }\r\n",
        );

        assert_eq!(pins, [("taplo", "0.10.0"), ("zizmor", "1.32.0")]);
    }

    #[test]
    fn read_pins_reads_a_list_of_platforms_with_spaces_and_a_trailing_comma() {
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
    fn read_pins_reads_a_pin_without_spaces() {
        let pins = pins_of::<1>("[tools]\ntaplo=\"0.10.0\"\n");

        assert_eq!(pins, [("taplo", "0.10.0")]);
    }

    #[test]
    fn read_pins_reads_a_table_with_quoted_keys() {
        let pins =
            pins_of::<1>("[tools]\nstiles = { \"os\" = [\"linux\"], \"version\" = \"0.6.0\" }\n");

        assert_eq!(pins, [("stiles", "0.6.0")]);
    }

    #[test]
    fn read_pins_reads_both_forms_in_the_order_of_the_file() {
        let pins = pins_of::<3>(
            "# The pins of the crate.\n[tools]\ntaplo = \"0.10.0\"\n\"npm:renovate\" = \"44.240.0\"\n\"github:aonyx-ai/stiles\" = { version = \"stiles@0.6.0\", os = [\"linux\"] }\n",
        );

        assert_eq!(
            pins,
            [
                ("taplo", "0.10.0"),
                ("npm:renovate", "44.240.0"),
                ("github:aonyx-ai/stiles", "stiles@0.6.0"),
            ]
        );
    }

    #[test]
    fn read_pins_reads_each_platform_of_the_list() {
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
    fn read_pins_reads_exact_versions_with_a_prefix_and_a_suffix() {
        let pins = pins_of::<2>("[tools]\nshfmt = \"v3.14.1\"\nnode = \"26.0.0-rc.1\"\n");

        assert_eq!(pins, [("shfmt", "v3.14.1"), ("node", "26.0.0-rc.1")]);
    }

    #[test]
    fn read_pins_reads_the_list_before_the_version() {
        let file = "[tools]\nstiles = { os = [\"macos\"], version = \"0.6.0\" }\n";

        assert_eq!(
            platforms_in(file),
            Some(BTreeSet::from([every(OperatingSystem::Macos)]))
        );
    }

    #[test]
    #[should_panic(expected = "has no `=`")]
    fn read_pins_with_a_dotted_key_panics() {
        read_pins::<1>("[tools]\ntaplo.version = \"0.10.0\"\n");
    }

    #[test]
    fn read_pins_with_a_key_that_starts_with_another_reads_both() {
        let pins = pins_of::<2>("[tools]\ntaplo-cli = \"1.0.0\"\ntaplo = \"0.10.0\"\n");

        assert_eq!(pins, [("taplo-cli", "1.0.0"), ("taplo", "0.10.0")]);
    }

    #[test]
    #[should_panic(expected = "has no value")]
    fn read_pins_with_a_key_without_a_value_in_the_table_panics() {
        read_pins::<1>("[tools]\nstiles = { version }\n");
    }

    #[test]
    #[should_panic(expected = "the key of a pin of the mise file has no closing quotation mark")]
    fn read_pins_with_a_key_without_its_closing_mark_panics() {
        read_pins::<1>("[tools]\n\"npm:renovate = 44.240.0\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_a_keyword_as_the_version_panics() {
        read_pins::<1>("[tools]\ntaplo = \"latest\"\n");
    }

    #[test]
    #[should_panic(expected = "starts with no key that Rakko reads")]
    fn read_pins_with_a_line_that_starts_with_no_key_panics() {
        read_pins::<1>("[tools]\n= \"0.10.0\"\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_list_at_the_end_of_the_file_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_list_without_its_closing_bracket_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\" }\n");
    }

    #[test]
    #[should_panic(expected = "neither a version in double quotation marks nor a table")]
    fn read_pins_with_a_literal_string_panics() {
        read_pins::<1>("[tools]\ntaplo = '0.10.0'\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_a_major_version_alone_panics() {
        read_pins::<1>("[tools]\nshfmt = \"3\"\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_a_minor_version_alone_panics() {
        read_pins::<1>("[tools]\nshfmt = \"3.14\"\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_misspelled_platform_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"0.6.0\", os = [\"macos-arm64\"] }\n");
    }

    #[test]
    #[should_panic(expected = "has no value")]
    fn read_pins_with_a_pin_without_a_value_panics() {
        read_pins::<1>("[tools]\ntaplo =\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_platform_without_its_closing_mark_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_platform_without_its_opening_mark_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [xlinux\"] }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_platform_without_quotation_marks_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [linux] }\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_a_reference_as_the_version_panics() {
        read_pins::<1>("[tools]\ntaplo = \"ref:0.10.0\"\n");
    }

    #[test]
    #[should_panic(expected = "are not a list")]
    fn read_pins_with_a_string_as_the_list_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = \"linux\" }\n");
    }

    #[test]
    #[should_panic(expected = "a table other than `[tools]`")]
    fn read_pins_with_a_table_of_its_own_for_a_tool_panics() {
        read_pins::<1>("[tools.zizmor]\nversion = \"1.32.0\"\n");
    }

    #[test]
    #[should_panic(expected = "has no closing brace")]
    fn read_pins_with_a_table_that_ends_after_a_comma_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"0.6.0\",\n");
    }

    #[test]
    #[should_panic(expected = "gives no version")]
    fn read_pins_with_a_table_without_a_version_panics() {
        read_pins::<1>("[tools]\nstiles = { os = [\"linux\"] }\n");
    }

    #[test]
    #[should_panic(expected = "has no closing brace")]
    fn read_pins_with_a_table_without_its_closing_brace_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"0.6.0\"\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_a_tag_without_a_version_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"stiles@latest\" }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_a_target_triple_panics() {
        platforms_in(
            "[tools]\nstiles = { version = \"0.6.0\", os = [\"aarch64-apple-darwin\"] }\n",
        );
    }

    #[test]
    #[should_panic(expected = "pins a tool twice")]
    fn read_pins_with_a_tool_pinned_twice_panics() {
        read_pins::<2>("[tools]\ntaplo = \"0.10.0\"\n\"taplo\" = \"0.9.0\"\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_a_version_that_ends_with_a_dot_panics() {
        read_pins::<1>("[tools]\nshfmt = \"3.14.1.\"\n");
    }

    #[test]
    #[should_panic(expected = "is not a string")]
    fn read_pins_with_a_version_that_is_no_string_in_the_table_panics() {
        read_pins::<1>("[tools]\nstiles = { version = 6, os = [\"linux\"] }\n");
    }

    #[test]
    #[should_panic(
        expected = "the version of a pin of the mise file has no closing quotation mark"
    )]
    fn read_pins_with_a_version_without_its_closing_mark_panics() {
        read_pins::<1>("[tools]\ntaplo = \"0.10.0\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_an_alias_of_an_architecture_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"macos/aarch64\"] }\n");
    }

    #[test]
    #[should_panic(expected = "neither a version in double quotation marks nor a table")]
    fn read_pins_with_an_array_panics() {
        read_pins::<1>("[tools]\ntaplo = [\"0.10.0\"]\n");
    }

    #[test]
    #[should_panic(expected = "has an empty key")]
    fn read_pins_with_an_empty_key_panics() {
        read_pins::<1>("[tools]\n\"\" = \"0.10.0\"\n");
    }

    #[test]
    #[should_panic(expected = "lists no platform")]
    fn read_pins_with_an_empty_list_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [] }\n");
    }

    #[test]
    #[should_panic(expected = "a version that is not exact")]
    fn read_pins_with_an_empty_part_of_the_version_panics() {
        read_pins::<1>("[tools]\nshfmt = \"3..14.1\"\n");
    }

    #[test]
    #[should_panic(expected = "gives an empty version")]
    fn read_pins_with_an_empty_version_panics() {
        read_pins::<1>("[tools]\ntaplo = \"\"\n");
    }

    #[test]
    #[should_panic(expected = "has no `=`")]
    fn read_pins_with_an_entry_without_an_equals_sign_panics() {
        read_pins::<1>("[tools]\ntaplo : \"0.10.0\"\n");
    }

    #[test]
    #[should_panic(expected = "the key of a pin of the mise file holds an escape")]
    fn read_pins_with_an_escape_in_the_key_panics() {
        read_pins::<1>("[tools]\n\"npm\\u003Arenovate\" = \"44.240.0\"\n");
    }

    #[test]
    #[should_panic(expected = "the version of a pin of the mise file holds an escape")]
    fn read_pins_with_an_escape_in_the_version_panics() {
        read_pins::<1>("[tools]\ntaplo = \"0.10\\u002E0\"\n");
    }

    #[test]
    #[should_panic(expected = "is not a string")]
    fn read_pins_with_an_unclosed_version_in_the_table_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"0.6.0 }\n");
    }

    #[test]
    #[should_panic(expected = "does not match the number of pins")]
    fn read_pins_with_another_number_of_pins_panics() {
        read_pins::<2>("[tools]\ntaplo = \"0.10.0\"\n");
    }

    #[test]
    #[should_panic(expected = "holds a key other than")]
    fn read_pins_with_another_option_in_the_table_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"0.6.0\", version_prefix = \"stiles@\" }\n");
    }

    #[test]
    #[should_panic(expected = "a table other than `[tools]`")]
    fn read_pins_with_another_table_panics() {
        read_pins::<1>("[tools]\ntaplo = \"0.10.0\"\n\n[env]\nRUST_LOG = \"info\"\n");
    }

    #[test]
    #[should_panic(expected = "has text after its value")]
    fn read_pins_with_text_after_a_table_panics() {
        read_pins::<1>("[tools]\nzizmor = { version = \"1.32.0\" } os\n");
    }

    #[test]
    #[should_panic(expected = "has text after its value")]
    fn read_pins_with_text_after_a_version_panics() {
        read_pins::<1>("[tools]\ntaplo = \"0.10.0\" 0.11.0\n");
    }

    #[test]
    #[should_panic(expected = "gives the key `os` twice")]
    fn read_pins_with_the_platforms_twice_in_the_table_panics() {
        read_pins::<1>(
            "[tools]\nzizmor = { version = \"1.32.0\", os = [\"linux\"], os = [\"macos\"] }\n",
        );
    }

    #[test]
    #[should_panic(expected = "the `[tools]` table twice")]
    fn read_pins_with_the_table_twice_panics() {
        read_pins::<2>("[tools]\ntaplo = \"0.10.0\"\n[tools]\nzizmor = \"1.32.0\"\n");
    }

    #[test]
    #[should_panic(expected = "gives the key `version` twice")]
    fn read_pins_with_the_version_twice_in_the_table_panics() {
        read_pins::<1>("[tools]\ntaplo = { version = \"0.9.0\", version = \"0.10.0\" }\n");
    }

    #[test]
    #[should_panic(expected = "has no closing brace")]
    fn read_pins_with_two_keys_without_a_comma_panics() {
        read_pins::<1>("[tools]\nstiles = { version = \"0.6.0\" os = [\"linux\"] }\n");
    }

    #[test]
    #[should_panic(expected = "Rakko does not read")]
    fn read_pins_with_two_platforms_without_a_comma_panics() {
        platforms_in("[tools]\nstiles = { version = \"0.6.0\", os = [\"linux\" \"macos\"] }\n");
    }

    #[test]
    #[should_panic(expected = "pins no tool")]
    fn read_pins_without_a_pin_panics() {
        read_pins::<0>("[tools]\n# no pin yet\n");
    }

    #[test]
    fn requirement_of_a_pin_holds_no_program() {
        let pin = ToolPin {
            tool: "taplo",
            version: "0.10.0",
            platforms: None,
        };

        assert_eq!(pin.requirement().executable(), None);
    }

    #[test]
    fn requirement_of_a_pin_keeps_its_exact_version() {
        let pin = ToolPin {
            tool: "taplo",
            version: "0.10.0",
            platforms: None,
        };

        assert_eq!(
            pin.requirement().version(),
            &crate::tool::RequiredVersion::Exact(ToolVersion::new("0.10.0"))
        );
    }

    #[test]
    fn requirement_of_a_pin_keeps_its_tool() {
        let pin = ToolPin {
            tool: "npm:renovate",
            version: "44.240.0",
            platforms: None,
        };

        assert_eq!(pin.requirement().tool().get(), "npm:renovate");
    }

    #[test]
    fn requirement_of_a_pin_without_a_list_needs_every_platform() {
        let pin = ToolPin {
            tool: "taplo",
            version: "0.10.0",
            platforms: None,
        };

        assert_eq!(pin.requirement().platforms(), None);
    }

    #[test]
    fn tool_pin_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolPin>();
    }
}
