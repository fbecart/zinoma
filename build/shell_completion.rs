include!("../src/cli.rs");

use std::ffi::OsString;

use clap_generate::{generate_to, generators};

pub fn generate_shell_completion_scripts(outdir: &OsString) {
    // Use clap to build completion files.
    let mut app = get_app();
    generate_to::<generators::Bash, _, _>(&mut app, "zinoma", outdir);
    generate_to::<generators::Zsh, _, _>(&mut app, "zinoma", outdir);
    generate_to::<generators::Fish, _, _>(&mut app, "zinoma", outdir);
    generate_to::<generators::PowerShell, _, _>(&mut app, "zinoma", outdir);
}
