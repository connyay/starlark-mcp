use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

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

    /// Monitors extensions directory for file changes and triggers on_change callback.
    /// Uses OS-native file watching (inotify/FSEvents/kqueue) in a separate thread.
    pub fn start_watching<F>(&self, engine: Arc<StarlarkEngine>, on_change: F) -> Result<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let extensions_dir = self.extensions_dir.clone();
        let dir_path = PathBuf::from(&extensions_dir);

        if !dir_path.exists() {
            warn!("Extensions directory does not exist, skipping file watching");
            return Ok(());
        }

        info!("Starting file watcher for: {}", extensions_dir);

        let (tx, mut rx) = mpsc::channel::<Event>(100);

        // Required to call async tx.send() from the blocking notify thread
        let rt = tokio::runtime::Handle::current();

        // notify crate requires synchronous blocking thread, not tokio runtime
        std::thread::spawn(move || {
            let mut watcher =
                notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                    Ok(event) => {
                        let _ = rt.block_on(tx.send(event));
                    }
                    Err(e) => error!("Watch error: {:?}", e),
                })
                .expect("Failed to create file watcher");

            watcher
                .watch(&dir_path, RecursiveMode::NonRecursive)
                .expect("Failed to watch extensions directory");

            // Park the thread indefinitely - watcher stays alive
            std::thread::park();
        });

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = Self::handle_file_event(event, &engine, &on_change).await {
                    error!("Error handling file event: {}", e);
                }
            }
        });

        Ok(())
    }

    async fn handle_file_event<F>(
        event: Event,
        engine: &StarlarkEngine,
        on_change: &F,
    ) -> Result<()>
    where
        F: Fn(),
    {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) == Some("star") {
                        let file_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");

                        info!("Extension file changed: {}", path.display());

                        match fs::read_to_string(&path).await {
                            Ok(content) => match engine.load_extension(file_name, &content).await {
                                Ok(_) => {
                                    info!("Successfully reloaded extension: {}", file_name);
                                    on_change();
                                }
                                Err(e) => {
                                    warn!("Failed to reload extension {}: {}", file_name, e)
                                }
                            },
                            Err(e) => {
                                warn!("Failed to read extension file {}: {}", path.display(), e)
                            }
                        }
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) == Some("star") {
                        let file_name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");

                        info!("Extension file removed: {}", path.display());

                        if let Some(_) = engine.remove_extension(file_name).await {
                            info!("Successfully removed extension: {}", file_name);
                            on_change();
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
