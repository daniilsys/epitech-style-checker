use crate::diagnostic::{Diagnostic, Severity};

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_goto(filename, content));

    diagnostics
}

fn check_goto(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if line.contains("goto") {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Major,
                code: "C-C3".to_string(),
                message: "goto keyword is forbidden".to_string(),
            });
        }
    }
    diagnostics
}
