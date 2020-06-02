use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::ffi;
use std::process::Command;

#[test]
fn circular_dependency() {
    zinoma_command("circular_dependency", &["target_1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency"));
}

#[test]
fn imports() {
    zinoma_command("imports", &["target_2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("This is target 1"));
}

#[test]
fn named_project() {
    zinoma_command("named_project", &["target_3", "my_project::target_3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("This is target 1"));
}

#[test]
fn non_matching_imported_project_name() {
    zinoma_command("non_matching_imported_project_name", &["--clean"])
        .assert()
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
    zinoma_command("import_project_with_no_name", &["--clean"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to import noname"))
        .stderr(predicate::str::contains(
            "Project cannot be imported as it has no name",
        ));
}

#[test]
fn invalid_project_name() {
    zinoma_command("invalid_project_name", &["--clean"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(":::: is not a valid project name"));
}

#[test]
fn root_input_path() {
    zinoma_command("root_input_path", &["--clean", "print_source"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Content of my source file"));

    zinoma_command("root_input_path", &["print_source"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Content of my source file").not())
        .stderr(predicate::str::contains("Build skipped (Not Modified)"));
}

#[test]
fn env_var_input() {
    zinoma_command("env_var_input", &["--clean", "my_target"])
        .assert()
        .success()
        .stderr(predicate::str::contains("my_target - Build success"));

    zinoma_command("env_var_input", &["my_target"])
        .assert()
        .success()
        .stderr(predicate::str::contains("my_target - Build skipped"));

    zinoma_command("env_var_input", &["my_target"])
        .env("TEST_VAR", "new_value")
        .assert()
        .success()
        .stderr(predicate::str::contains("my_target - Build success"));
}

#[test]
fn cmd_stdout_input() {
    zinoma_command("cmd_stdout_input", &["--clean", "random", "stable"])
        .assert()
        .success()
        .stderr(predicate::str::contains("random - Build success"))
        .stderr(predicate::str::contains("stable - Build success"));

    zinoma_command("cmd_stdout_input", &["random", "stable"])
        .assert()
        .success()
        .stderr(predicate::str::contains("random - Build success"))
        .stderr(predicate::str::contains("stable - Build skipped"));
}

fn zinoma_command<I, S>(integ_test_dir_name: &str, args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<ffi::OsStr>,
{
    let mut cmd = Command::cargo_bin("zinoma").unwrap();
    cmd.arg("-p")
        .arg(format!("tests/integ/{}", integ_test_dir_name))
        .args(args);
    cmd
}
