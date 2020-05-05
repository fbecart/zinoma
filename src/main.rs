mod clean;
mod config;
mod domain;
mod engine;

use anyhow::{Context, Result};
use clap::{App, Arg};
use clean::clean_target_outputs;
use config::Config;
use engine::incremental::IncrementalRunner;
use engine::Engine;
use std::path::Path;

fn main() -> Result<()> {
    let arg_matches =
        App::new("Buildy")
            .about("An ultra-fast parallel build system for local iteration")
            .arg(
                Arg::with_name("project_dir")
                    .short("p")
                    .long("project")
                    .takes_value(true)
                    .value_name("PROJECT_DIR")
                    .help("Directory of the project to build (in which 'buildy.yml' is located)"),
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
                Arg::with_name("clean")
                    .long("clean")
                    .help("Start by cleaning the target outputs"),
            )
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
    let requested_targets = arg_matches.values_of_lossy("targets").unwrap();
    let watch_mode_enabled = arg_matches.is_present("watch");
    let clean_before_run = arg_matches.is_present("clean");

    let config = Config::load(&project_dir)?;
    let targets = config.into_targets(&project_dir, &requested_targets)?;

    let checksum_dir = project_dir.join(".buildy");
    let incremental_runner = IncrementalRunner::new(&checksum_dir);

    if clean_before_run {
        incremental_runner.clean_checksums(&targets)?;
        clean_target_outputs(&targets)?;
    }

    let engine = Engine::new(targets, incremental_runner);

    crossbeam::scope(|scope| {
        if watch_mode_enabled {
            engine.watch(scope).with_context(|| "Watch error")
        } else {
            engine.build(scope).with_context(|| "Build error")
        }
    })
    .map_err(|_| anyhow::anyhow!("Unknown crossbeam parallelism failure (thread panicked)"))?
}
