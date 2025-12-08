#![forbid(unsafe_code)]

use crate::extract::extract_creation_datetime_from_media_source;
use crate::glob::evaluate_files_from_glob_pattern;
use crate::rename::rename_file;
use crate::statistics::Statistics;

use chrono::FixedOffset;
use nom_exif::{MediaParser, MediaSource};
use std::fs::File;
use std::ops::Not;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

mod extract;
mod glob;
mod rename;
mod statistics;

pub struct RamboOptions {
    pub pattern: String,
    pub no_dry_run: bool,
    pub case_insensitive: bool,
    pub format: String,
    pub time_offset: Option<String>,
    pub include_symlinks: bool,
}

pub fn run(options: RamboOptions) -> ExitCode {
    let mut statistics = Statistics::new();

    let current_working_directory = match std::env::current_dir() {
        Ok(working_directory) => format!("{}{}", working_directory.display(), std::path::MAIN_SEPARATOR),
        Err(error) => {
            log::error!("Cannot determine current working directory: {}", error);
            return ExitCode::FAILURE;
        }
    };

    let time_offset = match options.time_offset {
        None => None,
        Some(time_offset_string) => match FixedOffset::from_str(&time_offset_string) {
            Ok(time_offset) => Some(time_offset),
            Err(error) => {
                log::error!("Time offset '{}' is invalid: {}", time_offset_string, error);
                return ExitCode::FAILURE;
            }
        },
    };

    let Some((paths, errors)) = evaluate_files_from_glob_pattern(&options.pattern, options.case_insensitive, options.include_symlinks) else {
        return ExitCode::FAILURE;
    };

    if errors.is_empty().not() {
        statistics.failed_files += errors.len() as u64;

        log::warn!(
            "Some paths could not be read to determine if their contents match the given glob pattern '{}'. \
            Make sure you have the permissions for these paths and symlinks are not broken.",
            options.pattern
        );

        for error in errors.iter() {
            log::warn!("{}", error);
        }
    }

    if paths.is_empty() && errors.is_empty() {
        log::warn!("No media files will be processed. Make sure the glob pattern '{}' is correct.", options.pattern);

        return ExitCode::SUCCESS;
    } else if paths.is_empty() && errors.is_empty().not() {
        log::warn!(
            "No media files will be processed. Make sure the glob pattern '{}' is correct and you have adequate permissions.",
            options.pattern
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

        let datetime = match extract_creation_datetime_from_media_source(media_asset.media_source, &mut media_parser) {
            Ok(datetime) => datetime,
            Err(error) => {
                statistics.failed_files += 1;
                log::warn!(
                    "Cannot extract creation datetime from {}: {}",
                    format_path_buf_without_prefix(&media_asset.path_buf, &current_working_directory),
                    error
                );
                continue;
            }
        };

        let datetime_formatted = time_offset
            .map(|time_offset| datetime.with_timezone(&time_offset))
            .unwrap_or(datetime)
            .format(&options.format)
            .to_string();

        rename_file(
            &media_asset.path_buf,
            &datetime_formatted,
            options.no_dry_run.not(),
            &current_working_directory,
            &mut statistics,
        );
    }

    println!("==============================");
    println!("Failed files:  {}", statistics.failed_files);
    println!("Skipped files: {}", statistics.skipped_files);
    println!("Renamed files: {}", statistics.renamed_files);

    if options.no_dry_run.not() {
        log::warn!("This was just a dry run. To actually apply the renaming, use the '--no-dry-run' flag.")
    }

    if statistics.failed_files > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

struct MediaAsset {
    media_source: MediaSource<File>,
    path_buf: PathBuf,
}

/// We return the iterator which will create (and clean up!) the [MediaSource]s on-demand when it gets iterated over, i.e. in a for-loop.
/// Returning a vector for example, will create all [MediaSource]s at once, which will result in all respective files being opened and kept open at once.
/// This could cause a _Too many files open_ error.
fn get_media_assets_from_path_bufs(path_bufs: Vec<PathBuf>) -> impl Iterator<Item = Result<MediaAsset, (PathBuf, nom_exif::Error)>> {
    path_bufs.into_iter().filter(|path_buf| path_buf.is_file()).map(|path_buf| {
        MediaSource::file_path(&path_buf)
            .map_err(|error| (path_buf.clone(), error))
            .map(|media_source| MediaAsset { media_source, path_buf })
    })
}

pub(crate) fn format_path_buf_without_prefix(path_buf: &PathBuf, prefix: &str) -> String {
    let path_string = path_buf.display().to_string();

    path_string.strip_prefix(prefix).map(String::from).unwrap_or(path_string)
}
