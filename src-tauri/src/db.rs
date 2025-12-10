use std::io::Error;
use std::ops::{Add, Deref};
use std::sync::{Arc, Mutex};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rusqlite::{params, Connection, Result, LoadExtensionGuard};
use crate::fs_crawler::{read_file, FileItem};

pub fn seed_database(conn: Arc<Mutex<Connection>>, files: Vec<FileItem>) {
        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        ).unwrap();
        let filenames = &files.iter().filter_map(|item| {
            match read_file(item) {
                Ok(file_contents) => return Some(item.path.clone().add("\n\n").add(&file_contents)),
                Err(_) => {return Some(item.path.clone())}
            }
            let file_data = read_file(item);
            Option::from(item.label.clone())
        }).collect::<Vec<String>>();
        let embeddings = model.embed(filenames, None);

        let mut conn = conn.lock().expect("Failed to get lock for db");
        let tx = conn.transaction().expect("Failed to start transaction");
        {
            let mut stmt = tx
                .prepare("INSERT INTO items (embedding, label, path) VALUES (?1, ?2, ?3)")
                .expect("Failed to prepare insert statement");

            for (embedding, file) in embeddings.unwrap().iter().zip(files.iter()) {
                println!("Adding embeddings to transaction for:\npath:{}\nlabel:{}\n", file.path, file.label);
                let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_ne_bytes()).collect();
                stmt
                    .execute(params![embedding_bytes, file.label, file.path])
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