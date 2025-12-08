#![forbid(unsafe_code)]

use clap::Parser;
use log::LevelFilter;
use rambo::RamboOptions;
use std::process::ExitCode;

mod cli;

fn main() -> ExitCode {
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(LevelFilter::Info)
        .init();

    let args = cli::RamboCli::parse();

    if let Some(completion_generator) = args.completions {
        cli::RamboCli::print_completions(completion_generator);
        return ExitCode::SUCCESS;
    }

    let options = RamboOptions {
        pattern: args.pattern,
        no_dry_run: args.no_dry_run,
        case_insensitive: args.case_insensitive,
        format: args.format,
        time_offset: args.time_offset,
        include_symlinks: args.include_symlinks,
    };

    rambo::run(options)
}
