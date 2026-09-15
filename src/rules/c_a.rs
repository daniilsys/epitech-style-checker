use crate::diagnostic::{Diagnostic, Severity};

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_file_end(filename, content));

    diagnostics
}

fn check_file_end(filename: &str, content: &str) -> Vec<Diagnostic> {
    if !content.ends_with('\n') {
        let line_count = content.lines().count().max(1);
        return vec![Diagnostic {
            file: filename.to_string(),
            line: line_count,
            severity: Severity::Info,
            code: "C-A3".to_string(),
            message: "File must end with a line break".to_string(),
        }];
    }
    vec![]
}
