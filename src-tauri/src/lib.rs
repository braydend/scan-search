mod db;
mod timer;
mod fs_crawler;

use std::string::String;
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use rusqlite::{Connection, OptionalExtension};
use tauri::State;
use std::sync::{Arc, Mutex};
use serde::Serialize;
use std::{thread};

struct AppState {
    conn: Arc<Mutex<Connection>>,
    model: Mutex<TextEmbedding>,
}
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[derive(serde::Serialize)]
struct SearchResponse {
    data: String,
    success: bool,
}

#[derive(serde::Serialize)]
struct ItemRow {
    id: i64,
    distance: f64,
    label: String,
    path: String,
}

#[tauri::command]
fn search(state: State<AppState>, query: String) -> SearchResponse {
    println!("Starting search");
    println!("Attempting to acquire DB lock for search");
    let conn_guard = match state.conn.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            println!("DB busy: returning non-blocking error response");
            return SearchResponse {
                data: "Database is not ready".to_string(),
                success: false,
            };
        }
    };

    // Also avoid blocking on the model; fail fast if busy
    let mut model_guard = match state.model.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            println!("Model busy: returning non-blocking error response");
            return SearchResponse {
                data: "Model is not ready".to_string(),
                success: false,
            };
        }
    };
    println!("locks acquired");
    let result = timer::timer("Search completed", || {
        let inputs: Vec<&str> = vec![&query];
        let embeddings = model_guard
            .embed(&inputs, None)
            .expect("Failed to create embeddings");

        //         let conn = Connection::open("../sqlite/local.db").expect("Failed to open local.db");
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

        let embedding_bytes: Vec<u8> = embeddings[0].iter().flat_map(|f| f.to_ne_bytes()).collect();

       let result = conn_guard.query_row(
            "SELECT e.id, v.distance, e.label, e.path FROM items AS e
                  JOIN vector_quantize_scan('items', 'embedding', ?1, 20) AS v
                  ON e.id = v.rowid;",
            (embedding_bytes,),
            |row| {
                let id: i64 = row.get(0)?;
                let distance: f64 = row.get(1)?;
                let label: String = row.get(2)?;
                let path: String = row.get(3)?;
                Ok(ItemRow{id, distance, label, path})
            }
        ).optional().expect("Failed to run nearest neighbor search");

        result
    });

    match result {
        Ok(Some(item)) => SearchResponse {
            data: serde_json::to_string(&item).unwrap(),
            success: true,
        },
        Ok(None) => SearchResponse {
            data: "No results".to_string(),
            success: false,
        },
        Err(e) => {
            println!("Search error: {}", e);
            SearchResponse {
                data: "Search error".to_string(),
                success: false,
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // let mut ready = false;
    let conn = db::get_db_connection().unwrap();
    let db_mutex = Arc::new(Mutex::new(conn));
    let conn_mutex = Arc::clone(&db_mutex);
    let app_db_mutex = Arc::clone(&db_mutex);
    timer::timer("init database", || db::init_database(db_mutex));
    thread::spawn(move || {
        let mut files = Vec::new();
        timer::timer("collecting files", || {
            files.append(fs_crawler::list_src_files().unwrap().as_mut());
        }).expect("failed to collect files");

        timer::timer("adding files to db", || {
            db::seed_database(conn_mutex, files);
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
