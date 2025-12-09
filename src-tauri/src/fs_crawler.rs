use std::{fs, thread};
use std::path::{Path, PathBuf};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct FileItem {
    pub label: String,
    pub path: String,
}

fn collect_files_recursive(root: &Path, rel_base: &Path) -> Vec<FileItem> {
    let mut items: Vec<FileItem> = Vec::new();
    for entry in fs::read_dir(root).expect("Failed to read dir") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.is_dir() {
            println!("Collecting files from {:?}", path);
            let base = rel_base.join(path.file_name().unwrap());
            let handler = thread::spawn(move || {
                collect_files_recursive(&path, &*base)
            });
            items.append(&mut handler.join().unwrap());
        } else if path.is_file() {
            let label = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            println!("Collecting file {:?}", label);
            let relative = path.strip_prefix(rel_base).unwrap_or(&path);
            let rel_str = relative.to_string_lossy().into_owned();
            items.append(&mut Vec::from([FileItem { label, path: rel_str }]));
        }
    }
    return items
}

pub fn list_src_files() -> Result<Vec<FileItem>, String> {
    // The Rust (Tauri) binary runs with CWD at src-tauri by default during dev,
    // so the frontend source directory is one level up in "../src".
    let src_dir = PathBuf::from(".");
    if !src_dir.exists() {
        return Err(format!("src directory not found at {:?}", src_dir));
    }
    let items = collect_files_recursive(&src_dir, &src_dir);

    Ok(items)
}