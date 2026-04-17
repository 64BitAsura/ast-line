use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineProgress {
    pub phase: PipelinePhase,
    pub percent: u8,
    pub message: String,
    pub detail: Option<String>,
    pub stats: Option<PipelineStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    pub files_processed: usize,
    pub total_files: usize,
    pub nodes_created: usize,
    pub relationships_created: usize,
}
