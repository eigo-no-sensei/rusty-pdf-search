#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use walkdir::WalkDir;
use tauri::{Manager, State};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookInfo {
    pub path: String,
    pub file_type: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub size_bytes: u64,
    pub content_preview: String, // First 2000 chars of extracted text
}

#[derive(Default)]
pub struct Library {
    pub books: Mutex<Vec<BookInfo>>,
}

fn extract_epub_content(path: &Path) -> String {
    let mut content = String::new();
    if let Ok(mut doc) = epub::doc::EpubDoc::new(path) {
        // Extract text from first few chapters to avoid memory bloat
        for (_, id) in doc.toc.iter().take(5) {
            if let Some(raw_html) = doc.get_chapter_raw(id) {
                if let Ok(text) = html2text::from_read(&raw_html[..], 80) {
                    content.push_str(&text);
                    content.push(' ');
                    if content.len() > 2000 {
                        content.truncate(2000);
                        break;
                    }
                }
            }
        }
    }
    content.trim().to_string()
}

fn extract_pdf_content(path: &Path) -> String {
    match pdf_extract::extract_text(path, &pdf_extract::OutputFormat::Text) {
        Ok(text) => text.chars().take(2000).collect(),
        Err(_) => String::new(), // Gracefully skip unreadable PDFs
    }
}

#[tauri::command]
fn index_directory(dir_path: String, state: State<Library>) -> Result<usize, String> {
    let mut books = Vec::new();
    let path = Path::new(&dir_path);

    if !path.exists() || !path.is_dir() {
        return Err("Invalid directory path".to_string());
    }

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if !entry_path.is_file() { continue; }

        let ext = entry_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());

        if let Some(ext) = ext {
            if ext == "epub" || ext == "pdf" {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let mut title = None;
                let mut author = None;
                let mut content = String::new();

                if ext == "epub" {
                    if let Ok(doc) = epub::doc::EpubDoc::new(entry_path) {
                        title = doc.metadata.get("title").and_then(|v| v.first()).cloned();
                        author = doc.metadata.get("creator").and_then(|v| v.first()).cloned();
                    }
                    content = extract_epub_content(entry_path);
                } else if ext == "pdf" {
                    if let Ok(doc) = lopdf::Document::load(entry_path) {
                        if let Ok(info_ref) = doc.trailer.get(b"Info") {
                            if let Ok(info_obj) = doc.get_object(info_ref.as_reference().unwrap()) {
                                if let Ok(dict) = info_obj.as_dict() {
                                    if let Ok(t) = dict.get(b"Title") {
                                        title = extract_pdf_string(t);
                                    }
                                    if let Ok(a) = dict.get(b"Author") {
                                        author = extract_pdf_string(a);
                                    }
                                }
                            }
                        }
                    }
                    content = extract_pdf_content(entry_path);
                }

                books.push(BookInfo {
                    path: entry_path.to_string_lossy().to_string(),
                    file_type: ext,
                    title,
                    author,
                    size_bytes: size,
                    content_preview: content,
                });
            }
        }
    }

    *state.books.lock().unwrap() = books;
    Ok(state.books.lock().unwrap().len())
}

fn extract_pdf_string(obj: &lopdf::Object) -> Option<String> {
    if let lopdf::Object::String(bytes, _) = obj {
        Some(String::from_utf8_lossy(bytes).to_string())
    } else {
        None
    }
}

#[tauri::command]
fn search_books(query: String, state: State<Library>) -> Vec<BookInfo> {
    let q = query.to_lowercase();
    state.books.lock().unwrap()
        .iter()
        .filter(|book| {
            book.title.as_ref().map(|t| t.to_lowercase().contains(&q)).unwrap_or(false) ||
            book.author.as_ref().map(|a| a.to_lowercase().contains(&q)).unwrap_or(false) ||
            book.content_preview.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Library::default())
        .invoke_handler(tauri::generate_handler![index_directory, search_books])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}