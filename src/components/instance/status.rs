#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    #[default]
    Ready,
    Running,
    Installing,
    Error,
}

impl InstanceStatus {
    pub fn label(&self) -> &str {
        match self {
            InstanceStatus::Ready => "Ready",
            InstanceStatus::Running => "Running",
            InstanceStatus::Installing => "Installing",
            InstanceStatus::Error => "Error",
        }
    }

    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            InstanceStatus::Ready => (34, 197, 94),      // Green
            InstanceStatus::Running => (59, 130, 246),   // Blue
            InstanceStatus::Installing => (234, 179, 8), // Yellow
            InstanceStatus::Error => (239, 68, 68),      // Red
        }
    }
}
