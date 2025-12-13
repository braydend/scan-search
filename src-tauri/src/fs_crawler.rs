use std::fs;
use std::ops::Add;
use std::path::{Path, PathBuf};
use serde::Serialize;
use sha2::Digest;

#[derive(Debug, Serialize, Clone)]
pub struct FileItem {
    pub label: String,
    pub path: String,
}

impl FileItem {
    pub fn read(self: &Self) -> String {
        let path_to_file = self.path.clone();
        let path = Path::new(path_to_file.as_str());
        // will error if file is not UTF8
        let file_contents = fs::read_to_string(path);
        return match file_contents {
            Ok(file_contents) => {
                self.path.clone().add("\n\n").add(file_contents.as_str())
            },
            Err(_) => { self.path.clone() }
        }
    }
    pub fn hash(self: &Self, hasher_base: impl Digest + Clone) -> String {
        let file_contents = self.read();
        let mut hasher = hasher_base.clone();
        hasher.update(file_contents.clone());
        let hash = hasher.finalize();
        return hex::encode(hash);
    }
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