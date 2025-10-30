use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

use crate::instance::Instance;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceManager {
    instances: Vec<Instance>,
}

impl InstanceManager {
    #[instrument(skip(self), level = "info")]
    pub async fn load_instances(&mut self) -> Result<(), InstanceManagerError> {
        info!("Starting to load instances");

        let instances_dir = Self::get_instances_dir()?;
        debug!("Instances directory: {}", instances_dir.display());

        if (tokio::fs::metadata(&instances_dir).await).is_err() {
            info!(
                "Instances directory doesn't exist, creating: {}",
                instances_dir.display()
            );
            tokio::fs::create_dir_all(&instances_dir)
                .await
                .context("Failed to create instances directory")
                .map_err(|e| {
                    error!(
                        "Failed to create instances directory {}: {}",
                        instances_dir.display(),
                        e
                    );
                    InstanceManagerError::DirectoryCreationFailed {
                        path: instances_dir.clone(),
                        source: e,
                    }
                })?;
        }

        let mut entries = tokio::fs::read_dir(&instances_dir)
            .await
            .context("Failed to read instances directory")
            .map_err(|e| {
                error!(
                    "Failed to read instances directory {}: {}",
                    instances_dir.display(),
                    e
                );
                InstanceManagerError::DirectoryReadFailed {
                    path: instances_dir.clone(),
                    source: e,
                }
            })?;

        self.instances.clear();
        let mut loaded_count = 0;
        let mut failed_count = 0;

        while let Some(entry) = entries
            .next_entry()
            .await
            .context("Failed to read directory entry")
            .map_err(|e| {
                error!(
                    "Failed to read directory entry in {}: {}",
                    instances_dir.display(),
                    e
                );
                InstanceManagerError::DirectoryEntryReadFailed {
                    directory: instances_dir.clone(),
                    source: e,
                }
            })?
        {
            let path = entry.path();
            debug!(
                "Processing potential instance directory: {}",
                path.display()
            );

            if path.is_dir() {
                match self.load_instance(path.clone()).await {
                    Ok(_) => {
                        loaded_count += 1;
                        debug!("Successfully loaded instance from: {}", path.display());
                    }
                    Err(e) => {
                        failed_count += 1;
                        warn!(
                            "Failed to load instance from {}: {}. Skipping...",
                            path.display(),
                            e
                        );
                    }
                }
            } else {
                debug!("Skipping non-directory entry: {}", path.display());
            }
        }

        info!(
            "Finished loading instances: {} loaded, {} failed",
            loaded_count, failed_count
        );

        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn load_instance(&mut self, path: PathBuf) -> Result<(), InstanceManagerError> {
        let instance_file = path.join("instance.toml");
        debug!("Loading instance from: {}", instance_file.display());

        if !instance_file.exists() {
            warn!("Instance file does not exist: {}", instance_file.display());
            return Err(InstanceManagerError::InstanceFileNotFound {
                path: instance_file,
            });
        }

        let content = tokio::fs::read(&instance_file)
            .await
            .context("Failed to read instance.toml file")
            .map_err(|e| {
                error!(
                    "Failed to read instance file {}: {}",
                    instance_file.display(),
                    e
                );
                InstanceManagerError::InstanceFileReadFailed {
                    path: instance_file.clone(),
                    source: e,
                }
            })?;

        debug!(
            "Successfully read {} bytes from {}",
            content.len(),
            instance_file.display()
        );

        let instance: Instance = toml::from_slice(&content)
            .context("Failed to parse instance.toml file")
            .map_err(|e| {
                error!(
                    "Failed to parse instance file {}: {}",
                    instance_file.display(),
                    e
                );
                InstanceManagerError::InstanceParsingFailed {
                    path: instance_file.clone(),
                    source: e,
                }
            })?;

        info!(
            "Successfully loaded instance '{}' from {}",
            instance.name,
            path.display()
        );
        self.instances.push(instance);

        Ok(())
    }

    #[instrument(level = "debug")]
    fn get_instances_dir() -> Result<PathBuf, InstanceManagerError> {
        debug!("Getting instances directory");

        let proj_dirs = ProjectDirs::from("com", "rauncher", "rauncher-mc")
            .ok_or_else(|| {
                error!("Failed to determine project directories - this usually indicates an unsupported OS or missing home directory");
                InstanceManagerError::ProjectDirectoriesUnavailable
            })?;

        let instances_dir = proj_dirs.data_dir().join("instances");
        debug!(
            "Instances directory resolved to: {}",
            instances_dir.display()
        );

        Ok(instances_dir)
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    }

    #[tokio::test]
    async fn test_load_instances_empty_directory() {
        init_tracing();

        let temp_dir = tempdir().unwrap();
        let empty_dir = temp_dir.path().join("empty_instances");
        fs::create_dir_all(&empty_dir).unwrap();

        let mut manager = InstanceManager::default();

        let result = manager.load_instance(empty_dir).await;
        assert!(result.is_err());
        assert_eq!(manager.instance_count(), 0);
    }

    #[tokio::test]
    async fn test_load_instance_file_not_found() {
        init_tracing();

        let temp_dir = tempdir().unwrap();
        let instance_dir = temp_dir.path().join("test_instance");
        fs::create_dir_all(&instance_dir).unwrap();

        let mut manager = InstanceManager::default();
        let result = manager.load_instance(instance_dir.clone()).await;

        assert!(result.is_err());
        if let Err(InstanceManagerError::InstanceFileNotFound { path }) = result {
            assert_eq!(path, instance_dir.join("instance.toml"));
        } else {
            panic!("Expected InstanceFileNotFound error");
        }
    }

    #[tokio::test]
    async fn test_load_instance_invalid_toml() {
        init_tracing();

        let temp_dir = tempdir().unwrap();
        let instance_dir = temp_dir.path().join("test_instance");
        fs::create_dir_all(&instance_dir).unwrap();

        let instance_file = instance_dir.join("instance.toml");
        fs::write(&instance_file, "invalid toml content {{{").unwrap();

        let mut manager = InstanceManager::default();
        let result = manager.load_instance(instance_dir).await;

        assert!(result.is_err());
        if let Err(InstanceManagerError::InstanceParsingFailed { path, .. }) = result {
            assert_eq!(path, instance_file);
        } else {
            panic!("Expected InstanceParsingFailed error");
        }
    }
}

#[derive(Debug, Error)]
pub enum InstanceManagerError {
    #[error(
        "Project directories are unavailable - this usually indicates an unsupported OS or missing home directory"
    )]
    ProjectDirectoriesUnavailable,

    #[error("Failed to create directory '{path}': {source}")]
    DirectoryCreationFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("Failed to read directory '{path}': {source}")]
    DirectoryReadFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("Failed to read directory entry in '{directory}': {source}")]
    DirectoryEntryReadFailed {
        directory: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("Instance file not found: '{path}'")]
    InstanceFileNotFound { path: PathBuf },

    #[error("Failed to read instance file '{path}': {source}")]
    InstanceFileReadFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("Failed to parse instance file '{path}': {source}")]
    InstanceParsingFailed {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },
}
