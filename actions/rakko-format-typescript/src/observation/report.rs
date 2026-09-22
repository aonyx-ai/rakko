use std::ops::RangeInclusive;
use std::path::PathBuf;

use super::Observation;
use crate::problem::OxfmtProblem;

/// The character that opens a sequence which colors what follows it
const ESCAPE: char = '\u{1b}';

/// The character that follows the escape of a sequence which carries a command
const SEQUENCE_OPEN: char = '[';

/// The characters that end a sequence which carries a command
const SEQUENCE_END: RangeInclusive<char> = '@'..='~';

/// The marks that open what oxfmt reports about something it could not do
///
/// Oxfmt draws its report for a reader when it believes that the stream
/// carries one, and with plain marks otherwise. The two shapes differ in the
/// characters that they draw and in nothing that they say, so the reading
/// knows both. Oxfmt offers no option to choose between them, and it gives a
/// build server the drawn one.
const FAILURE_MARKS: [&str; 2] = ["x ", "\u{d7} "];

/// The word that opens what oxfmt suggests about a failure
///
/// Oxfmt writes the suggestion at the end of a report, behind this word, and a
/// message that carries one reads the same way.
const SUGGESTION: &str = "help: ";

/// The character that opens the place of a failure
const PLACE_OPEN: char = '[';

/// The character that closes the place of a failure
const PLACE_CLOSE: char = ']';

/// The character that separates the parts of the place of a failure
const PLACE_SEPARATOR: char = ':';

/// The number of parts that the place of a failure splits into
const PLACE_PARTS: usize = 3;

/// The markers of the lines that report a configuration file oxfmt refused
///
/// Oxfmt writes the first one for a file that it cannot read as JSON and the
/// second for a file that holds a value of the wrong kind. It formats nothing
/// at all in either case, so the run never looked at a file of the project.
const REFUSED_CONFIGURATION: [&str; 2] = [
    "Failed to load configuration file",
    "Failed to parse configuration",
];

/// The marker of the line that reports a pattern without a match
///
/// Oxfmt writes it for a project that holds no file of the language, and for a
/// project whose ignore files exclude every one of them.
const UNMATCHED_PATTERN: &str = "Expected at least one target file";

/// Reads what one run of oxfmt produced
///
/// The files that a run listed arrive on the standard output stream, and what
/// oxfmt could not do arrives on the standard error stream. The reading walks
/// both, recognizes what carries an answer, and ignores the rest.
pub(super) fn read(stdout: &str, stderr: &str, succeeded: bool) -> Observation {
    // formattypescript[impl check.decorated]
    let stderr = plain(stderr);

    let mut observation = Observation {
        problems: Vec::new(),
        rejected_configuration: None,
        stderr: stderr.clone(),
        succeeded,
        unmatched_pattern: false,
    };

    // formattypescript[impl check.unformatted]
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        observation.problems.push(OxfmtProblem::Unformatted {
            path: PathBuf::from(line.trim()),
        });
    }

    for line in stderr.lines() {
        let line = line.trim_start();

        // formattypescript[impl check.configuration]
        if REFUSED_CONFIGURATION
            .iter()
            .any(|marker| line.starts_with(marker))
        {
            observation.rejected_configuration = Some(stderr.trim().to_owned());
        }

        // formattypescript[impl skip.unmatched]
        if line.starts_with(UNMATCHED_PATTERN) {
            observation.unmatched_pattern = true;
        }
    }

    observation.problems.extend(problems(&stderr));

    observation
}

/// Returns the text without the sequences that color it
///
/// Oxfmt colors its report when it believes that a person reads it, which is
/// what a build server gets, and it offers no option to stop that. The color
/// arrives as sequences that carry no text of their own, so removing them
/// leaves the report that a plain stream would have carried.
// formattypescript[impl check.decorated]
fn plain(text: &str) -> String {
    /// Where the reading stands between the characters of the text
    #[derive(Copy, Clone)]
    enum State {
        /// The character belongs to the text
        Text,

        /// The character follows the escape that opens a sequence
        Escape,

        /// The character belongs to a sequence that carries a command
        Sequence,
    }

    let mut plain = String::with_capacity(text.len());
    let mut state = State::Text;

    for character in text.chars() {
        state = match (state, character) {
            (State::Text, ESCAPE) => State::Escape,
            (State::Escape, SEQUENCE_OPEN) => State::Sequence,
            (State::Sequence, character) if !SEQUENCE_END.contains(&character) => State::Sequence,
            (State::Text | State::Escape, character) => {
                plain.push(character);
                State::Text
            }
            (State::Sequence, _) => State::Text,
        };
    }

    plain
}

