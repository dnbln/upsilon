use std::path::PathBuf;

pub struct FileHost {
    pub(crate) path: Option<PathBuf>,
    pub(crate) contents: String,
}

impl FileHost {
    pub fn new(path: Option<PathBuf>, contents: String) -> Self {
        Self { path, contents }
    }
}