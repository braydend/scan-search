use rusqlite::OptionalExtension;
use tauri::State;
use crate::{timer, AppState};

#[derive(serde::Serialize)]
pub struct SearchResponse {
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
pub fn search(state: State<AppState>, query: String) -> SearchResponse {
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
    let has_records = conn_guard.query_row(
        "SELECT count(id) count FROM items LIMIT 1;",
        [],
        |row| Ok(row.into().count())
    )
        .optional()
        .unwrap()
        .expect("Failed to count items");

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