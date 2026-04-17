use serde::{Deserialize, Serialize};

/// Phase name used in progress updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelinePhase {
    Extracting,
    Structure,
    Markdown,
    Cobol,
    Parse,
    Routes,
    Tools,
    Orm,
    CrossFile,
    Mro,
    Communities,
    Processes,
    Persisting,
    Done,
}

/// Progress update emitted by the ingestion pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    pub phase: PipelinePhase,
    /// 0–100 overall completion percentage.
    pub percent: u8,
    pub message: String,
    pub detail: Option<String>,
    pub stats: Option<PipelineStats>,
}

/// Runtime statistics included in pipeline progress updates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    pub files_processed: usize,
    pub total_files: usize,
    pub nodes_created: usize,
    pub relationships_created: usize,
}
