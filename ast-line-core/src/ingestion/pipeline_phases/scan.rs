use anyhow::Result;
use crate::graph::KnowledgeGraph;
use crate::ingestion::filesystem_walker::{ScannedFile, walk_repository_paths};
use crate::types::pipeline::{PipelinePhase, PipelineProgress, PipelineStats};

#[derive(Debug)]
pub struct ScanOutput {
    pub scanned_files: Vec<ScannedFile>,
    pub all_paths: Vec<String>,
    pub total_files: usize,
}

pub struct ScanPhase;

impl ScanPhase {
    pub fn execute(
        repo_path: &std::path::Path,
        graph: &KnowledgeGraph,
        on_progress: &mut dyn FnMut(PipelineProgress),
    ) -> Result<ScanOutput> {
        on_progress(PipelineProgress {
            phase: PipelinePhase::Extracting,
            percent: 0,
            message: "Scanning repository...".into(),
            detail: None,
            stats: None,
        });

        let node_count = graph.node_count();

        let scanned_files = walk_repository_paths(repo_path, |current, total, file_path| {
            let scan_progress = ((current as f64 / total.max(1) as f64) * 15.0).round() as u8;
            on_progress(PipelineProgress {
                phase: PipelinePhase::Extracting,
                percent: scan_progress,
                message: "Scanning repository...".into(),
                detail: Some(file_path.to_owned()),
                stats: Some(PipelineStats {
                    files_processed: current,
                    total_files: total,
                    nodes_created: node_count,
                    relationships_created: 0,
                }),
            });
        })?;

        let total_files = scanned_files.len();
        let all_paths = scanned_files.iter().map(|f| f.path.clone()).collect();

        on_progress(PipelineProgress {
            phase: PipelinePhase::Extracting,
            percent: 15,
            message: "Repository scanned successfully".into(),
            detail: None,
            stats: Some(PipelineStats {
                files_processed: total_files,
                total_files,
                nodes_created: node_count,
                relationships_created: 0,
            }),
        });

        Ok(ScanOutput { scanned_files, all_paths, total_files })
    }
}
