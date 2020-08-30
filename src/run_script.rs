use async_process::Command;
use async_std::path::Path;

pub fn build_command(script: &str, dir: &Path) -> Command {
    let (program, run_arg) = if cfg!(windows) {
        let comspec = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
        (comspec, "/C")
    } else {
        ("/bin/sh".into(), "-ce")
    };

    let mut command = Command::new(program);
    command.arg(run_arg).arg(script).current_dir(dir);

    command
}
