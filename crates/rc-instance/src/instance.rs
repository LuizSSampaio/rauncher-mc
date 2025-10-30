use serde::{Deserialize, Serialize};

use crate::config::InstanceConfig;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub config: InstanceConfig,
}
