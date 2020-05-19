use anyhow::Result;
use std::process::Child;

pub fn kill_and_wait(process: &mut Child) -> Result<()> {
    process.kill()?;
    process.wait()?;
    Ok(())
}
