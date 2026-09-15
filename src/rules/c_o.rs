use crate::diagnostic::{Diagnostic, Severity};
use std::path::Path;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_snake_case(filename, content));

    diagnostics
}

fn check_snake_case(filename: &str, _content: &str) -> Vec<Diagnostic> {
    let name = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if !name
        .chars()
        .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return vec![Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Minor,
            code: "C-O4".to_string(),
            message: "Filename must be in snake_case".to_string(),
        }];
    }
    vec![]
}
