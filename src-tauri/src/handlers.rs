use tauri::State;
use crate::{timer, AppState};

#[derive(serde::Serialize)]
pub struct SearchResponse {
    data: Option<String>,
    success: bool,
    message: Option<String>,
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
                data: None,
                success: false,
                message: Some("Database is still seeding".to_string())
            };
        }
    };

    // Also avoid blocking on the model; fail fast if busy or not initialized yet
    let mut model_guard = match state.model.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            println!("Model busy: returning non-blocking error response");
            return SearchResponse {
                data: None,
                success: false,
                message: Some("Model is still loading".to_string())
            };
        }
    };
    if model_guard.is_none() {
        println!("Model not initialized yet");
        return SearchResponse {
            data: None,
            success: false,
            message: Some("Model is still loading".to_string()),
        };
    }
    let has_records = conn_guard.query_row(
        "SELECT count(id) count FROM items LIMIT 1;",
        [],
        |row|{
            let row_count: i64 = row.get(0)?;
            Ok(row_count > 0)
        })
        .expect("Failed to count items");

    if !has_records {
        return SearchResponse {
            data: None,
            success: false,
            message: Some("Database is still seeding".to_string()),
        }
    }

    println!("locks acquired");
    let result = timer::timer("Search completed", || {
        let inputs: Vec<&str> = vec![&query];
        let embeddings = model_guard
            .as_mut()
            .unwrap()
            .embed(&inputs, None)
            .expect("Failed to create embeddings");

        //         let conn = Connection::open("../sqlite/local.db").expect("Failed to open local.db");
        conn_guard.query_row(
            "SELECT vector_init('items', 'embedding', 'type=FLOAT32,dimension=768');",
            [],
            |_row| Ok(())
        ).expect("Failed to initialise vector");


        conn_guard.query_row(
            "SELECT vector_quantize('items', 'embedding');",
            [],
            |_row| Ok(())
        ).expect("Failed to quantise vector");

        let embedding_bytes: Vec<u8> = embeddings[0].iter().flat_map(|f| f.to_ne_bytes()).collect();

        let mut stmt = conn_guard.prepare(
            "SELECT e.id, v.distance, e.label, e.path FROM items AS e
                  JOIN vector_quantize_scan('items', 'embedding', ?1, 20) AS v
                  ON e.id = v.rowid
                  limit 30;"
        ).unwrap();

        let rows = stmt
            .query_map(&[&embedding_bytes],|row| {
                let id: i64 = row.get(0)?;
                let distance: f64 = row.get(1)?;
                let label: String = row.get(2)?;
                let path: String = row.get(3)?;
                Ok(ItemRow{id, distance, label, path})
            });
        rows.unwrap().map(|row| row.unwrap()).collect::<Vec<ItemRow>>()
    });

    match result {
        Ok(rows) => {
            SearchResponse {
                data: Some(serde_json::to_string(&rows).unwrap()),
                success: true,
                message: None,
            }
        },
        Err(e) => {
            println!("Search error: {}", e);
            SearchResponse {
                data: None,
                success: false,
                message: Some("Search error".to_string()),
            }
        }
    }
}