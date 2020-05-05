use clap::{App, Arg};
use std::path::{Path, PathBuf};

pub struct AppArgs {
    pub verbosity: usize,
    pub project_dir: PathBuf,
    pub requested_targets: Vec<String>,
    pub watch_mode_enabled: bool,
    pub clean_before_run: bool,
}

pub fn get_app_args() -> AppArgs {
    let app = App::new("Buildy")
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
        .arg(
            Arg::with_name("targets")
                .value_name("TARGETS")
                .multiple(true)
                .required(true)
                .about("Targets to build"),
        );

    let arg_matches = app.get_matches();

    AppArgs {
        verbosity: arg_matches.occurrences_of("verbosity") as usize,
        project_dir: Path::new(arg_matches.value_of("project_dir").unwrap_or(".")).to_owned(),
        requested_targets: arg_matches.values_of_lossy("targets").unwrap(),
        watch_mode_enabled: arg_matches.is_present("watch"),
        clean_before_run: arg_matches.is_present("clean"),
    }
}
