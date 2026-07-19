#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState {
    Starting,
    Running,
    ShuttingDown,
    Stopped,
}

impl DaemonState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::ShuttingDown => "shutting_down",
            Self::Stopped => "stopped",
        }
    }
}

impl std::fmt::Display for DaemonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starting_state() {
        let state = DaemonState::Starting;
        assert_eq!(state.as_str(), "starting");
    }

    #[test]
    fn test_running_state() {
        let state = DaemonState::Running;
        assert_eq!(state.as_str(), "running");
    }

    #[test]
    fn test_shutting_down_state() {
        let state = DaemonState::ShuttingDown;
        assert_eq!(state.as_str(), "shutting_down");
    }

    #[test]
    fn test_stopped_state() {
        let state = DaemonState::Stopped;
        assert_eq!(state.as_str(), "stopped");
    }

    #[test]
    fn test_display_format() {
        assert_eq!(format!("{}", DaemonState::Running), "running");
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(DaemonState::Starting, DaemonState::Starting);
        assert_ne!(DaemonState::Starting, DaemonState::Running);
    }
}
