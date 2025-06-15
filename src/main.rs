#![forbid(unsafe_code)]

use crate::extract::extract_creation_datetime_from_media_source;
use crate::statistics::Statistics;
use chrono::FixedOffset;
use clap::Parser;
use glob::{GlobError, MatchOptions};
use log::LevelFilter;
use nom_exif::{MediaParser, MediaSource};
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

mod cli;
mod extract;
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
            Make sure you have the permissions for these paths:",
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
                log::warn!("Cannot process {}: {}", path_buf.display(), error);
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
                    media_asset.path_buf.display(),
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

fn evaluate_files_from_glob_pattern(
    pattern: &str,
    case_insensitive: bool,
) -> Option<(Vec<PathBuf>, Vec<GlobError>)> {
    let match_options = MatchOptions {
        case_sensitive: case_insensitive.not(),
        ..Default::default()
    };

    let glob_results = match glob::glob_with(pattern, match_options) {
        Ok(paths) => paths,
        Err(error) => {
            log::error!("Failed to interpret glob pattern: {}", error);
            return None;
        }
    };

    let (mut paths, mut errors) = glob_results.fold(
        (Vec::<PathBuf>::new(), Vec::<GlobError>::new()),
        |(mut paths, mut errors), glob_result| {
            match glob_result {
                Ok(path) => paths.push(
                    path.canonicalize()
                        .expect(&format!("Failed to canonicalize path: {}", path.display())),
                ),
                Err(error) => errors.push(error),
            };
            (paths, errors)
        },
    );

    let lowercase_os_str_from_path_buf =
        |path_buf: &PathBuf| path_buf.as_os_str().to_ascii_lowercase();

    let lowercase_os_str_from_glob_error =
        |path_buf: &GlobError| lowercase_os_str_from_path_buf(&path_buf.path().to_path_buf());

    paths.sort_by_key(lowercase_os_str_from_path_buf);
    errors.sort_by_key(lowercase_os_str_from_glob_error);

    Some((paths, errors))
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
    path_buf: &PathBuf,
    new_file_name_without_extension: &str,
    is_dry_run: bool,
    statistics: &mut Statistics,
) {
    let mut new_file_path_buf = path_buf.clone();
    new_file_path_buf.set_file_name(new_file_name_without_extension);
    if let Some(extension) = path_buf.extension() {
        new_file_path_buf.set_extension(extension.to_ascii_lowercase());
    }

    if *path_buf == new_file_path_buf {
        log::info!(
            "This file has already the correct name: {}",
            new_file_path_buf.display()
        );
        statistics.skipped_files += 1;
    } else if is_dry_run {
        log::info!(
            "[DRY RUN] Renaming {} ==> {}",
            path_buf.display(),
            new_file_path_buf.display()
        );
        statistics.renamed_files += 1;
    } else {
        match std::fs::rename(&path_buf, &new_file_path_buf) {
            Ok(_) => {
                log::info!(
                    "Renaming {} ==> {}",
                    path_buf.display(),
                    new_file_path_buf.display()
                );
                statistics.renamed_files += 1;
            }
            Err(error) => {
                log::warn!(
                    "Failed to rename {} to {}: {}",
                    path_buf.display(),
                    new_file_path_buf.display(),
                    error
                );
                statistics.failed_files += 1;
            }
        };
    }
}
