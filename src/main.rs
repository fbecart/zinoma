#![recursion_limit = "256"]

mod clean;
mod cli;
mod config;
mod domain;
mod engine;
mod run_script;
mod work_dir;

use anyhow::{Context, Result};
use async_std::sync::Sender;
use clean::clean_target_output_paths;
use config::{ir, yaml};
use domain::TargetId;
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

pub static DEFAULT_CHANNEL_CAP: usize = 100;

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

    let root_target_ids = if let Some(requested_targets) = &requested_targets {
        TargetId::try_parse_many(requested_targets, &config.root_project_name).unwrap()
    } else {
        config.list_all_targets()
    };

    let targets = config.try_into_domain_targets(&root_target_ids)?;

    if arg_matches.is_present(cli::arg::CLEAN) {
        if requested_targets.is_some() {
            targets.values().try_for_each(delete_saved_env_state)?;
        } else {
            project_dirs.into_iter().try_for_each(remove_work_dir)?;
        }

        targets.values().try_for_each(clean_target_output_paths)?;
    }

    if requested_targets.is_some() {
        let engine = Engine::new(targets, root_target_ids);
        let (termination_sender, termination_events) =
            async_std::sync::channel(DEFAULT_CHANNEL_CAP);
        terminate_on_ctrlc(termination_sender.clone())?;

        async_std::task::block_on(async {
            if arg_matches.is_present(cli::arg::WATCH) {
                engine
                    .watch(termination_events)
                    .await
                    .with_context(|| "Watch error")
            } else {
                engine
                    .build(termination_sender, termination_events)
                    .await
                    .with_context(|| "Build error")
            }
        })?;
    }

    Ok(())
}

fn terminate_on_ctrlc(termination_sender: Sender<()>) -> Result<()> {
    ctrlc::set_handler(move || {
        async_std::task::block_on(async { termination_sender.send(()).await })
    })
    .with_context(|| "Failed to set Ctrl-C handler")
}
