use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub window: Option<WindowConfig>,
    pub java: Option<JavaConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WindowConfig {
    pub start_maximized: bool,
    pub width: u64,
    pub height: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JavaConfig {
    pub path: String,
    pub min_memory: u64,
    pub max_memory: u64,
    pub arguments: String,
}
