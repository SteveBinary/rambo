use crate::format_path_buf_without_prefix;
use crate::statistics::Statistics;

use std::path::PathBuf;

pub fn rename_file(
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

    let clean_file_name_old = format_path_buf_without_prefix(file_path_buf, &current_working_directory);
    let clean_file_name_new = format_path_buf_without_prefix(&new_file_path_buf, &current_working_directory);

    if *file_path_buf == new_file_path_buf {
        log::info!("This file has already the correct name: {}", clean_file_name_new);
        statistics.skipped_files += 1;
    } else if is_dry_run {
        log::info!("[DRY RUN] Renaming: {} ==> {}", clean_file_name_old, clean_file_name_new);
        statistics.renamed_files += 1;
    } else {
        match std::fs::rename(&file_path_buf, &new_file_path_buf) {
            Ok(_) => {
                log::info!("Renaming: {} ==> {}", clean_file_name_old, clean_file_name_new);
                statistics.renamed_files += 1;
            }
            Err(error) => {
                log::warn!("Failed to rename {} to {}: {}", clean_file_name_old, clean_file_name_new, error);
                statistics.failed_files += 1;
            }
        };
    }
}
