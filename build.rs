include!("src/cli.rs");

use clap_generate::{generate_to, generators};
use std::env;
use std::fs::{self, File};
use std::path::Path;
use std::process;

fn main() {
    // OUT_DIR is set by Cargo and it's where any additional build artifacts
    // are written.
    let outdir = match env::var_os("OUT_DIR") {
        Some(outdir) => outdir,
        None => {
            eprintln!(
                "OUT_DIR environment variable not defined. \
                 Please file a bug: \
                 https://github.com/fbecart/zinoma/issues/new"
            );
            process::exit(1);
        }
    };
    fs::create_dir_all(&outdir).unwrap();

    let stamp_path = Path::new(&outdir).join("zinoma-stamp");
    if let Err(err) = File::create(&stamp_path) {
        panic!("failed to write {}: {}", stamp_path.display(), err);
    }

    // Use clap to build completion files.
    let mut app = get_app();
    generate_to::<generators::Bash, _, _>(&mut app, "zinoma", &outdir);
    generate_to::<generators::Zsh, _, _>(&mut app, "zinoma", &outdir);
    generate_to::<generators::Fish, _, _>(&mut app, "zinoma", &outdir);
    generate_to::<generators::PowerShell, _, _>(&mut app, "zinoma", &outdir);
}