/// Returns the place that a line of a report names, if it names one
///
/// Oxfmt draws the place of a failure into the frame of its report, so the
/// line carries characters of the frame around it. The place stands in
/// brackets and holds the path, the line, and the column, and a line that does
/// not hold those three parts is a line of the frame and nothing else.
fn place(line: &str) -> Option<(PathBuf, u32, u32)> {
    let line = line.trim_end();
    let line = line.strip_suffix(PLACE_CLOSE)?;
    let (_, place) = line.rsplit_once(PLACE_OPEN)?;

    let mut parts = place.rsplitn(PLACE_PARTS, PLACE_SEPARATOR);
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let path = parts.next().filter(|path| !path.is_empty())?;

    Some((PathBuf::from(path), line, column))
}

/// Returns the problems that oxfmt reported on its standard error stream
///
/// Oxfmt opens a report with a mark and then draws a frame around what it
/// knows: the place of the failure where it reached one, and the sentence that
/// it suggests about the failure where it has one. A report therefore runs
/// until the next one opens, and a line that says nothing this reading knows
/// belongs to the frame.
fn problems(stderr: &str) -> Vec<OxfmtProblem> {
    let mut problems = Vec::new();
    let mut message: Option<String> = None;
    let mut place: Option<(PathBuf, u32, u32)> = None;

    for line in stderr.lines() {
        if let Some(opened) = opening(line) {
            if let Some(message) = message.take() {
                problems.push(problem(message, place.take()));
            }

            message = Some(opened.trim_end().to_owned());
            continue;
        }

        if message.is_none() {
            continue;
        }

        if place.is_none()
            && let Some(found) = self::place(line)
        {
            place = Some(found);
            continue;
        }

        if let Some(suggestion) = line.trim_start().strip_prefix(SUGGESTION)
            && let Some(message) = message.as_mut()
        {
            message.push(' ');
            message.push_str(SUGGESTION);
            message.push_str(suggestion.trim_end());
        }
    }

    if let Some(message) = message {
        problems.push(problem(message, place));
    }

    problems
}

/// Returns the sentence of a report that a line opens, if it opens one
fn opening(line: &str) -> Option<&str> {
    let line = line.trim_start();

    FAILURE_MARKS
        .iter()
        .find_map(|mark| line.strip_prefix(mark))
}

