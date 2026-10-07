use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| e.to_string())
}

// ─── Fingerprinting (external-change detection) ───

#[derive(Serialize)]
struct FileFp {
    mtime: i64,
    hash: String,
}

#[derive(Serialize)]
struct FileContent {
    content: String,
    mtime: i64,
    hash: String,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn file_mtime_ms(p: &Path) -> i64 {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325u64;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn fingerprint(path: &Path) -> Result<FileFp, String> {
    let data = fs::read(path).map_err(|e| e.to_string())?;
    Ok(FileFp {
        mtime: file_mtime_ms(path),
        hash: fnv1a64(&data),
    })
}

#[tauri::command]
fn file_fingerprint(path: String) -> Result<FileFp, String> {
    fingerprint(Path::new(&path))
}

#[tauri::command]
fn read_file_fp(path: String) -> Result<FileContent, String> {
    let p = Path::new(&path);
    let content = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let mtime = file_mtime_ms(p);
    let hash = fnv1a64(content.as_bytes());
    Ok(FileContent {
        content,
        mtime,
        hash,
    })
}

// ─── Atomic writes (tmp + fsync + rename) ───

fn atomic_write(path: &Path, data: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".tmp{}", std::process::id()));
    let tmp = PathBuf::from(tmp);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Atomically writes the file and returns its new fingerprint.
#[tauri::command]
fn write_file_atomic(path: String, content: String) -> Result<FileFp, String> {
    let p = PathBuf::from(&path);
    atomic_write(&p, &content)?;
    Ok(FileFp {
        mtime: file_mtime_ms(&p),
        hash: fnv1a64(content.as_bytes()),
    })
}

// ─── Drafts (unsaved edits; originals untouched until explicit Save) ───

#[derive(Serialize, Deserialize)]
struct Draft {
    path: String,
    content: String,
    orig_mtime: i64,
    orig_hash: String,
    updated_at: i64,
}

#[derive(Serialize)]
struct DraftMeta {
    path: String,
    orig_mtime: i64,
    orig_hash: String,
    updated_at: i64,
}

fn drafts_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let d = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("drafts");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn draft_file(app: &tauri::AppHandle, file: &str) -> Result<PathBuf, String> {
    Ok(drafts_dir(app)?.join(format!("{}.draft", fnv1a64(file.as_bytes()))))
}

#[tauri::command]
fn save_draft(
    app: tauri::AppHandle,
    path: String,
    content: String,
    mtime: i64,
    hash: String,
) -> Result<(), String> {
    let fp = draft_file(&app, &path)?;
    let d = Draft {
        path,
        content,
        orig_mtime: mtime,
        orig_hash: hash,
        updated_at: now_ms(),
    };
    let json = serde_json::to_string(&d).map_err(|e| e.to_string())?;
    atomic_write(&fp, &json)
}

#[tauri::command]
fn load_draft(app: tauri::AppHandle, path: String) -> Result<Option<Draft>, String> {
    let fp = draft_file(&app, &path)?;
    if !fp.exists() {
        return Ok(None);
    }
    let raw = match fs::read_to_string(&fp) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    match serde_json::from_str::<Draft>(&raw) {
        Ok(d) => Ok(Some(d)),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
fn delete_draft(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let fp = draft_file(&app, &path)?;
    if fp.exists() {
        let _ = fs::remove_file(&fp);
    }
    Ok(())
}

#[tauri::command]
fn list_drafts(app: tauri::AppHandle) -> Result<Vec<DraftMeta>, String> {
    let dir = drafts_dir(&app)?;
    let mut out: Vec<DraftMeta> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("draft") {
                continue;
            }
            if let Ok(raw) = fs::read_to_string(&p) {
                if let Ok(d) = serde_json::from_str::<Draft>(&raw) {
                    out.push(DraftMeta {
                        path: d.path,
                        orig_mtime: d.orig_mtime,
                        orig_hash: d.orig_hash,
                        updated_at: d.updated_at,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

#[tauri::command]
fn app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn minimize_window(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) {
    let _ = window.close();
}

// ─── Close handshake: flush drafts/session before the window goes away ───

static FLUSH_DONE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn finish_close(window: tauri::Window) {
    FLUSH_DONE.store(true, Ordering::SeqCst);
    let _ = window.close();
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if !FLUSH_DONE.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.emit("flush-before-close", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            read_file,
            write_file,
            read_file_fp,
            file_fingerprint,
            write_file_atomic,
            save_draft,
            load_draft,
            delete_draft,
            list_drafts,
            app_data_dir,
            minimize_window,
            toggle_maximize_window,
            close_window,
            finish_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
