use std::process::Command;
use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn circular_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("buildy")?;
    cmd.arg("-p").arg("tests/integ/circular_dependency").arg("target_1");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Circular dependency"));

    Ok(())
}
