use clap::{App, Arg};
use clap_generate::{generate, generators::Zsh};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct AppArgs {
    pub verbosity: usize,
    pub project_dir: PathBuf,
    pub requested_targets: Option<Vec<String>>,
    pub watch_mode_enabled: bool,
    pub clean_before_run: bool,
    pub generate_zsh_completion: bool,
}

pub fn get_app_args(allowed_target_names: Option<Vec<&str>>) -> AppArgs {
    let arg_matches = get_app(allowed_target_names).get_matches();

    AppArgs {
        verbosity: arg_matches.occurrences_of("verbosity") as usize,
        project_dir: Path::new(arg_matches.value_of("project_dir").unwrap()).to_owned(),
        requested_targets: arg_matches.values_of_lossy("targets"),
        watch_mode_enabled: arg_matches.is_present("watch"),
        clean_before_run: arg_matches.is_present("clean"),
        generate_zsh_completion: arg_matches.is_present("generate_zsh_completion"),
    }
}

pub fn write_zsh_completion(buf: &mut dyn Write) {
    generate::<Zsh, _>(&mut get_app(None), "zinoma", buf);
}

fn get_app(allowed_target_names: Option<Vec<&str>>) -> App {
    let targets_arg = Arg::with_name("targets")
        .value_name("TARGETS")
        .multiple(true)
        .required_unless_one(&["clean", "generate_zsh_completion"])
        .about("Targets to build");

    let targets_arg = if let Some(allowed_target_names) = allowed_target_names {
        targets_arg.possible_values(&allowed_target_names)
    } else {
        targets_arg
    };

    App::new("Žinoma")
        .about("Make your build flow incremental")
        .arg(
            Arg::with_name("project_dir")
                .short('p')
                .long("project")
                .takes_value(true)
                .value_name("PROJECT_DIR")
                .default_value(".")
                .hide_default_value(true)
                .about("Directory of the project to build (in which 'zinoma.yml' is located)"),
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
            Arg::with_name("generate_zsh_completion")
                .long("generate-zsh-completion")
                .hidden(true),
        )
        .arg(targets_arg)
}
