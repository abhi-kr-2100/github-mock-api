use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
enum BuildError {
    #[error("CARGO_MANIFEST_DIR not set")]
    MissingManifestDir(#[from] std::env::VarError),

    #[error("ffi crate has no parent directory")]
    NoParentDir,

    #[error("failed to load config.toml: {0}")]
    ConfigLoad(String),

    #[error("failed to generate C bindings with diplomat: {0}")]
    GenFailed(#[from] std::io::Error),
}

fn main() -> Result<(), BuildError> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let project_root = manifest_dir.parent().ok_or(BuildError::NoParentDir)?;
    let entry = manifest_dir.join("src/lib.rs");
    let out_folder = project_root.join("include/github_mock_api");
    let config_file = project_root.join("config.toml");

    let mut config = diplomat_tool::config::Config::default();
    if config_file.exists() {
        config.read_file(&config_file).map_err(BuildError::ConfigLoad)?;
    }

    diplomat_tool::r#gen(
        &entry,
        "c",
        &out_folder,
        &diplomat_tool::DocsUrlGenerator::with_base_urls(None, HashMap::new()),
        config,
        false,
    ).map_err(BuildError::GenFailed)?;

    println!("cargo:rerun-if-changed={}", entry.display());
    println!("cargo:rerun-if-changed={}", config_file.display());

    Ok(())
}
