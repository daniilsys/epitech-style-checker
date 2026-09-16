use crate::diagnostic::{Diagnostic, Severity};

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_null_character(filename, content));

    diagnostics
}

fn check_null_character(filename: &str, content: &str) -> Vec<Diagnostic> {
    if let Some(pos) = content.find('\0') {
        let line = content[..pos].matches('\n').count() + 1;
        return vec![Diagnostic {
            file: filename.to_string(),
            line,
            severity: Severity::Fatal,
            code: "C-Z1".to_string(),
            message: "File contains a null character".to_string(),
        }];
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_z1_triggers_on_null_character() {
        let content = "int main(void)\n{\0\n    return 0;\n}\n";
        let diags = check("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-Z1"));
    }

    #[test]
    fn c_z1_does_not_trigger_without_null_character() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-Z1"));
    }
}
