use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_line_length(filename, content));
    diagnostics.extend(check_function_length(filename, content));
    diagnostics.extend(check_function_params(filename, content));

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
