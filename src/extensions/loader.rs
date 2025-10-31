use anyhow::Result;
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};

use crate::starlark::StarlarkEngine;

pub struct ExtensionLoader {
    extensions_dir: String,
}

impl ExtensionLoader {
    pub fn new(extensions_dir: String) -> Self {
        Self { extensions_dir }
    }

    pub async fn load_all(&self, engine: &StarlarkEngine) -> Result<()> {
        let dir_path = Path::new(&self.extensions_dir);

        if !dir_path.exists() {
            warn!(
                "Extensions directory does not exist: {}",
                self.extensions_dir
            );
            return Ok(());
        }

        info!("Loading extensions from: {}", self.extensions_dir);

        let mut entries = fs::read_dir(dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("star") {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                info!("Loading extension file: {}", path.display());

                match self.load_extension_file(engine, &path, file_name).await {
                    Ok(_) => info!("Successfully loaded extension: {}", file_name),
                    Err(e) => warn!("Failed to load extension {}: {}", file_name, e),
                }
            }
        }

        Ok(())
    }

    async fn load_extension_file(
        &self,
        engine: &StarlarkEngine,
        path: &Path,
        name: &str,
    ) -> Result<()> {
        let content = fs::read_to_string(path).await?;
        engine.load_extension(name, &content).await?;
        Ok(())
    }
}
