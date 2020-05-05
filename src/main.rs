mod clean;
mod cli;
mod config;
mod domain;
mod engine;

use anyhow::{Context, Result};
use clean::clean_target_outputs;
use cli::get_app_args;
use config::Config;
use engine::incremental::IncrementalRunner;
use engine::Engine;

fn main() -> Result<()> {
    let app_args = get_app_args(None);

    stderrlog::new()
        .module(module_path!())
        .verbosity(app_args.verbosity + 2)
        .init()
        .unwrap();

    let config = Config::load(&app_args.project_dir)?;
    let all_target_names = config.get_target_names();

    let app_args = get_app_args(Some(all_target_names));

    let targets = config.into_targets(&app_args.project_dir, &app_args.requested_targets)?;

    let checksum_dir = app_args.project_dir.join(".buildy");
    let incremental_runner = IncrementalRunner::new(&checksum_dir);

    if app_args.clean_before_run {
        incremental_runner.clean_checksums(&targets)?;
        clean_target_outputs(&targets)?;
    }

    if app_args.requested_targets.is_some() {
        let engine = Engine::new(targets, incremental_runner);

        crossbeam::scope(|scope| {
            if app_args.watch_mode_enabled {
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
