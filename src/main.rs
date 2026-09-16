mod checker;
mod diagnostic;
pub mod parser;
mod rules;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 {
        Path::new(&args[1])
    } else {
        Path::new(".")
    };

    let files = collect_c_files(path);
    let mut total = 0;
    let mut file_contents: Vec<(String, String)> = Vec::new();

    for file in &files {
        let diagnostics = checker::check_file(file);
        for d in &diagnostics {
            println!(
                "{}: {}:{}: {:?}:{}",
                d.file, d.line, d.code, d.severity, d.message
            );
            total += 1;
        }
        if let Ok(content) = fs::read_to_string(file) {
            file_contents.push((file.to_string_lossy().to_string(), content));
        }
    }

    let project_diagnostics = rules::check_project(&file_contents);
    for d in &project_diagnostics {
        println!(
            "{}: {}:{}: {:?}:{}",
            d.file, d.line, d.code, d.severity, d.message
        );
        total += 1;
    }

    println!("\n{} error(s) found", total);
}

fn collect_c_files(path: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
        return files;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                files.extend(collect_c_files(&p));
            } else if let Some(ext) = p.extension() {
                if ext == "c" || ext == "h" {
                    files.push(p);
                }
            }
        }
    }
    files
}
