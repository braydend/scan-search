mod db;
mod timer;
mod fs_crawler;
mod handlers;

use std::string::String;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use rusqlite::{Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use serde::Serialize;
use std::{thread};
use std::time::Duration;
use crate::handlers::search;

struct AppState {
    conn: Arc<Mutex<Connection>>,
    // Model is initialized asynchronously; None while loading or on failure
    model: Arc<Mutex<Option<TextEmbedding>>>,
}
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::get_db_connection().unwrap();
    let db_mutex = Arc::new(Mutex::new(conn));
    let conn_mutex = Arc::clone(&db_mutex);
    let app_db_mutex = Arc::clone(&db_mutex);
    timer::timer("init database", || db::init_database(db_mutex));
//     thread::spawn(move || {
//         let mut files = Vec::new();
//         timer::timer("collecting files", || {
//             files.append(fs_crawler::list_src_files("../../ascension/src".to_string()).unwrap().as_mut());
//         }).expect("failed to collect files");
//
//         timer::timer("adding files to db", || {
//             db::seed_database(conn_mutex, files);
//         }).expect("failed to seed database");
// });
    // Allow frontend to display during indexing and give progress report?
    /*
    Use model JinaEmbeddingsV2BaseCode for code embeddings
    Use model AllMiniLML6V2 for text embeddings
     */
    // Initialize embedding model asynchronously with retries to avoid panics on cache lock issues
    let model_arc: Arc<Mutex<Option<TextEmbedding>>> = Arc::new(Mutex::new(None));
    let model_init_target = Arc::clone(&model_arc);
    thread::spawn(move || {
        let max_attempts = 5;
        for attempt in 1..=max_attempts {
            match TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::JinaEmbeddingsV2BaseCode)
                    .with_show_download_progress(true),
            ) {
                Ok(model) => {
                    if let Ok(mut guard) = model_init_target.lock() {
                        *guard = Some(model);
                    }
                    println!("Embedding model initialized");
                    let mut files = Vec::new();
                    timer::timer("collecting files", || {
                        files.append(fs_crawler::list_src_files("../../ascension/src/E1/AppBundle/Controller".to_string()).unwrap().as_mut());
                    }).expect("failed to collect files");

                    timer::timer("adding files to db", || {
                        db::seed_database(conn_mutex, files);
                    }).expect("failed to seed database");

                    return;
                }
                Err(e) => {
                    eprintln!("Model init failed (attempt {}/{}): {}", attempt, max_attempts, e);
                    // If it's a lock acquisition issue, wait and retry; otherwise still retry a few times
                    if attempt < max_attempts {
                        // Exponential backoff
                        let sleep_secs = 3 * attempt as u64;
                        thread::sleep(Duration::from_secs(sleep_secs));
                        continue;
                    } else {
                        eprintln!("Giving up on model initialization. You can delete the stale lock in .fastembed_cache if present and restart.");
                    }
                }
            }
        }
    });
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { conn: app_db_mutex, model: model_arc })
        .invoke_handler(tauri::generate_handler![search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
