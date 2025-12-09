mod db;

use std::time::SystemTime;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use rusqlite::{params, Connection, Result, LoadExtensionGuard};
use tauri::State;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use std::{fs, thread};
use std::fs::read;
use std::path::{Path, PathBuf};

struct AppState {
    conn: Arc<Mutex<Connection>>,
    model: Mutex<TextEmbedding>,
    ready: bool
}
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[derive(Debug, Serialize, Clone)]
struct FileItem {
    label: String,
    path: String,
}

fn collect_files_recursive(root: &Path, rel_base: &Path, out: &mut Vec<FileItem>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            println!("Collecting files from {:?}", path);
            collect_files_recursive(&path, rel_base, out)?;
        } else if path.is_file() {
            let label = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let relative = path.strip_prefix(rel_base).unwrap_or(&path);
            let rel_str = relative.to_string_lossy().into_owned();
            out.push(FileItem { label, path: rel_str });
        }
    }
    Ok(())
}

fn list_src_files() -> std::result::Result<Vec<FileItem>, String> {
    // The Rust (Tauri) binary runs with CWD at src-tauri by default during dev,
    // so the frontend source directory is one level up in "../src".
    let src_dir = PathBuf::from(".");
    if !src_dir.exists() {
        return Err(format!("src directory not found at {:?}", src_dir));
    }
    let mut items = Vec::new();
    collect_files_recursive(&src_dir, &src_dir, &mut items)
        .map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
fn is_ready(state: State<AppState>) -> bool {
    let ready = state.conn.try_lock().is_ok();
    println!("is_ready called {:?}" , ready);
    ready
}

#[tauri::command]
fn search(state: State<AppState>, query: String) -> String {
    let result = timer("Search completed", || {
        let inputs: Vec<&str> = vec![&query];
        let embeddings = state.model.lock().expect("Failed to get lock for model").embed(&inputs, None);

        //         let conn = Connection::open("../sqlite/local.db").expect("Failed to open local.db");
        let conn_guard = state.conn.lock().expect("failed to lock db");
        conn_guard.query_row(
            "SELECT vector_init('items', 'embedding', 'type=FLOAT32,dimension=384');",
            [],
            |_row| Ok(())
        ).expect("Failed to initialise vector");


        conn_guard.query_row(
            "SELECT vector_quantize('items', 'embedding');",
            [],
            |_row| Ok(())
        ).expect("Failed to quantise vector");

        let embedding_bytes: Vec<u8> = embeddings.unwrap()[0].iter().flat_map(|f| f.to_ne_bytes()).collect();

       conn_guard.query_row(
            "SELECT e.id, v.distance, e.label, e.path FROM items AS e
                  JOIN vector_quantize_scan('items', 'embedding', ?1, 20) AS v
                  ON e.id = v.rowid;",
            (embedding_bytes,),
            |row| {
                let id: i64 = row.get(0)?;
                let distance: f64 = row.get(1)?;
                let label: String = row.get(2)?;
                let path: String = row.get(3)?;
                Ok((id, distance, label, path))
            }
        ).expect("Failed to run nearest neighbor search");
    });


          format!("{:?}", result)
    //     format!("{}", found.map_or("Not found".to_string(), |_| format!("Found: {:?}!", found)))
}

fn timer<T>(label: &str, func: impl FnOnce()->T) -> Result<T>{
    let start = SystemTime::now();
    let result = func();
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    println!("{} complete in ({:?})", label, duration);
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut ready = false;
    let conn = db::get_db_connection().unwrap();
    let db_mutex = Arc::new(Mutex::new(conn));
    let conn_mutex = Arc::clone(&db_mutex);
    let app_db_mutex = Arc::clone(&db_mutex);
    timer("init database", || db::init_database(db_mutex));
    // TODO: split out db seeding and do it async
        thread::spawn(move || {
            timer("db seeding", || {
                let files = list_src_files().unwrap();
                db::seed_database(conn_mutex, files);
                ready = true;
            }).unwrap();
        });
    // Allow frontend to display during indexing and give progress report?
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    ).unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { conn: app_db_mutex, model: Mutex::new(model), ready })
        .invoke_handler(tauri::generate_handler![search, is_ready])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
