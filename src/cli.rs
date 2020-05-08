use clap::{App, Arg};
use clap_generate::{generate, generators::Zsh};
use std::io::Write;

pub fn write_zsh_completion(buf: &mut dyn Write) {
    generate::<Zsh, _>(&mut get_app(), "zinoma", buf);
}

pub mod arg {
    pub static PROJECT_DIR: &str = "project_dir";
    pub static VERBOSITY: &str = "verbosity";
    pub static WATCH: &str = "watch";
    pub static CLEAN: &str = "clean";
    pub static GENERATE_ZSH_COMPLETION: &str = "generate_zsh_completion";
    pub static TARGETS: &str = "targets";
}

pub fn get_app() -> App<'static> {
    App::new("Žinoma")
        .about("Make your build flow incremental")
        .arg(
            Arg::with_name(arg::PROJECT_DIR)
                .short('p')
                .long("project")
                .takes_value(true)
                .value_name("PROJECT_DIR")
                .default_value(".")
                .hide_default_value(true)
                .about("Directory of the project to build (in which 'zinoma.yml' is located)"),
        )
        .arg(
            Arg::with_name(arg::VERBOSITY)
                .short('v')
                .multiple(true)
                .about("Increases message verbosity"),
        )
        .arg(Arg::with_name(arg::WATCH).short('w').long("watch").about(
            "Enable watch mode: rebuild targets and restart services on file system changes",
        ))
        .arg(
            Arg::with_name(arg::CLEAN)
                .long("clean")
                .about("Start by cleaning the target outputs"),
        )
        .arg(
            Arg::with_name(arg::GENERATE_ZSH_COMPLETION)
                .long("generate-zsh-completion")
                .hidden(true),
        )
        .arg(
            Arg::with_name(arg::TARGETS)
                .value_name("TARGETS")
                .multiple(true)
                .about("Targets to build"),
        )
}
