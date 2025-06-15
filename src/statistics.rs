#[derive(Debug, Default)]
pub struct Statistics {
    pub skipped_files: u64,
    pub failed_files: u64,
    pub renamed_files: u64,
}

impl Statistics {
    pub fn new() -> Self {
        Self::default()
    }
}
