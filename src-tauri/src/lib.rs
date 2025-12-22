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
use crate::handlers::search;

struct AppState {
    conn: Arc<Mutex<Connection>>,
    model: Mutex<TextEmbedding>,
}
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let conn = db::get_db_connection().unwrap();
    let db_mutex = Arc::new(Mutex::new(conn));
    let conn_mutex = Arc::clone(&db_mutex);
    let app_db_mutex = Arc::clone(&db_mutex);
    timer::timer("init database", || db::init_database(db_mutex));
    thread::spawn(move || {
        let mut files = Vec::new();
        timer::timer("collecting files", || {
            files.append(fs_crawler::list_src_files("../../daily-leaderboard/packages/app".to_string()).unwrap().as_mut());
        }).expect("failed to collect files");

        timer::timer("indexing files in db", || {
            db::index_files(conn_mutex, files);
        }).expect("failed to seed database");
});
    // Allow frontend to display during indexing and give progress report?
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    ).unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { conn: app_db_mutex, model: Mutex::new(model)})
        .invoke_handler(tauri::generate_handler![search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
