//! Pipeline state management

use std::time::Instant;

/// Pipeline state machine
///
/// Represents the current state of a pipeline. State transitions are validated
/// to ensure consistent behavior across all stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineState {
    /// Pipeline is idle and not processing
    Idle,

    /// Pipeline is initializing (transitioning to Running)
    Initializing,

    /// Pipeline is actively processing media
    Running {
        /// When the pipeline started running
        started_at: Instant,
    },

    /// Pipeline is paused (can resume to Running)
    Paused {
        /// When the pipeline was paused
        paused_at: Instant,
    },

    /// Pipeline is stopping (transitioning to Stopped)
    Stopping,

    /// Pipeline has stopped and cannot be restarted
    Stopped,
}

impl PipelineState {
    /// Check if this state transition is valid
    pub fn can_transition_to(&self, target: &PipelineState) -> bool {
        use PipelineState::*;

        match (self, target) {
            // From Idle
            (Idle, Initializing) => true,

            // From Initializing
            (Initializing, Running { .. }) => true,
            (Initializing, Stopping) => true, // Can abort initialization

            // From Running
            (Running { .. }, Paused { .. }) => true,
            (Running { .. }, Stopping) => true,

            // From Paused
            (Paused { .. }, Running { .. }) => true,
            (Paused { .. }, Stopping) => true,

            // From Stopping
            (Stopping, Stopped) => true,

            // From Stopped - no transitions allowed
            (Stopped, _) => false,

            // Self-transitions
            (a, b) if a == b => true,

            // All other transitions invalid
            _ => false,
        }
    }

    /// Get a human-readable description of this state
    pub fn description(&self) -> &'static str {
        match self {
            PipelineState::Idle => "Idle",
            PipelineState::Initializing => "Initializing",
            PipelineState::Running { .. } => "Running",
            PipelineState::Paused { .. } => "Paused",
            PipelineState::Stopping => "Stopping",
            PipelineState::Stopped => "Stopped",
        }
    }

    /// Check if the pipeline is currently active (running or paused)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            PipelineState::Running { .. } | PipelineState::Paused { .. }
        )
    }

    /// Check if the pipeline is running
    pub fn is_running(&self) -> bool {
        matches!(self, PipelineState::Running { .. })
    }

    /// Check if the pipeline is paused
    pub fn is_paused(&self) -> bool {
        matches!(self, PipelineState::Paused { .. })
    }

    /// Check if the pipeline is stopped or stopping
    pub fn is_stopped(&self) -> bool {
        matches!(self, PipelineState::Stopped | PipelineState::Stopping)
    }

    /// Get the duration since the pipeline started (if running)
    pub fn running_duration(&self) -> Option<std::time::Duration> {
        if let PipelineState::Running { started_at } = self {
            Some(started_at.elapsed())
        } else {
            None
        }
    }
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let idle = PipelineState::Idle;
        let initializing = PipelineState::Initializing;
        let running = PipelineState::Running {
            started_at: Instant::now(),
        };
        let paused = PipelineState::Paused {
            paused_at: Instant::now(),
        };
        let stopping = PipelineState::Stopping;
        let stopped = PipelineState::Stopped;

        // Valid transitions
        assert!(idle.can_transition_to(&initializing));
        assert!(initializing.can_transition_to(&running));
        assert!(running.can_transition_to(&paused));
        assert!(paused.can_transition_to(&running));
        assert!(running.can_transition_to(&stopping));
        assert!(paused.can_transition_to(&stopping));
        assert!(stopping.can_transition_to(&stopped));

        // Self-transitions
        assert!(idle.can_transition_to(&idle));
        assert!(running.can_transition_to(&running));
    }

    #[test]
    fn test_invalid_transitions() {
        let idle = PipelineState::Idle;
        let running = PipelineState::Running {
            started_at: Instant::now(),
        };
        let stopped = PipelineState::Stopped;

        // Invalid transitions
        assert!(!idle.can_transition_to(&running)); // Must go through Initializing
        assert!(!idle.can_transition_to(&stopped)); // Can't stop from idle
        assert!(!stopped.can_transition_to(&running)); // Can't restart after stopped
        assert!(!stopped.can_transition_to(&idle)); // Can't reset to idle
    }

    #[test]
    fn test_state_checks() {
        let running = PipelineState::Running {
            started_at: Instant::now(),
        };
        let paused = PipelineState::Paused {
            paused_at: Instant::now(),
        };
        let stopped = PipelineState::Stopped;

        assert!(running.is_active());
        assert!(running.is_running());
        assert!(!running.is_paused());
        assert!(!running.is_stopped());

        assert!(paused.is_active());
        assert!(!paused.is_running());
        assert!(paused.is_paused());
        assert!(!paused.is_stopped());

        assert!(!stopped.is_active());
        assert!(!stopped.is_running());
        assert!(!stopped.is_paused());
        assert!(stopped.is_stopped());
    }
}
