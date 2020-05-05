mod clean;
mod cli;
mod config;
mod domain;
mod engine;

use anyhow::{Context, Result};
use clean::clean_target_outputs;
use cli::{get_app_args, AppArgs};
use config::Config;
use engine::incremental::IncrementalRunner;
use engine::Engine;

fn main() -> Result<()> {
    let AppArgs {
        verbosity,
        project_dir,
        requested_targets,
        watch_mode_enabled,
        clean_before_run,
    } = get_app_args();

    stderrlog::new()
        .module(module_path!())
        .verbosity(verbosity + 2)
        .init()
        .unwrap();

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
