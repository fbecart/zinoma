use clap::{App, Arg};
use std::path::{Path, PathBuf};

pub struct AppArgs {
    pub verbosity: usize,
    pub project_dir: PathBuf,
    pub requested_targets: Option<Vec<String>>,
    pub watch_mode_enabled: bool,
    pub clean_before_run: bool,
}

pub fn get_app_args(allowed_target_names: Option<Vec<&str>>) -> AppArgs {
    let arg_matches = get_app(allowed_target_names).get_matches();

    AppArgs {
        verbosity: arg_matches.occurrences_of("verbosity") as usize,
        project_dir: Path::new(arg_matches.value_of("project_dir").unwrap_or(".")).to_owned(),
        requested_targets: arg_matches.values_of_lossy("targets"),
        watch_mode_enabled: arg_matches.is_present("watch"),
        clean_before_run: arg_matches.is_present("clean"),
    }
}

fn get_app(allowed_target_names: Option<Vec<&str>>) -> App {
    let targets_arg = Arg::with_name("targets")
        .value_name("TARGETS")
        .multiple(true)
        .required_unless("clean")
        .about("Targets to build");

    let targets_arg = if let Some(allowed_target_names) = allowed_target_names {
        targets_arg.possible_values(&allowed_target_names)
    } else {
        targets_arg
    };

    App::new("Buildy")
        .about("An ultra-fast parallel build system for local iteration")
        .arg(
            Arg::with_name("project_dir")
                .short('p')
                .long("project")
                .takes_value(true)
                .value_name("PROJECT_DIR")
                .about("Directory of the project to build (in which 'buildy.yml' is located)"),
        )
        .arg(
            Arg::with_name("verbosity")
                .short('v')
                .multiple(true)
                .about("Increases message verbosity"),
        )
        .arg(Arg::with_name("watch").short('w').long("watch").about(
            "Enable watch mode: rebuild targets and restart services on file system changes",
        ))
        .arg(
            Arg::with_name("clean")
                .long("clean")
                .about("Start by cleaning the target outputs"),
        )
        .arg(targets_arg)
}
