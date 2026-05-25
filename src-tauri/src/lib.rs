pub mod ast;
pub mod editor_state;
pub mod typst_world;

use crate::ast::{
    BlockSyncRequest, BlockSyncResponse, BlockMoveRequest, BlockMoveResponse,
    InitialDocumentResponse, FormatRequest, FormatResponse, HistoryRequest, HistoryResponse,
    VersionHistoryResponse, VersionEntry, RestoreVersionRequest,
};
use crate::editor_state::EditorState;
use crate::typst_world::compile_typst;
use std::sync::Mutex;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn sync_block(
    req: BlockSyncRequest,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<Option<BlockSyncResponse>, String> {
    let mut editor = state
        .lock()
        .map_err(|_| "Failed to lock editor state".to_string())?;
    Ok(editor.process_sync_request(req))
}

#[tauri::command]
async fn open_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<Option<InitialDocumentResponse>, String> {
    use tauri_plugin_dialog::DialogExt;
    
    let file_path = match app.dialog().file().blocking_pick_file() {
        Some(path) => path.to_string(),
        None => return Ok(None),
    };
    
    let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    
    let (blocks, node_order) = crate::ast::parse_document_to_blocks(&content);
    
    let mut editor = state.lock().unwrap();
    editor.blocks.clear();
    for b in &blocks {
        editor.blocks.insert(b.id.clone(), crate::editor_state::BlockState {
            id: b.id.clone(),
            parent_id: None,
            previous_sibling_id: None,
            markdown: crate::editor_state::EditorState::ast_to_markdown(&b.ast_content),
            block_type: b.block_type.clone(),
        });
    }
    editor.node_order = node_order.clone();
    editor.current_file_path = Some(file_path.clone());
    editor.file_path_hash = Some(crate::editor_state::EditorState::hash_path(&file_path));
    editor.is_dirty = false;
    
    Ok(Some(InitialDocumentResponse {
        blocks,
        node_order,
        file_path: Some(file_path),
    }))
}

#[tauri::command]
fn save_file(
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<(), String> {
    let (path, markdown) = {
        let mut editor = state.lock().unwrap();
        if !editor.is_dirty {
            return Ok(());
        }
        let path = editor.current_file_path.clone().ok_or("No file opened")?;
        let mut md = String::new();
        for id in &editor.node_order {
            if let Some(block) = editor.blocks.get(id) {
                md.push_str(&block.markdown);
                md.push_str("\n\n");
            }
        }
        editor.is_dirty = false;
        (path, md)
    };

    // Write main file
    fs::write(&path, &markdown).map_err(|e| e.to_string())?;

    // Write portable history entry alongside the file
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let filename = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document");
        let history_dir = parent.join(".fastmm_history");
        if fs::create_dir_all(&history_dir).is_ok() {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let hist_path = history_dir.join(format!("{}.{}.md", filename, ts));
            let _ = fs::write(hist_path, &markdown);
        }
    }

    Ok(())
}

#[tauri::command]
fn move_block(
    req: BlockMoveRequest,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<Option<BlockMoveResponse>, String> {
    let mut editor = state.lock().unwrap();
    Ok(editor.move_block(req))
}

#[tauri::command]
fn apply_format(
    req: FormatRequest,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<Option<FormatResponse>, String> {
    let mut editor = state.lock().unwrap();
    Ok(editor.apply_format(req))
}

#[tauri::command]
fn trigger_history(
    req: HistoryRequest,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<Option<HistoryResponse>, String> {
    let mut editor = state.lock().unwrap();
    Ok(editor.trigger_history(req))
}

// ─── Auto-save (silent path: app_data_dir) ───────────────────────────────────

#[tauri::command]
async fn auto_save_silent(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<(), String> {
    // Clone needed data and drop the lock before doing any async FS work
    let (markdown, hash) = {
        let editor = state.lock().unwrap();
        if !editor.is_dirty {
            return Ok(());
        }
        let hash = match &editor.file_path_hash {
            Some(h) => h.clone(),
            None => return Ok(()), // No file open — skip silently
        };
        let mut md = String::new();
        for id in &editor.node_order {
            if let Some(block) = editor.blocks.get(id) {
                md.push_str(&block.markdown);
                md.push_str("\n\n");
            }
        }
        (md, hash)
    };

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let history_dir = data_dir.join("FastMM").join("history").join(&hash);
    fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?;

    // FIFO: remove oldest entries when over the 25-file limit
    let _ = prune_auto_history(&history_dir, 25);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let target = history_dir.join(format!("{}.md", ts));
    fs::write(target, markdown).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete oldest entries until at most `keep` remain.
fn prune_auto_history(dir: &std::path::Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    if files.len() <= keep { return; }
    // Sort ascending by filename (timestamp prefix → oldest first)
    files.sort_by_key(|e| e.file_name());
    for entry in &files[..files.len() - keep] {
        let _ = fs::remove_file(entry.path());
    }
}

// ─── Version history ──────────────────────────────────────────────────────────

#[tauri::command]
async fn get_version_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<VersionHistoryResponse, String> {
    let (hash, file_path) = {
        let editor = state.lock().unwrap();
        (editor.file_path_hash.clone(), editor.current_file_path.clone())
    };

    let mut entries: Vec<VersionEntry> = Vec::new();

    // ① Auto-save entries from app_data_dir
    if let Some(ref h) = hash {
        if let Ok(data_dir) = app.path().app_data_dir() {
            let auto_dir = data_dir.join("FastMM").join("history").join(h);
            collect_history_entries(&auto_dir, "auto", &mut entries);
        }
    }

    // ② Manual-save entries from .fastmm_history/ beside the file
    if let Some(ref fp) = file_path {
        if let Some(parent) = std::path::Path::new(fp).parent() {
            let manual_dir = parent.join(".fastmm_history");
            collect_history_entries(&manual_dir, "manual", &mut entries);
        }
    }

    // Sort newest first
    entries.sort_by(|a, b| b.version_id.cmp(&a.version_id));

    Ok(VersionHistoryResponse { entries })
}

fn collect_history_entries(dir: &std::path::Path, source: &str, out: &mut Vec<VersionEntry>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") { continue; }
        let version_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if version_id.is_empty() { continue; }
        // Best-effort: parse timestamp millis into a human label
        let label = if let Ok(ms) = version_id.parse::<u64>() {
            let secs = ms / 1000;
            // Approximate UTC datetime from unix timestamp (no external deps)
            format_unix_secs(secs)
        } else {
            version_id.clone()
        };
        out.push(VersionEntry { version_id, source: source.to_string(), label });
    }
}

/// Very small UTC formatter: "YYYY-MM-DD HH:MM:SS"
fn format_unix_secs(secs: u64) -> String {
    // Days since epoch
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;

    // Compute Gregorian date (Fliegel & Van Flandern algorithm)
    let jd = days as i64 + 2440588; // Julian Day Number for 1970-01-01
    let l = jd + 68569;
    let n = (4 * l) / 146097;
    let l = l - (146097 * n + 3) / 4;
    let i = (4000 * (l + 1)) / 1461001;
    let l = l - (1461 * i) / 4 + 31;
    let j = (80 * l) / 2447;
    let day = l - (2447 * j) / 80;
    let l = j / 11;
    let month = j + 2 - 12 * l;
    let year = 100 * (n - 49) + i + l;

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, h, m, s)
}

// ─── Restore version ──────────────────────────────────────────────────────────

#[tauri::command]
async fn restore_version(
    app: tauri::AppHandle,
    req: RestoreVersionRequest,
    state: tauri::State<'_, Mutex<EditorState>>,
) -> Result<HistoryResponse, String> {
    // ① Snapshot for undo (before restoring)
    let file_path = {
        let mut editor = state.lock().unwrap();
        let first_id = editor.node_order.first().cloned().unwrap_or_default();
        editor.push_undo_snapshot(&first_id, 0);
        editor.current_file_path.clone()
    };

    // ② Resolve file path
    let version_file = resolve_version_path(&app, &state, &req, &file_path)
        .map_err(|e| e.to_string())?;

    // ③ Read and parse — outside the lock
    let content = fs::read_to_string(&version_file).map_err(|e| e.to_string())?;
    let (new_blocks, new_order) = crate::ast::parse_document_to_blocks(&content);

    // ④ Apply to state
    let seq = req.seq;
    let mut editor = state.lock().unwrap();
    editor.blocks.clear();
    for b in &new_blocks {
        editor.blocks.insert(b.id.clone(), crate::editor_state::BlockState {
            id: b.id.clone(),
            parent_id: None,
            previous_sibling_id: None,
            markdown: b.plain_text.clone(),
            block_type: b.block_type.clone(),
        });
    }
    editor.node_order = new_order.clone();
    editor.is_dirty = true;

    let restored_blocks: Vec<crate::ast::RestoredBlockData> = new_blocks.iter().map(|b| {
        crate::ast::RestoredBlockData {
            id: b.id.clone(),
            block_type: b.block_type.clone(),
            ast_content: b.ast_content.clone(),
            markdown: b.plain_text.clone(),
        }
    }).collect();

    let first_id = new_order.first().cloned().unwrap_or_default();
    Ok(HistoryResponse {
        seq,
        restored_blocks,
        node_order: new_order,
        caret: crate::ast::CaretPosition { target_node_id: first_id, offset: 0 },
    })
}

fn resolve_version_path(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, Mutex<EditorState>>,
    req: &RestoreVersionRequest,
    file_path: &Option<String>,
) -> Result<std::path::PathBuf, String> {
    if req.source == "auto" {
        let hash = state.lock().unwrap().file_path_hash.clone()
            .ok_or("No file hash available")?;
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        Ok(data_dir.join("FastMM").join("history").join(&hash).join(format!("{}.md", req.version_id)))
    } else {
        let fp = file_path.as_deref().ok_or("No file opened")?;
        let parent = std::path::Path::new(fp).parent()
            .ok_or("Cannot resolve parent directory")?;
        let filename = std::path::Path::new(fp)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document");
        Ok(parent.join(".fastmm_history").join(format!("{}.{}.md", filename, req.version_id)))
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let editor_state = Mutex::new(EditorState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(editor_state)
        .invoke_handler(tauri::generate_handler![
            greet, sync_block, open_file, save_file, move_block, apply_format, trigger_history,
            compile_typst, auto_save_silent, get_version_history, restore_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
