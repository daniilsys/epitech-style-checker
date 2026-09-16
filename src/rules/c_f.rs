use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_line_length(filename, content));
    diagnostics.extend(check_function_names(filename, content));
    diagnostics.extend(check_function_length(filename, content));
    diagnostics.extend(check_function_params(filename, content));
    diagnostics.extend(check_empty_params(filename, content));
    diagnostics.extend(check_nested_functions(filename, content));
    diagnostics.extend(check_comments_in_functions(filename, content));
    diagnostics.extend(check_struct_by_copy(filename, content));

    diagnostics
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

fn check_function_names(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let declarator = node.child_by_field_name("declarator");
            if let Some(decl) = declarator {
                let mut c = decl.walk();
                for child in decl.children(&mut c) {
                    if child.kind() == "identifier" {
                        let name = child.utf8_text(content.as_bytes()).unwrap_or("");
                        let clean = name.replace('_', "");
                        if !name
                            .chars()
                            .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
                            || clean.len() <= 2
                        {
                            diagnostics.push(Diagnostic {
                                file: filename.to_string(),
                                line: node.start_position().row + 1,
                                severity: Severity::Minor,
                                code: "C-F2".to_string(),
                                message: format!("Function name '{}' must be in snake_case", name),
                            });
                        }
                    }
                }
            }
        }
    }
    diagnostics
}

fn check_function_length(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let start = node.start_position().row;
            let end = node.end_position().row;
            let line_count = end - start - 1;

            if line_count > 20 {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: start + 1,
                    severity: Severity::Major,
                    code: "C-F4".to_string(),
                    message: format!("Function exceeds 20 lines ({})", line_count),
                });
            }
        }
    }
    diagnostics
}

