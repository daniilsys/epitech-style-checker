use crate::diagnostic::{Diagnostic, Severity};

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    check_line_length(filename, content)
}

fn check_line_length(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if line.len() > 80 {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Major,
                code: "C-F3".to_string(),
                message: format!("Line exceeds 80 columns ({})", line.len()),
            });
        }
    }
    diagnostics
}
