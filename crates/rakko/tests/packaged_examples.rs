//! The examples of the website build against the packages of the crates that
//! they use
//!
//! A reader of the website takes the crates of Rakko, such as `rakko` and
//! `rakko-cli`, from crates.io and not from this repository. The workspace
//! builds the examples with the crates of this repository. These crates hold
//! every file of their directories. A package
//! holds only the files that cargo puts in it. So the test packages every
//! crate of the workspace with `cargo package`, and it unpacks each package as
//! a registry does. Then it copies the examples into a new workspace. That
//! workspace asks for each crate of Rakko by its version, and it patches the
//! version with the unpacked package. It builds every target of the examples,
//! their tests included, against the packages and nothing else of this
//! repository.
//!
//! The test publishes nothing, and it runs cargo without a network. It copies
//! the lockfile of this workspace, so cargo finds every other crate in the
//! cache that the build of the workspace filled.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(
    clippy::missing_panics_doc,
    reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The directories of the workspace that hold the crates that Rakko publishes
const PUBLISHED: [&str; 4] = ["actions", "bundles", "commands", "crates"];

/// The directory of the examples, relative to the root of the workspace
const EXAMPLES: &str = "site/examples";

/// The page of the website whose text a test of an example reads, relative to
/// the root of the workspace
const LANDING_PAGE: &str = "site/content/_index.md";

/// Returns the root of the workspace
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Returns the names of the directories in a directory, sorted
fn directories(parent: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(parent)
        .expect("the test reads a directory")
        .map(|entry| entry.expect("the test reads an entry of a directory"))
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();

    names
}

/// Runs cargo in a directory, with its own target directory
///
/// The variables of Git leave the environment, because a run from the hook
/// of a commit sets them for another directory.
fn cargo(directory: &Path, arguments: &[&str], target: &Path) -> Output {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut command = Command::new(cargo);
    command
        .args(arguments)
        .current_dir(directory)
        .env("CARGO_TARGET_DIR", target);

    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }

    command.output().expect("the test starts cargo")
}

/// Writes a file, and the directories that hold it
fn write(path: &Path, text: &str) {
    let parent = path
        .parent()
        .expect("the test writes a file in a directory");
    std::fs::create_dir_all(parent).expect("the test creates the directory of a file");
    std::fs::write(path, text).expect("the test writes a file");
}

/// Copies a directory and everything in it
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the test creates a directory");

    for entry in std::fs::read_dir(from).expect("the test reads a directory") {
        let entry = entry.expect("the test reads an entry of a directory");
        let path = entry.path();
        let destination = to.join(entry.file_name());

        if path.is_dir() {
            copy_tree(&path, &destination);
        } else {
            std::fs::copy(&path, &destination).expect("the test copies a file");
        }
    }
}

/// Packages every crate that Rakko publishes, and unpacks each package as a
/// registry does
///
/// The run first removes the packages of an earlier run, which can hold an
/// earlier version of a crate.
///
/// Returns the directory of each unpacked package. The name of a directory is
/// the name of its crate, a hyphen, and its version. The paths that tar reads
/// are relative, because tar on Windows can read the drive of an absolute path
/// as the name of a host.
fn package(root: &Path, target: &Path) -> Vec<PathBuf> {
    let workspace = workspace();
    let crates: Vec<String> = PUBLISHED
        .iter()
        .flat_map(|parent| directories(&workspace.join(parent)))
        .collect();

    let mut arguments = vec![
        "package",
        "--no-verify",
        "--exclude-lockfile",
        "--allow-dirty",
        "--offline",
    ];
    for name in &crates {
        arguments.extend(["--package", name]);
    }

    let packages = target.join("package");
    if packages.exists() {
        std::fs::remove_dir_all(&packages).expect("the test removes the earlier packages");
    }

    let output = cargo(&workspace, &arguments, target);
    assert!(
        output.status.success(),
        "expected cargo to package the crates: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::create_dir(root.join("registry"))
        .expect("the test creates the directory of the registry");

    let mut unpacked = Vec::new();
    for entry in std::fs::read_dir(&packages).expect("cargo wrote the packages") {
        let archive = entry.expect("the test reads a package").path();
        let Some(file) = archive.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = file.strip_suffix(".crate") else {
            continue;
        };

        std::fs::copy(&archive, root.join(file)).expect("the test copies a package");
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(file)
            .arg("-C")
            .arg("registry")
            .current_dir(root)
            .status()
            .expect("the test starts tar");
        assert!(status.success(), "expected tar to unpack {file}");

        unpacked.push(root.join("registry").join(stem));
    }

    unpacked
}

/// Returns the manifest of the workspace of the examples
///
/// The manifest is the one of this workspace with three changes. Its members
/// are the examples. Each crate of Rakko in its dependencies has a version and
/// no path, as in the manifest of a reader. And a patch puts the unpacked
/// package in the place of each such version.
fn manifest(packages: &[PathBuf]) -> String {
    let root = std::fs::read_to_string(workspace().join("Cargo.toml"))
        .expect("the test reads the manifest of the workspace");

    let mut text = String::new();
    let mut lines = root.lines();
    while let Some(line) = lines.next() {
        if line.starts_with("members = [") {
            text.push_str(&format!("members = [\"{EXAMPLES}/*\"]\n"));
            for skipped in lines.by_ref() {
                if skipped.starts_with(']') {
                    break;
                }
            }
            continue;
        }

        let registered = line
            .split_once(" = { path = \"")
            .and_then(|(name, rest)| {
                rest.split_once("\", version = \"")
                    .map(|(_, version)| (name, version))
            })
            .and_then(|(name, version)| {
                version
                    .strip_suffix("\" }")
                    .map(|version| format!("{name} = \"{version}\""))
            });
        text.push_str(registered.as_deref().unwrap_or(line));
        text.push('\n');
    }

    text.push_str("\n[patch.crates-io]\n");
    for package in packages {
        let stem = package
            .file_name()
            .and_then(|name| name.to_str())
            .expect("the directory of a package has a name");
        let (name, _version) = stem
            .rsplit_once('-')
            .expect("the directory of a package ends in its version");
        text.push_str(&format!("{name} = {{ path = '{}' }}\n", package.display()));
    }

    text
}

/// Writes the workspace that builds the examples, and returns its directory
fn write_consumer(root: &Path, packages: &[PathBuf]) -> PathBuf {
    let workspace = workspace();
    let consumer = root.join("consumer");

    copy_tree(&workspace.join(EXAMPLES), &consumer.join(EXAMPLES));
    write(
        &consumer.join(LANDING_PAGE),
        &std::fs::read_to_string(workspace.join(LANDING_PAGE))
            .expect("the test reads the landing page"),
    );
    write(&consumer.join("Cargo.toml"), &manifest(packages));
    std::fs::copy(workspace.join("Cargo.lock"), consumer.join("Cargo.lock"))
        .expect("the test copies the lockfile of the workspace");

    consumer
}

#[test]
fn examples_build_against_the_packages_of_their_crates() {
    let root = tempfile::tempdir().expect("the test creates a temporary directory");
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("packaged-examples");
    let packages = package(root.path(), &target);
    let consumer = write_consumer(root.path(), &packages);

    let output = cargo(
        &consumer,
        &["build", "--offline", "--workspace", "--all-targets"],
        &target,
    );

    assert!(
        output.status.success(),
        "expected the examples to build against the packages: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
