use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use rusqlite::{params, Connection, Result, LoadExtensionGuard};
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

fn initDatabase() -> Result<Connection> {
//         let values: Vec<&str> = vec!["bray", "is", "learning", "tauri"];
let conn = Connection::open("../sqlite/local.db").expect("Failed to open local.db");

{
    let _guard = unsafe { LoadExtensionGuard::new(&conn)? };
    unsafe { conn.load_extension("../sqlite/plugins/vector", None::<&str>).expect("Failed to load vector extension"); }
}

    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
             id INTEGER PRIMARY KEY,
             embedding BLOB,
             label TEXT
         );",
        [],
    ).expect("Failed to create table");

            let values: Vec<&str> = vec!["bray", "is", "learning", "tauri"];

                let mut model = TextEmbedding::try_new(
                    InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
                ).unwrap();
                let embeddings = model.embed(&values, None);

         for (embedding, label) in embeddings.unwrap().iter().zip(values.iter()) {
             let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_ne_bytes()).collect();
             conn.execute(
                 "INSERT INTO items (embedding, label) VALUES (?1, ?2)",
                 params![embedding_bytes, label],
             )?;
         }
Ok(conn)
}

#[tauri::command]
fn search(query: &str) -> String {
    let conn = initDatabase().expect("Failed to initialize database");
//     let values: Vec<&str> = vec!["bray", "is", "learning", "tauri"];
//     let found = values.iter().find(|value| value.contains(&query));
//     let mut model = TextEmbedding::try_new(Default::default()).unwrap();
    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    ).unwrap();
    let embeddings = model.embed(vec![query], None);

//         let conn = Connection::open("../sqlite/local.db").expect("Failed to open local.db");
    conn.query_row(
                    "SELECT vector_init('items', 'embedding', 'type=FLOAT32,dimension=384');",
                    [],
                        |_row| Ok(())
                ).expect("Failed to initialise vector");


        conn.query_row(
                        "SELECT vector_quantize('items', 'embedding');",
                        [],
                            |_row| Ok(())
                    ).expect("Failed to quantise vector");

let embedding_bytes: Vec<u8> = embeddings.unwrap()[0].iter().flat_map(|f| f.to_ne_bytes()).collect();

          let result = conn.query_row(
              "SELECT e.id, v.distance, e.label FROM items AS e
                  JOIN vector_quantize_scan('items', 'embedding', ?1, 20) AS v
                  ON e.id = v.rowid;",
              (embedding_bytes,),
              |row| {
                  let id: i64 = row.get(0)?;
                  let distance: f64 = row.get(1)?;
                  let label: String = row.get(2)?;
                  Ok((id, distance, label))
              }
          ).expect("Failed to run nearest neighbor search");

    format!("{:?}", result)
//     format!("{}", found.map_or("Not found".to_string(), |_| format!("Found: {:?}!", found)))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![search])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
