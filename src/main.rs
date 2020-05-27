mod clean;
mod cli;
mod config;
mod domain;
mod engine;

use anyhow::{Context, Result};
use clean::clean_target_output_paths;
use config::{ir, yaml};
use crossbeam::channel::{unbounded, Sender};
use engine::incremental::{remove_checksum_dir, remove_target_checksums};
use engine::Engine;
use std::convert::TryInto;
use std::path::Path;

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

    let project_dir = Path::new(arg_matches.value_of(cli::arg::PROJECT_DIR).unwrap());
    let projects = yaml::Projects::load(project_dir)?;
    let project_dirs = projects.get_project_dirs();
    let targets: ir::Targets = projects.try_into()?;
    let all_target_names = targets.get_target_names();

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
    let targets = targets.try_into_domain_targets(requested_targets)?;

    if arg_matches.is_present(cli::arg::CLEAN) {
        if has_requested_targets {
            targets.iter().try_for_each(remove_target_checksums)?;
        } else {
            project_dirs.into_iter().try_for_each(remove_checksum_dir)?;
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
