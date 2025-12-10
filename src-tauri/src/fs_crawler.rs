use std::fs;
use std::path::{Path, PathBuf};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct FileItem {
    pub label: String,
    pub path: String,
}

pub fn read_file(file: &FileItem) -> Result<String, std::io::Error> {
    let path_to_file = file.path.clone();
    let path = Path::new(path_to_file.as_str());
    // will error if file is not UTF8
    fs::read_to_string(path)
}

fn collect_files_recursive(root: &Path, current_path: &Path) -> Vec<FileItem> {
    let mut items: Vec<FileItem> = Vec::new();
    for entry in fs::read_dir(current_path).expect("Failed to read dir") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path().clone();
        if entry.path().is_dir() {
            let mut nested = collect_files_recursive(root, path.as_path());
            items.append(&mut nested);
        } else if path.is_file() {
            let label = path.file_name().unwrap().to_string_lossy().to_string();
            let path_string = path.as_path().to_string_lossy().to_string();

            items.push(FileItem { label, path: path_string });
        }
    }
    return items
}

pub fn list_src_files(path: String) -> Result<Vec<FileItem>, String> {
    // The Rust (Tauri) binary runs with CWD at src-tauri by default during dev,
    // so the frontend source directory is one level up in "../src".
    let src_dir = PathBuf::from(path);
    if !src_dir.exists() {
        return Err(format!("src directory not found at {:?}", src_dir));
    }
    let items = collect_files_recursive(&src_dir, &src_dir);

    Ok(items)
}