use std::sync::{Arc, Mutex};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rusqlite::{params, params_from_iter, Connection, Result, LoadExtensionGuard};
use sha2::{Digest, Sha256};
use crate::fs_crawler::FileItem;

// Helper for DB lookups
struct LookupResult {
    path: String,
}

fn get_files_to_update(conn: Arc<Mutex<Connection>>, files: Vec<FileItem>) -> Vec<FileItem> {
    if files.is_empty() {
        return Vec::new();
    }

    let hasher_base = Sha256::new();
    // Pre-compute (path, hash) for inputs
    let pairs: Vec<(String, String)> = files
        .iter()
        .map(|f| (f.path.clone(), f.hash(hasher_base.clone())))
        .collect();

    // Build VALUES list: ( ?, ? ), ( ?, ? ), ...
    let values = std::iter::repeat("(?, ?)")
        .take(pairs.len())
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "WITH input(path, hash) AS (VALUES {}) \
         SELECT i.path \
         FROM input AS i \
         LEFT JOIN items AS f ON f.path = i.path \
         WHERE f.path IS NULL OR f.hash IS NOT i.hash;",
        values
    );

    let conn = conn.lock().expect("Failed to get lock for db");
    let mut stmt = conn.prepare(&sql).expect("Failed to prepare statement");

    // Flatten params: [path1, hash1, path2, hash2, ...]
    let flat_iter = pairs.iter().flat_map(|(p, h)| [p.as_str(), h.as_str()]);

    let rows = stmt
        .query_map(params_from_iter(flat_iter), |row| {
            let path: String = row.get(0)?;
            Ok(LookupResult { path })
        })
        .expect("Failed to query mismatched files");

    let mismatched_paths: std::collections::HashSet<String> =
        rows.filter_map(|r| r.ok().map(|lr| lr.path)).collect();

    files
        .into_iter()
        .filter(|f| mismatched_paths.contains(&f.path))
        .collect()
}

pub fn index_files(conn: Arc<Mutex<Connection>>, files: Vec<FileItem>) {
    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    ).unwrap();
    let files_to_update = get_files_to_update(conn.clone(), files);
    if (files_to_update.is_empty()) {
        println!("No files to update");
        return;
    }
    let file_data = &files_to_update.iter().filter_map(|item| Some(item.read())).collect::<Vec<String>>();
    let embeddings = model.embed(file_data, None);

    let mut conn = conn.lock().expect("Failed to get lock for db");
    let tx = conn.transaction().expect("Failed to start transaction");
    {
        let mut stmt = tx
            .prepare("INSERT INTO items (embedding, label, path, hash) VALUES (?1, ?2, ?3, ?4)
                      ON CONFLICT(path) DO UPDATE SET
                        embedding=excluded.embedding,
                        label=excluded.label,
                        hash=excluded.hash")
            .expect("Failed to prepare insert statement");

        for (embedding, file) in embeddings.unwrap().iter().zip(files_to_update.iter()) {
            println!("Adding embeddings to transaction for:\npath:{}\nlabel:{}\n", file.path, file.label);
            let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_ne_bytes()).collect();
            let file_hash = file.hash(Sha256::new());
            stmt
                .execute(params![embedding_bytes, file.label, file.path, file_hash])
                .expect("failed to insert item in db");
        }
    }
    println!("Commiting transaction");
        tx.commit().expect("Failed to commit transaction");
    }

    #[derive(Debug)]
    struct SQLiteVectorExtensionConfig {
        extension_path: &'static str,
        entry_point: Option<&'static str>,
    }

    fn get_sqlite_vector_extension_for_current_os() -> Option<SQLiteVectorExtensionConfig> {
        match std::env::consts::OS {
            "macos" => Some(SQLiteVectorExtensionConfig {
                extension_path: "../sqlite/extensions/macos-arm-vector",
                entry_point: Some("sqlite3_vector_init"),
            }),
            "linux" => Some(SQLiteVectorExtensionConfig {
                extension_path: "../sqlite/extensions/linux-x86-vector",
                entry_point: Some("sqlite3_vector_init"),
            }),
            _ => None,
        }
    }

    pub fn get_db_connection() -> Result<Connection> {
        let conn = Connection::open("../sqlite/local.db").expect("Failed to open local.db");
        // Use in memory DB with:
        // let conn = Connection::open_in_memory().expect("Failed to open local.db");

        {
            let _guard = unsafe { LoadExtensionGuard::new(&conn)? };
            let vector_extension_config = get_sqlite_vector_extension_for_current_os().expect("Failed to get sqlite vector extension configuration");

            unsafe {
                conn
                    .load_extension(
                        vector_extension_config.extension_path,
                        vector_extension_config.entry_point,
                    )
                    .expect("Failed to load vector extension");
            }
        }

        Ok(conn)
    }

    pub fn init_database(conn_mutex: Arc<Mutex<Connection>>) {
        let conn = conn_mutex.lock().expect("Failed to get lock for db");

        // Uncomment to recreate the database
        // conn.execute(
        //     "DROP TABLE IF  EXISTS items;",
        //     [],
        // ).expect("Failed to remove existing table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS items (
             id INTEGER PRIMARY KEY,
             embedding BLOB,
             label TEXT,
             path TEXT UNIQUE,
             hash TEXT
         );",
            [],
        ).expect("Failed to create table");
    }