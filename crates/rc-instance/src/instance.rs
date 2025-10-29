use crate::config::InstanceConfig;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instance {
    pub name: String,
    pub config: InstanceConfig,
}
