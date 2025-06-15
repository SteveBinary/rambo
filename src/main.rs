#![forbid(unsafe_code)]

use crate::extract::extract_creation_datetime_from_media_source;
use crate::glob::evaluate_files_from_glob_pattern;
use crate::statistics::Statistics;

use chrono::FixedOffset;
use clap::Parser;
use log::LevelFilter;
use nom_exif::{MediaParser, MediaSource};
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

mod cli;
mod extract;
mod glob;
mod statistics;

struct MediaAsset {
    media_source: MediaSource<File>,
    path_buf: PathBuf,
}

fn main() -> ExitCode {
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .filter_level(LevelFilter::Info)
        .init();

    let args = cli::RamboCli::parse();
    let mut statistics = Statistics::new();

    let current_working_directory = match std::env::current_dir() {
        Ok(working_directory) => format!(
            "{}{}",
            working_directory.display(),
            std::path::MAIN_SEPARATOR
        ),
        Err(error) => {
            log::error!("Cannot determine current working directory: {}", error);
            return ExitCode::FAILURE;
        }
    };

    let time_offset = match args.time_offset {
        None => None,
        Some(time_offset_string) => match FixedOffset::from_str(&time_offset_string) {
            Ok(time_offset) => Some(time_offset),
            Err(error) => {
                log::error!("Time offset '{}' is invalid: {}", time_offset_string, error);
                return ExitCode::FAILURE;
            }
        },
    };

    let Some((paths, errors)) =
        evaluate_files_from_glob_pattern(&args.pattern, args.case_insensitive)
    else {
        return ExitCode::FAILURE;
    };

    if errors.is_empty().not() {
        statistics.failed_files += errors.len() as u64;

        log::warn!(
            "Some paths could not be read to determine if their contents match the given glob pattern '{}'. \
            Make sure you have the permissions for these paths and symlinks are not broken.",
            args.pattern
        );

        for error in errors.iter() {
            log::warn!("{}", error);
        }
    }

    if paths.is_empty() && errors.is_empty() {
        log::warn!(
            "No media files will be processed. Make sure the glob pattern '{}' is correct.",
            args.pattern
        );

        return ExitCode::SUCCESS;
    } else if paths.is_empty() && errors.is_empty().not() {
        log::warn!(
            "No media files will be processed. Make sure the glob pattern '{}' is correct and you have adequate permissions.",
            args.pattern
        );

        return ExitCode::FAILURE;
    }

    let media_assets = get_media_assets_from_path_bufs(paths);

    let mut media_parser = MediaParser::new();

    for media_asset in media_assets {
        let media_asset = match media_asset {
            Ok(media_asset) => media_asset,
            Err((path_buf, error)) => {
                statistics.failed_files += 1;
                log::warn!(
                    "Cannot process {}: {}",
                    format_path_buf_without_prefix(&path_buf, &current_working_directory),
                    error
                );
                continue;
            }
        };

        let datetime = match extract_creation_datetime_from_media_source(
            media_asset.media_source,
            &mut media_parser,
        ) {
            Ok(datetime) => datetime,
            Err(error) => {
                statistics.failed_files += 1;
                log::warn!(
                    "Cannot extract creation datetime from {}: {}",
                    format_path_buf_without_prefix(
                        &media_asset.path_buf,
                        &current_working_directory
                    ),
                    error
                );
                continue;
            }
        };

        let datetime_formatted = time_offset
            .map(|time_offset| datetime.with_timezone(&time_offset))
            .unwrap_or(datetime)
            .format(&args.format)
            .to_string();

        rename_file(
            &media_asset.path_buf,
            &datetime_formatted,
            args.no_dry_run.not(),
            &current_working_directory,
            &mut statistics,
        );
    }

    println!("==============================");
    println!("Failed files:  {}", statistics.failed_files);
    println!("Skipped files: {}", statistics.skipped_files);
    println!("Renamed files: {}", statistics.renamed_files);

    if args.no_dry_run.not() {
        log::warn!(
            "This was just a dry run. To actually apply the renaming, use the '--no-dry-run' flag."
        )
    }

    if statistics.failed_files > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn get_media_assets_from_path_bufs(
    path_bufs: Vec<PathBuf>,
) -> impl Iterator<Item = Result<MediaAsset, (PathBuf, nom_exif::Error)>> {
    path_bufs
        .into_iter()
        .filter(|path_buf| path_buf.is_file())
        .map(|path_buf| {
            MediaSource::file_path(&path_buf)
                .map_err(|error| (path_buf.clone(), error))
                .map(|media_source| MediaAsset {
                    media_source,
                    path_buf,
                })
        })
}

fn rename_file(
    file_path_buf: &PathBuf,
    new_file_name_without_extension: &str,
    is_dry_run: bool,
    current_working_directory: &str,
    statistics: &mut Statistics,
) {
    let mut new_file_path_buf = file_path_buf.clone();
    new_file_path_buf.set_file_name(new_file_name_without_extension);
    if let Some(extension) = file_path_buf.extension() {
        new_file_path_buf.set_extension(extension.to_ascii_lowercase());
    }

    if *file_path_buf == new_file_path_buf {
        log::info!(
            "This file has already the correct name: {}",
            format_path_buf_without_prefix(&new_file_path_buf, &current_working_directory)
        );
        statistics.skipped_files += 1;
    } else if is_dry_run {
        log::info!(
            "[DRY RUN] Renaming: {} ==> {}",
            format_path_buf_without_prefix(file_path_buf, &current_working_directory),
            format_path_buf_without_prefix(&new_file_path_buf, &current_working_directory)
        );
        statistics.renamed_files += 1;
    } else {
        match std::fs::rename(&file_path_buf, &new_file_path_buf) {
            Ok(_) => {
                log::info!(
                    "Renaming: {} ==> {}",
                    format_path_buf_without_prefix(file_path_buf, &current_working_directory),
                    format_path_buf_without_prefix(&new_file_path_buf, &current_working_directory)
                );
                statistics.renamed_files += 1;
            }
            Err(error) => {
                log::warn!(
                    "Failed to rename {} to {}: {}",
                    format_path_buf_without_prefix(file_path_buf, &current_working_directory),
                    format_path_buf_without_prefix(&new_file_path_buf, &current_working_directory),
                    error
                );
                statistics.failed_files += 1;
            }
        };
    }
}

fn format_path_buf_without_prefix(path_buf: &PathBuf, prefix: &str) -> String {
    let path_string = path_buf.display().to_string();

    path_string
        .strip_prefix(prefix)
        .map(String::from)
        .unwrap_or(path_string)
}
