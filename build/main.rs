mod config_schema;
mod shell_completion;

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

    shell_completion::generate_shell_completion_scripts(&outdir);
    config_schema::generate_config_json_schema(&outdir);
}
