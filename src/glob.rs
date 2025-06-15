use glob::{GlobError, MatchOptions};
use std::ops::Not;
use std::path::PathBuf;

pub fn evaluate_files_from_glob_pattern(
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
        |glob_error: &GlobError| lowercase_os_str_from_path_buf(&glob_error.path().to_path_buf());

    paths.sort_by_key(lowercase_os_str_from_path_buf);
    errors.sort_by_key(lowercase_os_str_from_glob_error);

    Some((paths, errors))
}