/// Returns the problem that one report of oxfmt describes
///
/// A report that named a place belongs to that place of that file, and a
/// report that named none belongs to the project, because oxfmt writes the
/// path of the file into the sentence when it has no place to point at.
// formattypescript[impl check.invalid]
fn problem(message: String, place: Option<(PathBuf, u32, u32)>) -> OxfmtProblem {
    match place {
        Some((path, line, column)) => OxfmtProblem::Invalid {
            path,
            line,
            column,
            message,
        },
        None => OxfmtProblem::Unplaced { message },
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;
    use crate::problem::OxfmtProblem;

    /// Returns the sentence that a problem carries
    fn message(problem: &OxfmtProblem) -> String {
        match problem {
            OxfmtProblem::Invalid { message, .. } | OxfmtProblem::Unplaced { message } => {
                message.clone()
            }
            OxfmtProblem::Unformatted { path } => path.display().to_string(),
        }
    }

    /// What oxfmt writes about a file that it suggests something about
    const SUGGESTED_BLOCK: &str = "\n  x Expected function name\n   ,-[src/b.ts:1:10]\n 1 | \
                                   function ( {\n   :          ^\n   `----\n  help: Function \
                                   name is required in function declaration or named export\n";

    /// What oxfmt writes about a file that it could not open
    ///
    /// Oxfmt reaches no place in a file that it never read, so it writes the
    /// path of the file into the sentence and draws no frame around it.
    const UNPLACED_BLOCK: &str = "\n  x Failed to save '/home/otter/project/src/a.ts': Permission \
                                  denied (os error 13)\nError occurred when checking code style \
                                  in the above files.\n";

    /// What oxfmt writes when it cannot read the configuration of the project
    const REFUSED_CONFIGURATION: &str =
        "Failed to load configuration file.\nkey must be a string at line 1 column 3\n";

    /// What oxfmt writes when the pattern of a run matched no file
    const UNMATCHED: &str = "Expected at least one target file. All matched files may have been excluded by ignore \
         rules.\n";

    /// What oxfmt writes about the same file when it draws for a reader
    ///
    /// Oxfmt colors the text and draws the frame with box characters when it
    /// believes that a person reads its output, which is what a build server
    /// gets. It says nothing here that the plain shape does not say.
    const DRAWN_BLOCK: &str = "\n  \u{1b}[38;2;225;80;80;1m\u{d7}\u{1b}[0m \
                               \u{1b}[38;2;225;80;80;1mUnexpected token\u{1b}[0m\n   \
                               \u{256d}\u{2500}[\u{1b}[38;2;92;157;255;1msrc/broken.ts\u{1b}\
                               [0m:1:14]\n \u{1b}[2m1\u{1b}[0m \u{2502} export const = ;;;\n   \
                               \u{b7} \u{1b}[38;2;246;87;248m             \u{2500}\u{1b}[0m\n   \
                               \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\nError occurred when \
                               checking code style in the above files.\n";

    /// What oxfmt writes about a file that it could not parse
    ///
    /// Oxfmt draws the block with plain marks when it believes that nothing
    /// reads its output for a person.
    const PLAIN_BLOCK: &str = "\n  x Unexpected token\n   ,-[src/broken.ts:1:14]\n 1 | export \
                               const = ;;;\n   :              ^\n   `----\nError occurred when \
                               checking code style in the above files.\n";

    // formattypescript[verify check.decorated]
    #[test]
    fn read_of_a_drawn_failure_reports_what_the_plain_one_reports() {
        let observation = read("", DRAWN_BLOCK, false);

        assert_eq!(
            observation.problems(),
            read("", PLAIN_BLOCK, false).problems()
        );
    }

    // formattypescript[verify check.invalid]
    #[test]
    fn read_of_a_failure_with_a_suggestion_carries_the_suggestion() {
        let observation = read("", SUGGESTED_BLOCK, false);

        assert_eq!(
            observation.problems().first().map(message),
            Some(String::from(
                "Expected function name help: Function name is required in function declaration \
                 or named export"
            ))
        );
    }

    // formattypescript[verify check.invalid]
    #[test]
    fn read_of_a_failure_without_a_place_reports_the_sentence_of_oxfmt() {
        let observation = read("", UNPLACED_BLOCK, false);

        assert_eq!(
            observation.problems(),
            &[OxfmtProblem::Unplaced {
                message: String::from(
                    "Failed to save '/home/otter/project/src/a.ts': Permission denied (os error 13)"
                ),
            }]
        );
    }

    // formattypescript[verify check.unformatted]
    #[test]
    fn read_of_a_listed_file_reports_it_as_unformatted() {
        let observation = read("src/index.ts\nsrc/other.ts", "", false);

        assert_eq!(
            observation.problems(),
            &[
                OxfmtProblem::Unformatted {
                    path: path("src/index.ts"),
                },
                OxfmtProblem::Unformatted {
                    path: path("src/other.ts"),
                },
            ]
        );
    }

    // formattypescript[verify check.invalid]
    #[test]
    fn read_of_a_placed_failure_reports_the_place_that_oxfmt_named() {
        let observation = read("", PLAIN_BLOCK, false);

        assert_eq!(
            observation.problems(),
            &[OxfmtProblem::Invalid {
                path: path("src/broken.ts"),
                line: 1,
                column: 14,
                message: String::from("Unexpected token"),
            }]
        );
    }

    // formattypescript[verify check.configuration]
    #[test]
    fn read_of_a_refused_configuration_holds_what_oxfmt_wrote() {
        let observation = read("", REFUSED_CONFIGURATION, false);

        assert_eq!(
            observation.rejected_configuration().as_deref(),
            Some("Failed to load configuration file.\nkey must be a string at line 1 column 3")
        );
    }

    // formattypescript[verify skip.unmatched]
    #[test]
    fn read_of_a_run_that_matched_nothing_reports_the_unmatched_pattern() {
        let observation = read("", UNMATCHED, false);

        assert!(
            observation.unmatched_pattern(),
            "expected the pattern to be reported as unmatched"
        );
    }
}
