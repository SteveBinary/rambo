use glob::{GlobError, MatchOptions};
use std::ffi::OsString;
use std::fmt::Display;
use std::ops::Not;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum GlobEvaluationError {
    GlobError(GlobError),
    Other {
        path_buf: PathBuf,
        description: String,
    },
}

impl GlobEvaluationError {
    pub fn path(&self) -> &Path {
        match self {
            GlobEvaluationError::GlobError(glob_error) => glob_error.path(),
            GlobEvaluationError::Other { path_buf, .. } => path_buf.as_path(),
        }
    }
}

impl Display for GlobEvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobEvaluationError::GlobError(glob_error) => {
                write!(f, "Failed to evaluate glob: {}", glob_error)
            }
            GlobEvaluationError::Other { description, .. } => {
                write!(f, "Failed to evaluate glob: {}", description)
            }
        }
    }
}

pub fn evaluate_files_from_glob_pattern(
    pattern: &str,
    case_insensitive: bool,
    include_symlinks: bool,
) -> Option<(Vec<PathBuf>, Vec<GlobEvaluationError>)> {
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
        (Vec::<PathBuf>::new(), Vec::<GlobEvaluationError>::new()),
        |(mut paths, mut errors), glob_result| {
            match glob_result {
                Ok(path) => {
                    if include_symlinks || path.is_symlink().not() {
                        match path.canonicalize() {
                            Ok(path) => paths.push(path),
                            Err(error) => {
                                let error_description = format!(
                                    "Failed to canonicalize path '{}': {}",
                                    path.display(),
                                    error
                                );
                                errors.push(GlobEvaluationError::Other {
                                    path_buf: path,
                                    description: error_description,
                                });
                            }
                        };
                    }
                }
                Err(error) => errors.push(GlobEvaluationError::GlobError(error)),
            };

            (paths, errors)
        },
    );

    paths.sort_by_key(lowercase_os_str_from_path_buf);
    errors.sort_by_key(lowercase_os_str_from_glob_evaluation_error);

    Some((paths, errors))
}

fn lowercase_os_str_from_path_buf(path_buf: &PathBuf) -> OsString {
    path_buf.as_os_str().to_ascii_lowercase()
}
fn lowercase_os_str_from_glob_evaluation_error(glob_error: &GlobEvaluationError) -> OsString {
    glob_error.path().as_os_str().to_ascii_lowercase()
}
