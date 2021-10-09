//! Managing assets needed for running task.

use crate::Error;

pub struct AssetManager;

impl AssetManager {
    pub fn prepare(&self) -> Result<(), Error> {
        Ok(())
    }
}
