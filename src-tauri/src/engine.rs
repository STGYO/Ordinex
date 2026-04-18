use serde::{Deserialize, Serialize};
use log::{info, warn};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveOperation {
    pub original_path: String,
    pub target_path: String,
    pub action: String, // "move", "copy", "delete", "ignore"
    pub rule_id: Option<String>,
    pub rule_name: Option<String>,
    pub status: String, // "pending", "success", "failed", "duplicate_skipped", "rolled_back"
    pub error_msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExecutionSummary {
    pub total_operations: usize,
    pub moved: usize,
    pub copied: usize,
    pub deleted: usize,
    pub simulated: usize,
    pub duplicate_skipped: usize,
    pub failed: usize,
    pub elapsed_ms: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionManifest {
    pub transaction_id: String,
    pub root_folder: String,
    pub dry_run: bool,
    pub moves: Vec<MoveOperation>,
    pub summary: ExecutionSummary,
    pub timestamp: String,
}

// System sandbox to prevent critical failure
const PROTECTED_DIRS: [&str; 4] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
];

fn is_protected(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    PROTECTED_DIRS
        .iter()
        .any(|&dir| path_lower.starts_with(&dir.to_lowercase()))
}

fn move_file_with_fallback(source: &Path, target: &Path) -> io::Result<()> {
    match fs::rename(source, target) {
        Ok(_) => Ok(()),
        Err(rename_err) => {
            let code = rename_err.raw_os_error();
            let is_cross_device = matches!(code, Some(17) | Some(18));
            if !is_cross_device {
                return Err(rename_err);
            }

            fs::copy(source, target)?;
            fs::remove_file(source)?;
            Ok(())
        }
    }
}

// Computes the SHA256 hash of a file for duplicate detection
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn generate_manifest(root: &str, files: &[crate::FileNode]) -> TransactionManifest {
    let mut moves = Vec::new();
    let root_path = Path::new(root);

    for file in files {
        if !file.is_dir {
            if let Some(suggested) = &file.suggested_folder {
                let target_dir = root_path.join(suggested);
                let target_path = target_dir.join(&file.name);
                let action = file
                    .planned_action
                    .clone()
                    .unwrap_or_else(|| "move".to_string());

                moves.push(MoveOperation {
                    original_path: file.path.clone(),
                    target_path: target_path.to_string_lossy().to_string(),
                    action,
                    rule_id: file.matched_rule_id.clone(),
                    rule_name: file.matched_rule_name.clone(),
                    status: "pending".to_string(),
                    error_msg: None,
                });
            }
        }
    }

    TransactionManifest {
        transaction_id: format!("txn_{}", chrono::Utc::now().timestamp()),
        root_folder: root.to_string(),
        dry_run: false,
        moves,
        summary: ExecutionSummary::default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

pub fn execute_manifest(
    mut manifest: TransactionManifest,
    dry_run: bool,
) -> Result<TransactionManifest, String> {
    if is_protected(&manifest.root_folder) {
        return Err(
            "Target folder is located in a protected system directory and cannot be modified."
                .into(),
        );
    }

    let started = Instant::now();
    manifest.dry_run = dry_run;
    let mut summary = ExecutionSummary {
        total_operations: manifest.moves.len(),
        ..ExecutionSummary::default()
    };

    for op in manifest.moves.iter_mut() {
        let orig = Path::new(&op.original_path);
        let target = Path::new(&op.target_path);

        if dry_run {
            op.status = "simulated".to_string();
            summary.simulated += 1;
            info!(
                "dry_run_action action='{}' source='{}' target='{}' rule_id='{}'",
                op.action,
                op.original_path,
                op.target_path,
                op.rule_id.clone().unwrap_or_else(|| "none".to_string())
            );
            continue;
        }

        let action = op.action.to_ascii_lowercase();
        if action == "ignore" {
            op.status = "success".to_string();
            continue;
        }

        if !orig.exists() {
            op.status = "failed".to_string();
            op.error_msg = Some("Original file no longer exists.".to_string());
            summary.failed += 1;
            warn!(
                "operation_failed reason='source_missing' action='{}' source='{}'",
                op.action,
                op.original_path
            );
            continue;
        }

        if action == "delete" {
            match fs::remove_file(orig) {
                Ok(_) => {
                    op.status = "success".to_string();
                    summary.deleted += 1;
                    info!(
                        "operation_success action='delete' source='{}' rule_id='{}'",
                        op.original_path,
                        op.rule_id.clone().unwrap_or_else(|| "none".to_string())
                    );
                }
                Err(e) => {
                    op.status = "failed".to_string();
                    op.error_msg = Some(format!("Delete failed: {}", e));
                    summary.failed += 1;
                    warn!(
                        "operation_failed reason='delete_error' action='delete' source='{}' error='{}'",
                        op.original_path,
                        e
                    );
                }
            }
            continue;
        }

        // Create target directory if needed
        if let Some(parent) = target.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                op.status = "failed".to_string();
                op.error_msg = Some(format!("Could not create target directory: {}", e));
                summary.failed += 1;
                warn!(
                    "operation_failed reason='mkdir_failed' action='{}' target='{}' error='{}'",
                    op.action,
                    op.target_path,
                    e
                );
                continue;
            }
        }

        // Handle collision by appending a timestamp or skipping if exact match
        let mut final_target = target.to_path_buf();
        if final_target.exists() {
            // Compare file hashes to see if they are duplicates
            match (hash_file(orig), hash_file(&final_target)) {
                (Ok(h1), Ok(h2)) if h1 == h2 => {
                    // It's a duplicate - skip moving & just mark it
                    op.status = "duplicate_skipped".to_string();
                    op.error_msg =
                        Some("Exact duplicate already exists at the target location.".to_string());
                    summary.duplicate_skipped += 1;
                    continue;
                }
                _ => {
                    // Collision but different content, rename uniquely
                    let stem = target.file_stem().unwrap_or_default().to_string_lossy();
                    let ext = target.extension().unwrap_or_default().to_string_lossy();
                    let unique_name = if ext.is_empty() {
                        format!("{}_{}", stem, chrono::Utc::now().timestamp_subsec_millis())
                    } else {
                        format!(
                            "{}_{}.{}",
                            stem,
                            chrono::Utc::now().timestamp_subsec_millis(),
                            ext
                        )
                    };
                    final_target.set_file_name(unique_name);
                    op.target_path = final_target.to_string_lossy().to_string();
                }
            }
        }

        // Move or copy the file
        let result = if action == "copy" {
            fs::copy(orig, &final_target).map(|_| ())
        } else {
            move_file_with_fallback(orig, &final_target)
        };

        match result {
            Ok(_) => {
                op.status = "success".to_string();
                if action == "copy" {
                    summary.copied += 1;
                } else {
                    summary.moved += 1;
                }
                info!(
                    "operation_success action='{}' source='{}' target='{}' rule_id='{}'",
                    action,
                    op.original_path,
                    op.target_path,
                    op.rule_id.clone().unwrap_or_else(|| "none".to_string())
                );
            }
            Err(e) => {
                op.status = "failed".to_string();
                op.error_msg = Some(format!("Action failed: {}", e));
                summary.failed += 1;
                warn!(
                    "operation_failed reason='io_error' action='{}' source='{}' target='{}' error='{}'",
                    action,
                    op.original_path,
                    op.target_path,
                    e
                );
            }
        }
    }

    summary.elapsed_ms = started.elapsed().as_millis();
    manifest.summary = summary;

    Ok(manifest)
}

pub fn undo_manifest(mut manifest: TransactionManifest) -> Result<TransactionManifest, String> {
    for op in manifest.moves.iter_mut() {
        if op.status == "success" {
            let target = Path::new(&op.target_path);
            let orig = Path::new(&op.original_path);
            let action = op.action.to_ascii_lowercase();

            if action == "copy" {
                if target.exists() {
                    match fs::remove_file(target) {
                        Ok(_) => {
                            op.status = "rolled_back".to_string();
                            op.error_msg = None;
                        }
                        Err(e) => {
                            op.error_msg = Some(format!("Rollback failed: {}", e));
                        }
                    }
                }
                continue;
            }

            if action == "delete" {
                op.error_msg = Some(
                    "Rollback is not supported for delete operations without snapshot backup."
                        .to_string(),
                );
                continue;
            }

            if target.exists() {
                if let Some(parent) = orig.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                match fs::rename(target, orig) {
                    Ok(_) => {
                        op.status = "rolled_back".to_string();
                        op.error_msg = None;
                    }
                    Err(e) => {
                        op.error_msg = Some(format!("Rollback failed: {}", e));
                    }
                }
            } else {
                op.error_msg = Some("File not found at target location for rollback.".to_string());
            }
        }
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_is_protected() {
        assert!(is_protected("C:\\Windows\\System32"));
        assert!(is_protected("C:\\Program Files\\App"));
        assert!(!is_protected("C:\\Users\\Bob\\Downloads"));
        assert!(!is_protected("D:\\Games"));
    }

    #[test]
    fn test_execute_manifest_with_protected_folder() {
        let manifest = TransactionManifest {
            transaction_id: "txn_123".to_string(),
            root_folder: "C:\\Windows\\Temp".to_string(),
            dry_run: false,
            moves: vec![],
            summary: ExecutionSummary::default(),
            timestamp: "now".to_string(),
        };

        let result = execute_manifest(manifest, false);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Target folder is located in a protected system directory and cannot be modified."
        );
    }

    #[test]
    fn test_hash_duplicate_detection() {
        let dir = tempdir().unwrap();
        let file_path1 = dir.path().join("file1.txt");
        let file_path2 = dir.path().join("file2.txt");
        let file_path3 = dir.path().join("file3.txt");

        // Write same content to two files
        let mut f1 = File::create(&file_path1).unwrap();
        writeln!(f1, "Hello World").unwrap();

        let mut f2 = File::create(&file_path2).unwrap();
        writeln!(f2, "Hello World").unwrap();

        // Write different content to third file
        let mut f3 = File::create(&file_path3).unwrap();
        writeln!(f3, "Goodbye World").unwrap();

        let hash1 = hash_file(&file_path1).unwrap();
        let hash2 = hash_file(&file_path2).unwrap();
        let hash3 = hash_file(&file_path3).unwrap();

        assert_eq!(hash1, hash2, "Identical files should have the same hash");
        assert_ne!(hash1, hash3, "Different files should have different hashes");
    }

    #[test]
    fn test_manifest_undo() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("source.txt");
        let target_dir = dir.path().join("Docs");
        let target_path = target_dir.join("source.txt");

        // Create the initial file
        File::create(&file_path).unwrap();

        // Create dummy valid node for simulation
        let manifest = TransactionManifest {
            transaction_id: "txn_test".to_string(),
            root_folder: dir.path().to_string_lossy().to_string(),
            dry_run: false,
            moves: vec![MoveOperation {
                original_path: file_path.to_string_lossy().to_string(),
                target_path: target_path.to_string_lossy().to_string(),
                action: "move".to_string(),
                rule_id: None,
                rule_name: None,
                status: "pending".to_string(),
                error_msg: None,
            }],
            summary: ExecutionSummary::default(),
            timestamp: "now".to_string(),
        };

        // Execute the move
        let executed_manifest = execute_manifest(manifest, false).unwrap();

        // Assert conditions post-move
        assert!(!file_path.exists());
        assert!(target_path.exists());
        assert_eq!(executed_manifest.moves[0].status, "success");

        // Execute undo
        let undone_manifest = undo_manifest(executed_manifest).unwrap();

        // Assert conditions post-undo
        assert!(file_path.exists());
        assert!(!target_path.exists());
        assert_eq!(undone_manifest.moves[0].status, "rolled_back");
    }
}
