use crate::engine::TransactionManifest;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tauri::Manager;

pub fn init_db(app_handle: &tauri::AppHandle) -> Result<()> {
    let mut db_path = match app_handle.path().app_data_dir() {
        Ok(dir) => dir,
        Err(_) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    std::fs::create_dir_all(&db_path).unwrap_or(());
    db_path.push("history.db");

    let conn = Connection::open(&db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS manifests (
            id TEXT PRIMARY KEY,
            root_folder TEXT NOT NULL,
            json_blob TEXT NOT NULL,
            timestamp TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

pub fn save_manifest(app_handle: &tauri::AppHandle, manifest: &TransactionManifest) -> Result<()> {
    let mut db_path = match app_handle.path().app_data_dir() {
        Ok(dir) => dir,
        Err(_) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    db_path.push("history.db");

    let conn = Connection::open(&db_path)?;

    let json_blob = serde_json::to_string(manifest).unwrap();
    conn.execute(
                "INSERT INTO manifests (id, root_folder, json_blob, timestamp)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                     root_folder = excluded.root_folder,
                     json_blob = excluded.json_blob,
                     timestamp = excluded.timestamp",
        [
            &manifest.transaction_id,
            &manifest.root_folder,
            &json_blob,
            &manifest.timestamp,
        ],
    )?;

    Ok(())
}

pub fn fetch_history(app_handle: &tauri::AppHandle) -> Result<Vec<TransactionManifest>> {
    let mut db_path = match app_handle.path().app_data_dir() {
        Ok(dir) => dir,
        Err(_) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    db_path.push("history.db");

    let conn = Connection::open(&db_path)?;
    let mut stmt =
        conn.prepare("SELECT json_blob FROM manifests ORDER BY timestamp DESC LIMIT 50")?;
    let mut rows = stmt.query([])?;

    let mut history = Vec::new();
    while let Some(row) = rows.next()? {
        let json_blob: String = row.get(0)?;
        if let Ok(manifest) = serde_json::from_str(&json_blob) {
            history.push(manifest);
        }
    }

    Ok(history)
}
