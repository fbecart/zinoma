mod clean;
mod cli;
mod config;
mod domain;
mod engine;

use anyhow::{Context, Result};
use clean::clean_target_outputs;
use config::Config;
use crossbeam::channel::{unbounded, Sender};
use engine::incremental;
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
    let config = Config::load(project_dir.to_path_buf())?;
    let all_target_names = config.get_target_names();

    let arg_matches = cli::get_app()
        .mut_arg(cli::arg::TARGETS, |arg| {
            arg.possible_values(
                &all_target_names
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
            .required_unless(cli::arg::CLEAN)
        })
        .get_matches();

    let requested_targets = arg_matches.values_of_lossy(cli::arg::TARGETS);
    let has_requested_targets = requested_targets.is_some();
    let targets = config.into_targets(requested_targets)?;

    if arg_matches.is_present(cli::arg::CLEAN) {
        incremental::clean_checksums(project_dir, &targets)?;
        clean_target_outputs(&targets)?;
    }

    if has_requested_targets {
        let engine = Engine::new(targets);
        let (termination_sender, termination_events) = unbounded();
        terminate_on_ctrlc(termination_sender.clone())?;

        if arg_matches.is_present(cli::arg::WATCH) {
            engine
                .watch(termination_events)
                .with_context(|| "Watch error")
        } else {
            engine
                .build(termination_sender, termination_events)
                .with_context(|| "Build error")
        }
    } else {
        Ok(())
    }
}

fn terminate_on_ctrlc(termination_sender: Sender<()>) -> Result<()> {
    ctrlc::set_handler(move || termination_sender.send(()).unwrap())
        .with_context(|| "Failed to set Ctrl-C handler")
}
