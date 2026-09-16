use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(check_include_guard(filename, content));
    diagnostics.extend(check_forbidden_directives(filename, content));
    diagnostics.extend(check_functions_placement(filename, content));
    diagnostics.extend(check_macro_size(filename, content));
    diagnostics
}

fn check_include_guard(filename: &str, content: &str) -> Vec<Diagnostic> {
    if !filename.ends_with(".h") {
        return vec![];
    }

    let has_ifndef = content
        .lines()
        .any(|l| l.trim_start().starts_with("#ifndef"));
    let has_define = content
        .lines()
        .any(|l| l.trim_start().starts_with("#define"));
    let has_endif = content
        .lines()
        .any(|l| l.trim_start().starts_with("#endif"));

    if !has_ifndef || !has_define || !has_endif {
        return vec![Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Major,
            code: "C-H2".to_string(),
            message: "Header must be protected by include guard".to_string(),
        }];
    }
    vec![]
}

fn is_empty_define(rest: &str) -> bool {
    let mut cleaned = rest.to_string();
    if let Some(idx) = cleaned.find("//") {
        cleaned.truncate(idx);
    }
    while let Some(start) = cleaned.find("/*") {
        if let Some(end) = cleaned[start + 2..].find("*/") {
            cleaned = format!("{}{}", &cleaned[..start], &cleaned[start + 2 + end + 2..]);
        } else {
            cleaned.truncate(start);
            break;
        }
    }
    let cleaned = cleaned.trim();
    !cleaned.contains(' ') && !cleaned.contains('(')
}

fn check_forbidden_directives(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !filename.ends_with(".c") {
        return diagnostics;
    }

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#define") {
            if !is_empty_define(rest) {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: i + 1,
                    severity: Severity::Major,
                    code: "C-H1".to_string(),
                    message: "Non-empty macro definitions are forbidden in source files"
                        .to_string(),
                });
            }
            continue;
        }
        if trimmed.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == "typedef") {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Major,
                code: "C-H1".to_string(),
                message: "typedef is forbidden in source files".to_string(),
            });
        }
    }
    diagnostics
}

fn check_functions_placement(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let is_source = filename.ends_with(".c");
    let is_header = filename.ends_with(".h");
    if !is_source && !is_header {
        return diagnostics;
    }

    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };
    let root = tree.root_node();
    let bytes = content.as_bytes();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let declarator_start = node
                .child_by_field_name("body")
                .map(|b| b.start_byte())
                .unwrap_or(node.start_byte());
            let prefix = std::str::from_utf8(&bytes[node.start_byte()..declarator_start])
                .unwrap_or("");
            let is_static = prefix.contains("static");
            let is_inline = prefix.contains("inline");

            if is_source && is_static && is_inline {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: node.start_position().row + 1,
                    severity: Severity::Major,
                    code: "C-H1".to_string(),
                    message: "static inline functions are forbidden in source files".to_string(),
                });
            }
            if is_header && !(is_static && is_inline) {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: node.start_position().row + 1,
                    severity: Severity::Major,
                    code: "C-H1".to_string(),
                    message: "Only static inline function definitions are allowed in headers"
                        .to_string(),
                });
            }
        } else if is_source && node.kind() == "declaration" {
            if node_contains_kind(&node, "function_declarator") {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: node.start_position().row + 1,
                    severity: Severity::Major,
                    code: "C-H1".to_string(),
                    message: "Function prototypes are forbidden in source files".to_string(),
                });
            }
        }
    }
    diagnostics
}

fn node_contains_kind(node: &tree_sitter::Node, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if node_contains_kind(&child, kind) {
            return true;
        }
    }
    false
}

fn is_abusive_macro(line: &str) -> bool {
    if line.ends_with('\\') {
        return true;
    }
    let cleaned = line.replace("\\\"", "");
    cleaned
        .split('"')
        .step_by(2)
        .any(|segment| segment.contains(';'))
}

fn check_macro_size(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !filename.ends_with(".c") && !filename.ends_with(".h") {
        return diagnostics;
    }

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#define") || trimmed.starts_with("# define") {
            let line_trimmed_end = line.trim_end_matches(|c| c == '\r');
            if is_abusive_macro(line_trimmed_end) {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: i + 1,
                    severity: Severity::Major,
                    code: "C-H3".to_string(),
                    message: "Macro should fit on a single line with a single statement"
                        .to_string(),
                });
            }
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_h2_triggers_on_missing_include_guard() {
        let content = "void f(void);\n";
        let diags = check_include_guard("test.h", content);
        assert!(diags.iter().any(|d| d.code == "C-H2"));
    }

    #[test]
    fn c_h2_does_not_trigger_with_include_guard() {
        let content = "#ifndef TEST_H\n#define TEST_H\n\nvoid f(void);\n\n#endif\n";
        let diags = check_include_guard("test.h", content);
        assert!(!diags.iter().any(|d| d.code == "C-H2"));
    }

    #[test]
    fn c_h2_ignored_for_c_files() {
        let content = "void f(void);\n";
        let diags = check_include_guard("test.c", content);
        assert!(diags.is_empty());
    }

    #[test]
    fn c_h1_triggers_on_non_empty_macro_in_source() {
        let content = "#define MAX 42\n\nint main(void)\n{\n    return 0;\n}\n";
        let diags = check_forbidden_directives("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_triggers_on_typedef_in_source() {
        let content = "typedef int myint;\n\nint main(void)\n{\n    return 0;\n}\n";
        let diags = check_forbidden_directives("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_does_not_trigger_on_empty_macro_in_source() {
        let content = "#define MAX\n\nint main(void)\n{\n    return 0;\n}\n";
        let diags = check_forbidden_directives("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_triggers_on_static_inline_in_source() {
        let content = "static inline int f(void)\n{\n    return 0;\n}\n";
        let diags = check_functions_placement("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_does_not_trigger_on_plain_function_in_source() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_functions_placement("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_triggers_on_non_static_inline_function_in_header() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_functions_placement("test.h", content);
        assert!(diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_does_not_trigger_on_static_inline_function_in_header() {
        let content = "static inline int f(void)\n{\n    return 0;\n}\n";
        let diags = check_functions_placement("test.h", content);
        assert!(!diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h1_triggers_on_prototype_in_source() {
        let content = "int f(void);\n";
        let diags = check_functions_placement("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-H1"));
    }

    #[test]
    fn c_h3_triggers_on_multiline_macro() {
        let content = "#define FOO(x) \\\n    do { x; } while (0)\n";
        let diags = check_macro_size("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-H3"));
    }

    #[test]
    fn c_h3_triggers_on_macro_with_multiple_statements() {
        let content = "#define FOO(x) a = 1; b = 2;\n";
        let diags = check_macro_size("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-H3"));
    }

    #[test]
    fn c_h3_does_not_trigger_on_single_statement_macro() {
        let content = "#define FOO(x) (x + 1)\n";
        let diags = check_macro_size("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-H3"));
    }
}
