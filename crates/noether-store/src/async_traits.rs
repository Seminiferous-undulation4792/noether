//! Async stage-store abstraction.
//!
//! Suitable for database-backed stores (PostgreSQL, etc.) that can serve many
//! concurrent Axum handlers without blocking the executor.
//!
//! All methods take `&self` — implementations are expected to hold their own
//! internal synchronisation (a connection pool, a `tokio::sync::Mutex`, …).
//! All return types are owned so they are safe to `.await` across task
//! boundaries without lifetime issues.

use crate::traits::{StoreError, StoreStats};
use noether_core::stage::{Stage, StageId, StageLifecycle};

/// Async storage abstraction for stages.
#[async_trait::async_trait]
pub trait AsyncStageStore: Send + Sync {
    /// Insert a stage. Returns `AlreadyExists` if a stage with the same ID is
    /// already present.
    async fn put(&self, stage: Stage) -> Result<StageId, StoreError>;

    /// Insert or replace a stage unconditionally.
    async fn upsert(&self, stage: Stage) -> Result<StageId, StoreError>;

    /// Remove a stage entirely. Succeeds even if the stage does not exist.
    async fn remove(&self, id: &StageId) -> Result<(), StoreError>;

    /// Retrieve a stage by ID, returning `None` if absent.
    async fn get(&self, id: &StageId) -> Result<Option<Stage>, StoreError>;

    /// Return true if a stage with the given ID exists.
    async fn contains(&self, id: &StageId) -> Result<bool, StoreError>;

    /// List all stages, optionally filtered by lifecycle variant.
    async fn list(&self, lifecycle: Option<StageLifecycle>) -> Result<Vec<Stage>, StoreError>;

    /// Transition the lifecycle of a stage, enforcing valid transitions.
    async fn update_lifecycle(
        &self,
        id: &StageId,
        lifecycle: StageLifecycle,
    ) -> Result<(), StoreError>;

    /// Aggregate statistics across the store.
    async fn stats(&self) -> Result<StoreStats, StoreError>;
}
