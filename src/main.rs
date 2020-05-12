mod clean;
mod cli;
mod config;
mod domain;
mod engine;

use anyhow::{Context, Result};
use clean::clean_target_outputs;
use config::Config;
use engine::incremental::IncrementalRunner;
use engine::Engine;
use std::path::Path;

fn main() -> Result<()> {
    let arg_matches = cli::get_app().get_matches();

    stderrlog::new()
        .module(module_path!())
        .verbosity(arg_matches.occurrences_of(cli::arg::VERBOSITY) as usize + 2)
        .init()
        .unwrap();

    let project_dir = Path::new(arg_matches.value_of(cli::arg::PROJECT_DIR).unwrap());
    let config = Config::load(project_dir)?;
    let all_target_names = config.get_target_names();

    let arg_matches = cli::get_app()
        .mut_arg(cli::arg::TARGETS, |arg| {
            arg.possible_values(&all_target_names)
                .required_unless(cli::arg::CLEAN)
        })
        .get_matches();

    let requested_targets = arg_matches.values_of_lossy(cli::arg::TARGETS);
    let targets = config.into_targets(project_dir, &requested_targets)?;

    let checksum_dir = project_dir.join(".zinoma");
    let incremental_runner = IncrementalRunner::new(&checksum_dir);

    if arg_matches.is_present(cli::arg::CLEAN) {
        incremental_runner.clean_checksums(&targets)?;
        clean_target_outputs(&targets)?;
    }

    if requested_targets.is_some() {
        let engine = Engine::new(targets, incremental_runner);

        crossbeam::scope(|scope| {
            if arg_matches.is_present(cli::arg::WATCH) {
                engine.watch(scope).with_context(|| "Watch error")
            } else {
                engine.build(scope).with_context(|| "Build error")
            }
        })
        .map_err(|_| {
            anyhow::anyhow!("Unknown crossbeam parallelism failure (thread panicked)")
        })??;
    }

    Ok(())
}
