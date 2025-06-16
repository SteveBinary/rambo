use clap::{CommandFactory, Parser};
use clap_complete::{generate, Generator, Shell};

#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct RamboCli {
    #[clap(
        default_value = "*",
        help = "The glob pattern to match the files that shall be renamed. Use **/* to match all files recursively. Provide the pattern in quotes to prevent your shell from expanding it."
    )]
    pub(crate) pattern: String,

    #[clap(
        long,
        default_value_t = false,
        help = "Apply the renaming. For safety, the default behavior is a dry run."
    )]
    pub(crate) no_dry_run: bool,

    #[clap(
        long,
        short = 'i',
        default_value_t = false,
        help = "Match the pattern in a case insensitive way."
    )]
    pub(crate) case_insensitive: bool,

    #[clap(
        long,
        short,
        default_value = "%Y-%m-%d__%H-%M-%S",
        help = "The format of the renamed file (without the extension). See: https://docs.rs/chrono/0.4.41/chrono/format/strftime/index.html#specifiers"
    )]
    pub(crate) format: String,

    #[clap(
        long,
        short,
        allow_hyphen_values = true,
        value_name = "OFFSET",
        help = "Override the time zone offset relative to UTC, like '+01:00' or '-02:30'."
    )]
    pub(crate) time_offset: Option<String>,

    #[clap(
        long,
        short = 's',
        default_value_t = false,
        help = "Include and follow symlinks."
    )]
    pub(crate) include_symlinks: bool,

    #[clap(
        long,
        value_name = "SHELL",
        help = "Generate completion scripts for your shell."
    )]
    pub(crate) completions: Option<Shell>,
}

impl RamboCli {
    pub(crate) fn print_completions<G: Generator>(generator: G) {
        let ref mut cmd = Self::command();
        generate(
            generator,
            cmd,
            cmd.get_name().to_string(),
            &mut std::io::stdout(),
        );
    }
}
