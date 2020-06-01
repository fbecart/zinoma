use assert_cmd::assert::Assert;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::ffi;
use std::process::Command;

#[test]
fn circular_dependency() {
    assert_zinoma("circular_dependency", &["target_1"])
        .failure()
        .stderr(predicate::str::contains("Circular dependency"));
}

#[test]
fn imports() {
    assert_zinoma("imports", &["target_2"])
        .success()
        .stdout(predicate::str::contains("This is target 1"));
}

#[test]
fn named_project() {
    assert_zinoma("named_project", &["target_3", "my_project::target_3"])
        .success()
        .stdout(predicate::str::contains("This is target 1"));
}

#[test]
fn non_matching_imported_project_name() {
    assert_zinoma("non_matching_imported_project_name", &["--clean"])
        .failure()
        .stderr(predicate::str::contains(
            "Failed to import incorrect_subproject_name",
        ))
        .stderr(predicate::str::contains(
            "The project should be imported with name subproject_name",
        ));
}

#[test]
fn import_project_with_no_name() {
    assert_zinoma("import_project_with_no_name", &["--clean"])
        .failure()
        .stderr(predicate::str::contains("Failed to import noname"))
        .stderr(predicate::str::contains(
            "Project cannot be imported as it has no name",
        ));
}

#[test]
fn invalid_project_name() {
    assert_zinoma("invalid_project_name", &["--clean"])
        .failure()
        .stderr(predicate::str::contains(":::: is not a valid project name"));
}

#[test]
fn root_input_path() {
    assert_zinoma("root_input_path", &["--clean", "print_source"])
        .success()
        .stdout(predicate::str::contains("Content of my source file"));

    assert_zinoma("root_input_path", &["print_source"])
        .success()
        .stdout(predicate::str::contains("Content of my source file").not())
        .stderr(predicate::str::contains("Build skipped (Not Modified)"));
}

fn assert_zinoma<I, S>(integ_test_dir_name: &str, args: I) -> Assert
where
    I: IntoIterator<Item = S>,
    S: AsRef<ffi::OsStr>,
{
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg(format!("tests/integ/{}", integ_test_dir_name))
        .args(args);
    cmd.assert()
}
