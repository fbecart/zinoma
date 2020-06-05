mod clean;
mod cli;
mod config;
mod domain;
mod engine;
mod run_script;
mod work_dir;

use anyhow::{Context, Result};
use clean::clean_target_output_paths;
use config::{ir, yaml};
use crossbeam::channel::{unbounded, Sender};
use engine::incremental::storage::delete_saved_env_state;
use engine::Engine;
use std::convert::TryInto;
use std::path::PathBuf;
use work_dir::remove_work_dir;

#[cfg(all(not(target_env = "msvc"), target_pointer_width = "64"))]
use jemallocator::Jemalloc;

#[cfg(all(not(target_env = "msvc"), target_pointer_width = "64"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    let arg_matches = cli::get_app().get_matches();

    stderrlog::new()
        .module(module_path!())
        .verbosity(arg_matches.occurrences_of(cli::arg::VERBOSITY) as usize + 2)
        .init()
        .unwrap();

    let root_project_dir = PathBuf::from(arg_matches.value_of(cli::arg::PROJECT_DIR).unwrap());
    let config = yaml::Config::load(&root_project_dir)?;
    let project_dirs = config.get_project_dirs();
    let config: ir::Config = config.try_into()?;
    let all_target_names = config.list_all_available_target_names();

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
    let targets = config.try_into_domain_targets(requested_targets)?;

    if arg_matches.is_present(cli::arg::CLEAN) {
        if has_requested_targets {
            targets.iter().try_for_each(delete_saved_env_state)?;
        } else {
            project_dirs.into_iter().try_for_each(remove_work_dir)?;
        }

        targets.iter().try_for_each(clean_target_output_paths)?;
    }

    if has_requested_targets {
        let engine = Engine::new(targets);
        let (termination_sender, termination_events) = unbounded();
        terminate_on_ctrlc(termination_sender.clone())?;

        if arg_matches.is_present(cli::arg::WATCH) {
            engine
                .watch(termination_events)
                .with_context(|| "Watch error")?;
        } else {
            engine
                .build(termination_sender, termination_events)
                .with_context(|| "Build error")?;
        }
    }

    Ok(())
}

fn terminate_on_ctrlc(termination_sender: Sender<()>) -> Result<()> {
    ctrlc::set_handler(move || termination_sender.send(()).unwrap())
        .with_context(|| "Failed to set Ctrl-C handler")
}
