use crate::diagnostic::Diagnostic;
use crate::rules;
use std::path::Path;

pub fn check_file(path: &Path) -> Vec<Diagnostic> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let filename = path.to_string_lossy().to_string();
    let mut diagnostics = Vec::new();

    diagnostics.extend(rules::check(&filename, &content));
    diagnostics
}
