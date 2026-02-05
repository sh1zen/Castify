//! Pipeline stage trait and coordinator
//!
//! Defines the interface for pipeline stages and provides a coordinator
//! for managing stage lifecycle.

use anyhow::Result;
use async_trait::async_trait;

use super::health::PipelineHealth;
use super::state::PipelineState;

/// Trait for pipeline stages that process media data
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Run the stage, processing data until shutdown signal
    async fn run(&mut self) -> Result<()>;

    /// Get the name of this stage for logging
    fn name(&self) -> &'static str;

    /// Gracefully shutdown the stage
    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Pipeline coordinator that manages stage lifecycle
pub struct PipelineCoordinator {
    stages: Vec<Box<dyn PipelineStage>>,
    state: std::sync::Arc<tokio::sync::RwLock<PipelineState>>,
    health: std::sync::Arc<PipelineHealth>,
}

impl PipelineCoordinator {
    /// Create a new pipeline coordinator
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            state: std::sync::Arc::new(tokio::sync::RwLock::new(PipelineState::Idle)),
            health: std::sync::Arc::new(PipelineHealth::new()),
        }
    }

    /// Add a stage to the pipeline
    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    /// Get current pipeline state
    pub async fn state(&self) -> PipelineState {
        *self.state.read().await
    }

    /// Get health metrics
    pub fn health(&self) -> std::sync::Arc<PipelineHealth> {
        self.health.clone()
    }

    /// Start all stages
    pub async fn start(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = PipelineState::Running {
            started_at: std::time::Instant::now(),
        };
        Ok(())
    }

    /// Stop all stages
    pub async fn stop(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = PipelineState::Stopping;

        // Shutdown all stages
        for stage in &mut self.stages {
            stage.shutdown().await?;
        }

        *state = PipelineState::Stopped;
        Ok(())
    }
}

impl Default for PipelineCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
