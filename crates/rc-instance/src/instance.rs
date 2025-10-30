use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::config::InstanceConfig;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub config: InstanceConfig,
}

impl Instance {
    pub async fn load(folder_name: &str) -> anyhow::Result<Self> {
        let file_path = get_instances_dir()?.join(folder_name).join("instance.toml");
        let content = tokio::fs::read(file_path).await?;

        match toml::from_slice(&content) {
            Ok(instance) => Ok(instance),
            Err(e) => Err(anyhow::anyhow!(e.to_string())),
        }
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let instances_dir = get_instances_dir()?;
        let instance_path = instances_dir.join(&self.name);

        tokio::fs::create_dir_all(&instance_path)
            .await
            .context("Failed to create instance directory")?;

        let toml = toml::to_string_pretty(&self).context("Failed to serialize instance to TOML")?;
        let file_path = instance_path.join("instance.toml");

        tokio::fs::write(&file_path, toml)
            .await
            .context("Failed to write instance.toml file")?;

        Ok(())
    }
}

fn get_instances_dir() -> anyhow::Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "rauncher", "rauncher-mc")
        .context("Failed to get project directories")?;
    Ok(proj_dirs.data_dir().join("instances"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InstanceConfig, JavaConfig, WindowConfig};

    #[tokio::test]
    async fn test_save_instance() {
        let instance = Instance {
            name: "test_instance".to_string(),
            config: InstanceConfig {
                window: Some(WindowConfig {
                    start_maximized: false,
                    width: 1920,
                    height: 1080,
                }),
                java: Some(JavaConfig {
                    path: "/usr/bin/java".to_string(),
                    min_memory: 2048,
                    max_memory: 4096,
                    arguments: "-XX:+UseG1GC".to_string(),
                }),
            },
        };

        let result = instance.save().await;
        assert!(
            result.is_ok(),
            "Failed to save instance: {:?}",
            result.err()
        );

        let instances_dir = get_instances_dir().unwrap();
        let instance_path = instances_dir.join("test_instance").join("instance.toml");
        assert!(
            instance_path.exists(),
            "instance.toml file was not created at {:?}",
            instance_path
        );

        let _ = tokio::fs::remove_dir_all(instances_dir.join("test_instance")).await;
    }

    #[tokio::test]
    async fn test_load_instance_success() {
        let instance = Instance {
            name: "test_load_instance".to_string(),
            config: InstanceConfig {
                window: Some(WindowConfig {
                    start_maximized: true,
                    width: 1280,
                    height: 720,
                }),
                java: Some(JavaConfig {
                    path: "/usr/bin/java".to_string(),
                    min_memory: 1024,
                    max_memory: 2048,
                    arguments: "-XX:+UseZGC".to_string(),
                }),
            },
        };

        let save_result = instance.save().await;
        assert!(
            save_result.is_ok(),
            "Failed to save test instance: {:?}",
            save_result.err()
        );

        let loaded_result = Instance::load("test_load_instance").await;
        assert!(
            loaded_result.is_ok(),
            "Failed to load instance: {:?}",
            loaded_result.err()
        );

        let loaded_instance = loaded_result.unwrap();
        assert_eq!(loaded_instance.name, "test_load_instance");
        assert_eq!(loaded_instance.config, instance.config);

        let instances_dir = get_instances_dir().unwrap();
        let _ = tokio::fs::remove_dir_all(instances_dir.join("test_load_instance")).await;
    }

    #[tokio::test]
    async fn test_load_instance_nonexistent() {
        let result = Instance::load("nonexistent_instance").await;
        assert!(result.is_err(), "Loading nonexistent instance should fail");
    }

    #[tokio::test]
    async fn test_load_instance_invalid_toml() {
        let instances_dir = get_instances_dir().unwrap();
        let invalid_instance_path = instances_dir.join("invalid_toml_instance");

        tokio::fs::create_dir_all(&invalid_instance_path)
            .await
            .unwrap();
        let invalid_toml_path = invalid_instance_path.join("instance.toml");

        tokio::fs::write(&invalid_toml_path, "invalid toml content [[[")
            .await
            .unwrap();

        let result = Instance::load("invalid_toml_instance").await;
        assert!(
            result.is_err(),
            "Loading instance with invalid TOML should fail"
        );

        let _ = tokio::fs::remove_dir_all(invalid_instance_path).await;
    }

    #[tokio::test]
    async fn test_load_instance_empty_config() {
        let minimal_instance = Instance {
            name: "minimal_instance".to_string(),
            config: InstanceConfig {
                window: None,
                java: None,
            },
        };

        minimal_instance.save().await.unwrap();

        let loaded_result = Instance::load("minimal_instance").await;
        assert!(
            loaded_result.is_ok(),
            "Failed to load minimal instance: {:?}",
            loaded_result.err()
        );

        let loaded_instance = loaded_result.unwrap();
        assert_eq!(loaded_instance.name, "minimal_instance");
        assert_eq!(loaded_instance.config.window, None);
        assert_eq!(loaded_instance.config.java, None);

        let instances_dir = get_instances_dir().unwrap();
        let _ = tokio::fs::remove_dir_all(instances_dir.join("minimal_instance")).await;
    }

    #[test]
    fn test_get_instances_dir() {
        let result = get_instances_dir();
        assert!(result.is_ok(), "Failed to get instances directory");

        let path = result.unwrap();
        assert!(
            path.to_string_lossy().contains("instances"),
            "Path should contain 'instances' directory"
        );
    }
}
