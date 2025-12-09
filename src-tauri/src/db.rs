use std::sync::{Arc, Mutex};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rusqlite::{params, Connection, Result, LoadExtensionGuard};
use crate::FileItem;

pub fn seed_database(conn: Arc<Mutex<Connection>>, files: Vec<FileItem>) {
        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        ).unwrap();
        let filenames = &files.iter().filter_map(|item| item.path.parse().ok()).collect::<Vec<String>>();
        let embeddings = model.embed(filenames, None);

        let conn = conn.lock().expect("Failed to get lock for db");
        for (embedding, file) in embeddings.unwrap().iter().zip(files.iter()) {
            let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_ne_bytes()).collect();
            conn.execute(
                "INSERT INTO items (embedding, label, path) VALUES (?1, ?2, ?3)",
                params![embedding_bytes, file.label, file.path],
            ).expect("failed to insert item in db");
        }
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
                entry_point: None,
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

        conn.execute(
            "DROP TABLE IF  EXISTS items;",
            [],
        ).expect("Failed to remove existing table");

        conn.execute(
            "CREATE TABLE IF NOT EXISTS items (
             id INTEGER PRIMARY KEY,
             embedding BLOB,
             label TEXT,
             path TEXT
         );",
            [],
        ).expect("Failed to create table");
    }