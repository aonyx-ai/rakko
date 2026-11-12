//! The tests of the guide "Test an Action"

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

// region: imports
use std::fs;

use otter_check_readme::{CheckReadme, CheckReadmeArgs};
use rakko::{Access, Action, Args, ArgsValues, ArgumentValue, Context, Outcome, argument_name};
use rakko_test_utils::run_checking_access;
// endregion: imports

// region: values
/// Returns the values of a run with `--fix`, as the command line gives them
fn values_with_fix() -> ArgsValues {
    ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
}
// endregion: values

// region: access
#[test]
fn access_with_fix_is_write() {
    let project = tempfile::tempdir().unwrap();
    let context = Context::builder().root(project.path()).build();
    let args = CheckReadmeArgs::from_values(&values_with_fix()).unwrap();

    let access = CheckReadme.access(&context, &args);

    assert_eq!(access, Access::Write);
}
// endregion: access

#[test]
fn access_without_fix_is_read() {
    let project = tempfile::tempdir().unwrap();
    let context = Context::builder().root(project.path()).build();

    let access = CheckReadme.access(&context, &CheckReadmeArgs::default());

    assert_eq!(access, Access::Read);
}

#[tokio::test]
async fn run_with_fix_changes_the_project() {
    let project = tempfile::tempdir().unwrap();
    let context = Context::builder().root(project.path()).build();
    let args = CheckReadmeArgs::from_values(&values_with_fix()).unwrap();

    let outcome = run_checking_access(&CheckReadme, &context, &args).await;

    assert!(matches!(outcome, Outcome::Changed { .. }));
}

// region: fix
#[tokio::test]
async fn run_with_fix_writes_readme() {
    let project = tempfile::tempdir().unwrap();
    let context = Context::builder().root(project.path()).build();
    let args = CheckReadmeArgs::from_values(&values_with_fix()).unwrap();

    run_checking_access(&CheckReadme, &context, &args).await;

    assert!(project.path().join("README.md").exists());
}
// endregion: fix

// region: passes
#[tokio::test]
async fn run_with_readme_passes() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("README.md"), "# Otter\n").unwrap();
    let context = Context::builder().root(project.path()).build();

    let outcome = run_checking_access(&CheckReadme, &context, &CheckReadmeArgs::default()).await;

    assert!(matches!(outcome, Outcome::Passed { .. }));
}
// endregion: passes

// region: fails
#[tokio::test]
async fn run_without_readme_fails() {
    let project = tempfile::tempdir().unwrap();
    let context = Context::builder().root(project.path()).build();

    let outcome = run_checking_access(&CheckReadme, &context, &CheckReadmeArgs::default()).await;

    assert!(matches!(outcome, Outcome::Failed { .. }));
}
// endregion: fails
