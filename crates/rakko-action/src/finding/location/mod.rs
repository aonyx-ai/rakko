/// The path of a directory that a problem is in
mod directory_path;
/// The path of a file that a problem is in
mod file_path;
/// The position of a problem in a file
mod position;
/// The range of a file that a problem covers
mod span;

pub use self::directory_path::{DirectoryPath, ParseDirectoryPathError};
pub use self::file_path::{FilePath, ParseFilePathError};
pub use self::position::{Column, Line, Position};
pub use self::span::Span;

/// How precisely an action can place a problem in a project
///
/// The tools that actions wrap do not agree on how much they know about the
/// place of a problem. A formatter knows that a file differs from its
/// formatted form, and it knows no line in that file. A linter knows a line
/// and a column. A check of the dependencies of a project knows no path at
/// all.
///
/// A location therefore names the level that it speaks at, and each level
/// carries what that level knows and nothing more. A reader of a location
/// gets the level first, and the level alone tells the reader what it can
/// show. A level that says less is an honest answer, not a defect: an action
/// gives the level that its tool supports, and it never claims a precision
/// that the tool did not give it.
///
/// Every path in a location is relative to the project root. A reader, a
/// machine, and a code host therefore see the same path for the same file.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Location {
    /// The problem belongs to the project, and to no path in it
    ///
    /// A check of the dependencies of a project reports at this level,
    /// because its answer is about the project and not about one file.
    // action[impl location.project]
    Project,
    /// The problem belongs to a directory
    ///
    /// A tool that reports per directory rather than per file reports at this
    /// level. A coverage report is one example.
    // action[impl location.directory]
    Directory {
        /// The path of the directory that the problem is in
        path: DirectoryPath,
    },
    /// The problem belongs to a file, and to no place in that file
    ///
    /// A formatter reports at this level, because it knows that a file
    /// differs from its formatted form and knows no line in it.
    // action[impl location.file]
    File {
        /// The path of the file that the problem is in
        path: FilePath,
    },
    /// The problem is at one position in a file
    ///
    /// Most linters report at this level. The position always has a line, and
    /// it has a column when the tool gives one.
    // action[impl location.position+2]
    Position {
        /// The path of the file that the problem is in
        path: FilePath,
        /// The position of the problem in that file
        position: Position,
    },
    /// The problem covers a range of a file
    ///
    /// A compiler and a formatter report at this level, because a problem
    /// that they find can start on one line and end on a later one.
    // action[impl location.span]
    Span {
        /// The path of the file that the problem is in
        path: FilePath,
        /// The range of the file that the problem covers
        span: Span,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    // action[verify location.directory]
    #[test]
    fn directory_level_holds_directory_path() {
        let location = Location::Directory {
            path: DirectoryPath::try_from("crates/rakko").unwrap(),
        };

        let Location::Directory { path } = &location else {
            panic!("expected Directory level");
        };
        assert_eq!(path.get(), Path::new("crates/rakko"));
    }

    // action[verify location.file]
    #[test]
    fn file_level_holds_file_path() {
        let location = Location::File {
            path: FilePath::try_from("Cargo.toml").unwrap(),
        };

        let Location::File { path } = &location else {
            panic!("expected File level");
        };
        assert_eq!(path.get(), Path::new("Cargo.toml"));
    }

    // action[verify location.position+2]
    #[test]
    fn position_level_holds_file_path_and_position() {
        let location = Location::Position {
            path: FilePath::try_from("src/main.rs").unwrap(),
            position: Position::builder().line(10).column(3).build(),
        };

        let Location::Position { path, position } = &location else {
            panic!("expected Position level");
        };
        assert_eq!(
            (path.get(), position.line().get()),
            (Path::new("src/main.rs"), 10),
        );
    }

    // action[verify location.project]
    #[test]
    fn project_level_holds_no_path() {
        let location = Location::Project;

        assert_eq!(location, Location::Project);
    }

    // action[verify location.span]
    #[test]
    fn span_level_holds_file_path_and_span() {
        let span = Span::builder()
            .start(Position::builder().line(1).build())
            .end(Position::builder().line(3).build())
            .build();

        let location = Location::Span {
            path: FilePath::try_from("src/lib.rs").unwrap(),
            span,
        };

        let Location::Span { path, span: range } = &location else {
            panic!("expected Span level");
        };
        assert_eq!((path.get(), range), (Path::new("src/lib.rs"), &span));
    }
}
