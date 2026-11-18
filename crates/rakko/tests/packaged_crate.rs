//! A crate that a project receives as a package declares the pins of its mise
//! file
//!
//! A project gets an action as a package from a registry, and not as the
//! checkout that the author built. So the test packages a crate with
//! `cargo package`, and it unpacks the package as a registry would. It builds
//! a program that reads the tools of the crate. It then removes the package
//! before the program runs. The program still gives every pin, because the
//! crate embedded its mise file when it compiled.
//!
//! The test runs cargo without a network. It copies the lockfile of this
//! workspace into the program. So cargo finds every crate that it needs in
//! the cache that the build of the workspace filled.

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
use std::process::Command;

/// The mise file of the packaged crate
///
/// It holds both forms of a pin, and a tool with a build on one platform.
const PINS: &str = "# The pins of the crate.\n[tools]\ntaplo = \"0.10.0\"\n\"github:aonyx-ai/stiles\" = { version = \"stiles@0.6.0\", os = [\"linux\", \"macos/arm64\"] }\n";

/// The name of the packaged crate
const PACKAGED: &str = "packaged-pins";

/// The source of the packaged crate, an action that runs the tools of its
/// mise file
const LIBRARY: &str = r#"//! An action that a project receives as a package

use rakko::{Action, Context, Documented, Name, Outcome, Tools, action_name};

/// Runs the tools of the mise file of the crate
#[derive(Documented)]
pub struct CheckPins;

impl Action for CheckPins {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-pins")
    }

    fn bundled_tools(&self) -> Tools {
        rakko::bundled_tools!()
    }

    async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
        Outcome::Passed { summary: None }
    }
}
"#;

/// The source of the program that reads the tools of the packaged crate
const PROGRAM: &str = r#"use rakko::Action;

fn main() {
    for requirement in packaged_pins::CheckPins.tools() {
        println!("{requirement:?}");
    }
}
"#;

/// Returns the directory of the crate under test
fn manifest_directory() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Returns the cargo that runs the test
fn cargo() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

/// Runs cargo in a directory and stops the test when it fails
fn run_cargo(directory: &Path, arguments: &[&str], target: &Path) {
    let output = Command::new(cargo())
        .args(arguments)
        .current_dir(directory)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("the test starts cargo");

    assert!(
        output.status.success(),
        "expected `cargo {}` to succeed: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Writes a file, and the directories that hold it
fn write(path: &Path, text: &str) {
    let parent = path
        .parent()
        .expect("the test writes a file in a directory");
    std::fs::create_dir_all(parent).expect("the test creates the directory of a file");
    std::fs::write(path, text).expect("the test writes a file");
}

/// Writes the crate that the test packages, and returns its directory
fn write_crate(root: &Path) -> PathBuf {
    let directory = root.join("source");
    let contract = manifest_directory().display().to_string();

    write(
        &directory.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{PACKAGED}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nrakko = {{ path = '{contract}', version = \"{}\" }}\n",
            env!("CARGO_PKG_VERSION")
        ),
    );
    write(&directory.join("mise.rakko.toml"), PINS);
    write(&directory.join("src/lib.rs"), LIBRARY);

    directory
}

/// Writes the program that reads the tools of the unpacked crate, and
/// returns its directory
fn write_program(root: &Path, unpacked: &Path) -> PathBuf {
    let directory = root.join("program");
    let contract = manifest_directory().display().to_string();
    let package = unpacked.display().to_string();

    write(
        &directory.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"program\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\n{PACKAGED} = {{ path = '{package}' }}\nrakko = \"{}\"\n\n\
             [patch.crates-io]\nrakko = {{ path = '{contract}' }}\n",
            env!("CARGO_PKG_VERSION")
        ),
    );
    write(&directory.join("src/main.rs"), PROGRAM);

    let lockfile = manifest_directory().join("../../Cargo.lock");
    std::fs::copy(lockfile, directory.join("Cargo.lock"))
        .expect("the test copies the lockfile of the workspace");

    directory
}

/// Packages the crate and unpacks the package, as a registry does, and
/// returns the directory of the unpacked package
///
/// The crate is removed afterwards, so that only the package remains. The
/// paths that tar reads are relative, because tar on Windows can read the
/// drive of an absolute path as the name of a host.
///
/// The package holds no lockfile. To write one, cargo looks for `rakko` in
/// its cache of the registry index. That cache has no entry for `rakko` on a
/// machine that never fetched it from the registry. The program resolves
/// `rakko` to this checkout instead.
fn package(root: &Path, target: &Path) -> PathBuf {
    let source = write_crate(root);
    run_cargo(
        &source,
        &[
            "package",
            "--no-verify",
            "--exclude-lockfile",
            "--allow-dirty",
            "--offline",
        ],
        target,
    );

    let archive = format!("{PACKAGED}-0.1.0.crate");
    std::fs::copy(target.join("package").join(&archive), root.join(&archive))
        .expect("the test copies the package");
    std::fs::remove_dir_all(&source).expect("the test removes the crate that it packaged");

    let unpacked = root.join("registry");
    std::fs::create_dir(&unpacked).expect("the test creates the directory of the registry");

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg("registry")
        .current_dir(root)
        .status()
        .expect("the test starts tar");
    assert!(status.success(), "expected tar to unpack the package");

    unpacked.join(format!("{PACKAGED}-0.1.0"))
}

#[test]
fn packaged_crate_declares_its_pins_without_its_files() {
    const TOOLS: rakko::Tools = rakko::__tools_of_mise_file!(PINS);
    let root = tempfile::tempdir().expect("the test creates a temporary directory");
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("packaged-crate");
    let unpacked = package(root.path(), &target);
    let program = write_program(root.path(), &unpacked);
    run_cargo(&program, &["build", "--offline"], &target);
    std::fs::remove_dir_all(&unpacked).expect("the test removes the unpacked package");

    let output = Command::new(
        target
            .join("debug")
            .join(format!("program{}", std::env::consts::EXE_SUFFIX)),
    )
    .output()
    .expect("the test runs the program");

    let expected: String = TOOLS
        .requirements()
        .iter()
        .map(|requirement| format!("{requirement:?}\n"))
        .collect();
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}
