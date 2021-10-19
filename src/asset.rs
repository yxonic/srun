use std::{collections::HashMap, fs::File, io::Write, path::Path};

use cached_path::cached_path;
use data_url::DataUrl;
use tempfile::TempDir;
use tokio::task::spawn_blocking;

use crate::Error;

/// Managing assets needed for running task.
pub struct AssetManager {
    tempdir: TempDir,
}

impl AssetManager {
    pub fn new() -> Result<Self, Error> {
        Ok(AssetManager {
            tempdir: TempDir::new()?,
        })
    }

    /// Parse or fetch all assets into temp directory
    pub async fn prepare(&self, assets: HashMap<String, String>) -> Result<(), Error> {
        for (k, v) in assets {
            let file_path = self.tempdir.path().join(&k);
            std::fs::create_dir_all(&file_path.parent().expect("should have parent"))?;
            if v.starts_with("data:") {
                let url = DataUrl::process(&v).map_err(|e| Error::SpecError(format!("{:?}", e)))?;
                let (body, _) = url
                    .decode_to_vec()
                    .map_err(|e| Error::SpecError(format!("{:?}", e)))?;
                log::debug!("writing to: {:?}", file_path);
                let mut file = File::create(file_path)?;
                file.write_all(&body)?;
                file.flush()?;
            } else if v.starts_with("http") {
                log::debug!("downloading: {}", k);
                // do blocking reqwest in a new thread
                let cached_path = spawn_blocking(move || cached_path(&v))
                    .await
                    .map_err(|e| Error::UnknownError(format!("{:?}", e)))??;
                log::trace!("copying to: {:?}", file_path);
                std::fs::copy(cached_path, file_path)?;
            } else {
                return Err(Error::SpecError(
                    "asset must start with either data or http".into(),
                ));
            }
        }
        Ok(())
    }

    /// Get cache path
    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }
}
