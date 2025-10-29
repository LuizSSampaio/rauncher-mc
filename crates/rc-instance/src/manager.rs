use crate::instance::Instance;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InstanceManager {
    instances: Vec<Instance>,
}
