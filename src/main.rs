mod config;
mod engine;
mod incremental;
mod target;
mod watcher;

use crate::config::Config;
use crate::engine::Engine;
use crate::incremental::IncrementalRunner;
use clap::{App, Arg};
use std::path::Path;

fn main() -> Result<(), String> {
    let arg_matches =
        App::new("Buildy")
            .about("An ultra-fast parallel build system for local iteration")
            .arg(
                Arg::with_name("project_dir")
                    .short("p")
                    .long("project")
                    .value_name("PROJECT_DIR")
                    .help("Directory of the project to build (in which 'buildy.yml' is located)")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("verbosity")
                    .short("v")
                    .multiple(true)
                    .help("Increases message verbosity"),
            )
            .arg(Arg::with_name("watch").short("w").long("watch").help(
                "Enable watch mode: rebuild targets and restart services on file system changes",
            ))
            .arg(
                Arg::with_name("targets")
                    .value_name("TARGETS")
                    .multiple(true)
                    .required(true)
                    .help("Targets to build"),
            )
            .get_matches();

    stderrlog::new()
        .module(module_path!())
        .verbosity(arg_matches.occurrences_of("verbosity") as usize + 2)
        .init()
        .unwrap();

    let project_dir = Path::new(arg_matches.value_of("project_dir").unwrap_or("."));
    let config_file_name = project_dir.join("buildy.yml");
    let requested_targets = arg_matches.values_of_lossy("targets").unwrap();
    let targets =
        Config::from_yml_file(&config_file_name)?.into_targets(&project_dir, &requested_targets)?;
    // TODO: Detect cycles.

    let checksum_dir = project_dir.join(".buildy");
    let incremental_runner = IncrementalRunner::new(&checksum_dir);

    let engine = Engine::new(project_dir, targets, incremental_runner);

    if arg_matches.is_present("watch") {
        engine.watch().map_err(|e| format!("Watch error: {}", e))
    } else {
        engine.build().map_err(|e| format!("Build error: {}", e))
    }
}