fn check_function_params(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let declarator = node.child_by_field_name("declarator");
            if let Some(decl) = declarator {
                let mut c = decl.walk();
                for child in decl.children(&mut c) {
                    if child.kind() == "parameter_list" {
                        let param_count = child
                            .children(&mut child.walk())
                            .filter(|n| n.kind() == "parameter_declaration")
                            .count();
                        if param_count > 4 {
                            diagnostics.push(Diagnostic {
                                file: filename.to_string(),
                                line: node.start_position().row + 1,
                                severity: Severity::Major,
                                code: "C-F5".to_string(),
                                message: format!(
                                    "Function has too many parameters ({})",
                                    param_count
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
    diagnostics
}

fn check_empty_params(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let declarator = node.child_by_field_name("declarator");
            if let Some(decl) = declarator {
                let mut c = decl.walk();
                for child in decl.children(&mut c) {
                    if child.kind() == "parameter_list" {
                        let params: Vec<_> = child
                            .children(&mut child.walk())
                            .filter(|n| n.kind() == "parameter_declaration")
                            .collect();
                        let has_void = child.children(&mut child.walk()).any(|n| {
                            n.kind() == "void"
                                || (n.kind() == "parameter_declaration"
                                    && n.utf8_text(content.as_bytes()).unwrap_or("") == "void")
                        });
                        if params.is_empty() && !has_void {
                            diagnostics.push(Diagnostic {
                                file: filename.to_string(),
                                line: node.start_position().row + 1,
                                severity: Severity::Major,
                                code: "C-F6".to_string(),
                                message: "Function with no parameters must take void".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    diagnostics
}
fn check_nested_functions(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let body = node.child_by_field_name("body");
            if let Some(body) = body {
                let mut body_cursor = body.walk();
                for child in body.children(&mut body_cursor) {
                    if child.kind() == "function_definition" {
                        diagnostics.push(Diagnostic {
                            file: filename.to_string(),
                            line: child.start_position().row + 1,
                            severity: Severity::Major,
                            code: "C-F9".to_string(),
                            message: "Nested functions are not allowed".to_string(),
                        });
                    }
                }
            }
        }
    }
    diagnostics
}

fn check_comments_in_functions(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let body = node.child_by_field_name("body");
            if let Some(body) = body {
                find_comments_in_node(&body, content, filename, &mut diagnostics);
            }
        }
    }
    diagnostics
}

fn find_comments_in_node(
    node: &tree_sitter::Node,
    content: &str,
    filename: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "comment" {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: child.start_position().row + 1,
                severity: Severity::Minor,
                code: "C-F8".to_string(),
                message: "No comments inside a function".to_string(),
            });
        }
        find_comments_in_node(&child, content, filename, diagnostics);
    }
}

fn check_struct_by_copy(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            let declarator = node.child_by_field_name("declarator");
            if let Some(decl) = declarator {
                let mut c = decl.walk();
                for child in decl.children(&mut c) {
                    if child.kind() == "parameter_list" {
                        let mut pc = child.walk();
                        for param in child.children(&mut pc) {
                            if param.kind() == "parameter_declaration" {
                                let text = param.utf8_text(content.as_bytes()).unwrap_or("");
                if text.contains("struct") && !text.contains('*') {
                                    diagnostics.push(Diagnostic {
                                        file: filename.to_string(),
                                        line: param.start_position().row + 1,
                                        severity: Severity::Major,
                                        code: "C-F7".to_string(),
                                        message: "Structures must be passed by pointer".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_f3_triggers_on_long_line() {
        let long_line = "int a = ".to_string() + &"1".repeat(80) + ";";
        let content = format!("{}\n", long_line);
        let diags = check_line_length("test.c", &content);
        assert!(diags.iter().any(|d| d.code == "C-F3"));
    }

    #[test]
    fn c_f3_does_not_trigger_on_short_line() {
        let content = "int a = 1;\n";
        let diags = check_line_length("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F3"));
    }

    #[test]
    fn c_f2_triggers_on_uppercase_name() {
        let content = "int Foo(void)\n{\n    return 0;\n}\n";
        let diags = check_function_names("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F2"));
    }

    #[test]
    fn c_f2_triggers_on_too_short_name() {
        let content = "int ab(void)\n{\n    return 0;\n}\n";
        let diags = check_function_names("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F2"));
    }

    #[test]
    fn c_f2_does_not_trigger_on_valid_snake_case() {
        let content = "int foo_bar(void)\n{\n    return 0;\n}\n";
        let diags = check_function_names("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F2"));
    }

    #[test]
    fn c_f4_triggers_when_function_too_long() {
        let mut body = String::new();
        for _ in 0..22 {
            body.push_str("    a = a + 1;\n");
        }
        let content = format!("int f(int a)\n{{\n{}    return a;\n}}\n", body);
        let diags = check_function_length("test.c", &content);
        assert!(diags.iter().any(|d| d.code == "C-F4"));
    }

    #[test]
    fn c_f4_does_not_trigger_when_function_short() {
        let mut body = String::new();
        for _ in 0..5 {
            body.push_str("    a = a + 1;\n");
        }
        let content = format!("int f(int a)\n{{\n{}    return a;\n}}\n", body);
        let diags = check_function_length("test.c", &content);
        assert!(!diags.iter().any(|d| d.code == "C-F4"));
    }

    #[test]
    fn c_f5_triggers_with_too_many_params() {
        let content = "int f(int a, int b, int c, int d, int e)\n{\n    return a;\n}\n";
        let diags = check_function_params("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F5"));
    }

    #[test]
    fn c_f5_does_not_trigger_with_four_params() {
        let content = "int f(int a, int b, int c, int d)\n{\n    return a;\n}\n";
        let diags = check_function_params("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F5"));
    }

    #[test]
    fn c_f6_triggers_on_empty_parameter_list() {
        let content = "int f()\n{\n    return 0;\n}\n";
        let diags = check_empty_params("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F6"));
    }

    #[test]
    fn c_f6_does_not_trigger_with_void() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_empty_params("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F6"));
    }

    #[test]
    fn c_f9_triggers_on_nested_function() {
        let content = "int f(void)\n{\n    int g(void) { return 1; }\n    return g();\n}\n";
        let diags = check_nested_functions("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F9"));
    }

    #[test]
    fn c_f9_does_not_trigger_without_nested_function() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_nested_functions("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F9"));
    }

    #[test]
    fn c_f8_triggers_on_comment_in_function() {
        let content = "int f(void)\n{\n    // comment\n    return 0;\n}\n";
        let diags = check_comments_in_functions("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F8"));
    }

    #[test]
    fn c_f8_does_not_trigger_without_comment() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_comments_in_functions("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F8"));
    }

    #[test]
    fn c_f7_triggers_on_struct_by_copy() {
        let content = "int f(struct point p)\n{\n    return 0;\n}\n";
        let diags = check_struct_by_copy("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-F7"));
    }

    #[test]
    fn c_f7_does_not_trigger_on_struct_pointer() {
        let content = "int f(struct point *p)\n{\n    return 0;\n}\n";
        let diags = check_struct_by_copy("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-F7"));
    }
}
