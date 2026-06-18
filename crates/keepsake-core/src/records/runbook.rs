//! Scenario runbooks: a title, a description, and an ordered list of
//! steps to follow when something happens.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single step in a runbook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunbookStep {
    /// Zero-based ordering.
    pub order: u32,
    /// Short title for the step ("Submit police report").
    pub title: String,
    /// Detailed instructions.
    pub body: String,
    /// Whether this step is done.  Tracked as free-form text so we
    /// can record "done 2026-01-24" or "blocked: waiting on ...".
    #[serde(default)]
    pub status: Option<String>,
}

/// A scenario runbook: "if X, then do Y" — used for insurance
/// claims, court dates, emergency procedures, infra incidents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioRunbook {
    /// Stable record id.
    pub id: Uuid,
    /// Short, searchable title.
    pub title: String,
    /// What scenario triggers this runbook.
    pub description: String,
    /// Ordered steps.
    pub steps: Vec<RunbookStep>,
    /// Free-form notes (links, references, contacts).
    #[serde(default)]
    pub notes: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
