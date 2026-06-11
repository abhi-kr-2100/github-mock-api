use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse JSON from {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// Load and parse JSON from a file using a buffered reader.
pub fn load_json_from_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, LoadError> {
    let path = path.as_ref();
    let file = std::fs::File::open(path).map_err(|source| LoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|source| LoadError::Json {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn hash(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}
