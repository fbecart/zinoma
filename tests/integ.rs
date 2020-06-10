use assert_cmd::prelude::*;
use predicate::str::contains;
use predicates::prelude::*;
use std::ffi;
use std::process::Command;

#[test]
fn circular_dependency() {
    zinoma_command("circular_dependency", &["target_1"])
        .assert()
        .failure()
        .stderr(contains("Circular dependency"));
}

#[test]
fn imports() {
    zinoma_command("imports", &["target_2"])
        .assert()
        .success()
        .stdout(contains("This is target 1"));
}

#[test]
fn named_project() {
    zinoma_command("named_project", &["target_3", "my_project::target_3"])
        .assert()
        .success()
        .stdout(contains("This is target 1"));
}

#[test]
fn non_matching_imported_project_name() {
    zinoma_command("non_matching_imported_project_name", &["--clean"])
        .assert()
        .failure()
        .stderr(contains("Failed to import incorrect_subproject_name"))
        .stderr(contains(
            "The project should be imported with name subproject_name",
        ));
}

#[test]
fn import_project_with_no_name() {
    zinoma_command("import_project_with_no_name", &["--clean"])
        .assert()
        .failure()
        .stderr(contains("Failed to import noname"))
        .stderr(contains("Project cannot be imported as it has no name"));
}

#[test]
fn invalid_project_name() {
    zinoma_command("invalid_project_name", &["--clean"])
        .assert()
        .failure()
        .stderr(contains(":::: is not a valid project name"));
}

#[test]
fn root_input_path() {
    zinoma_command("root_input_path", &["--clean", "print_source"])
        .assert()
        .success()
        .stdout(contains("Content of my source file"));

    zinoma_command("root_input_path", &["print_source"])
        .assert()
        .success()
        .stdout(contains("Content of my source file").not())
        .stderr(contains("Build skipped (Not Modified)"));
}

#[test]
fn cmd_stdout_input() {
    let integ_test_dir_name = if cfg!(windows) {
        "cmd_stdout_input_windows"
    } else if cfg!(target_os = "macos") {
        "cmd_stdout_input_macos"
    } else {
        "cmd_stdout_input"
    };

    zinoma_command(integ_test_dir_name, &["--clean", "changing", "stable"])
        .assert()
        .success()
        .stderr(contains("changing - Build success"))
        .stderr(contains("stable - Build success"));

    zinoma_command(integ_test_dir_name, &["changing", "stable"])
        .assert()
        .success()
        .stderr(contains("changing - Build success"))
        .stderr(contains("stable - Build skipped"));
}

#[test]
fn dependency_output_as_input() {
    zinoma_command("dependency_output_as_input", &["--clean", "print"])
        .assert()
        .success()
        .stdout(contains("Intermediate build result"));
}

#[test]
fn circular_dependency_in_resources() {
    zinoma_command(
        "circular_dependency_in_resources",
        &["target_1", "target_2"],
    )
    .assert()
    .failure()
    .stderr(contains("Circular dependency"));
}

#[test]
#[cfg(target_os = "macos")]
fn incremental_multi_projects_build() {
    zinoma_command(
        "incremental_multi_projects_build",
        &["--clean", "print_outputs"],
    )
    .assert()
    .success()
    .stderr(contains("print_outputs - Build success"));

    zinoma_command("incremental_multi_projects_build", &["print_outputs"])
        .assert()
        .success()
        .stderr(contains("print_outputs - Build skipped"));

    zinoma_command(
        "incremental_multi_projects_build",
        &["--clean", "cmd_output::build"],
    )
    .assert()
    .success()
    .stderr(contains("cmd_output::build - Build success"));

    zinoma_command("incremental_multi_projects_build", &["print_outputs"])
        .assert()
        .success()
        .stderr(contains("print_outputs - Build success"));

    zinoma_command(
        "incremental_multi_projects_build",
        &["--clean", "fs_output::build"],
    )
    .assert()
    .success()
    .stderr(contains("fs_output::build - Build success"));

    zinoma_command("incremental_multi_projects_build", &["print_outputs"])
        .assert()
        .success()
        .stderr(contains("print_outputs - Build success"));
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
