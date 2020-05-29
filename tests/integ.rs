use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn circular_dependency() {
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg("tests/integ/circular_dependency")
        .arg("target_1");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency"));
}

#[test]
fn imports() {
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p").arg("tests/integ/imports").arg("target_2");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("This is target 1"));
}

#[test]
fn named_project() {
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg("tests/integ/named_project")
        .arg("target_3")
        .arg("my_project::target_3");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("This is target 1"));
}

#[test]
fn non_matching_imported_project_name() {
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg("tests/integ/non_matching_imported_project_name")
        .arg("--clean");
    cmd.assert()
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
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg("tests/integ/import_project_with_no_name")
        .arg("--clean");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to import noname"))
        .stderr(predicate::str::contains(
            "Project cannot be imported as it has no name",
        ));
}

#[test]
fn invalid_project_name() {
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg("tests/integ/invalid_project_name")
        .arg("--clean");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(":::: is not a valid project name"));
}
