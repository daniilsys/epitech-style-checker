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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_a3_triggers_when_no_trailing_newline() {
        let content = "int main(void)\n{\n    return 0;\n}";
        let diags = check("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-A3"));
    }

    #[test]
    fn c_a3_does_not_trigger_with_trailing_newline() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A3"));
    }
}
