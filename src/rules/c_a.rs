use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;
use regex::Regex;
use tree_sitter::Node;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_file_end(filename, content));
    diagnostics.extend(check_const_pointers(filename, content));
    diagnostics.extend(check_typing(filename, content));

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

/// Resolves the identifier name at the bottom of a (possibly nested)
/// declarator, and whether a `function_declarator` was encountered along
/// the way (meaning the declarator ultimately names a function, not a
/// plain variable).
fn resolve_declarator_name<'a>(node: Node, content: &'a str) -> Option<(&'a str, bool)> {
    match node.kind() {
        "identifier" | "field_identifier" => {
            Some((node.utf8_text(content.as_bytes()).unwrap_or(""), false))
        }
        "pointer_declarator" | "init_declarator" | "array_declarator" | "parenthesized_declarator" => {
            let inner = node.child_by_field_name("declarator")?;
            resolve_declarator_name(inner, content)
        }
        "function_declarator" => {
            let inner = node.child_by_field_name("declarator")?;
            resolve_declarator_name(inner, content).map(|(name, _)| (name, true))
        }
        _ => None,
    }
}

/// Returns the parameter_list node reachable from a function_definition's
/// declarator, unwrapping a pointer-returning declarator if needed.
fn find_parameter_list<'a>(node: Node<'a>) -> Option<Node<'a>> {
    match node.kind() {
        "function_declarator" => node.child_by_field_name("parameters"),
        "pointer_declarator" => find_parameter_list(node.child_by_field_name("declarator")?),
        _ => None,
    }
}

fn has_storage_class(node: Node, content: &str, keyword: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|c| {
        c.kind() == "storage_class_specifier"
            && c.utf8_text(content.as_bytes()).unwrap_or("") == keyword
    })
}

fn check_const_pointers(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };
    let bytes = content.as_bytes();
    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() != "function_definition" {
            continue;
        }
        let declarator = match node.child_by_field_name("declarator") {
            Some(d) => d,
            None => continue,
        };
        let params = match find_parameter_list(declarator) {
            Some(p) => p,
            None => continue,
        };
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => continue,
        };

        let mut pcursor = params.walk();
        for param in params.children(&mut pcursor) {
            if param.kind() != "parameter_declaration" {
                continue;
            }
            let param_declarator = match param.child_by_field_name("declarator") {
                Some(d) => d,
                None => continue,
            };
            if param_declarator.kind() != "pointer_declarator" {
                continue;
            }
            // "const" on the pointee appears as a type_qualifier child that
            // precedes the pointer_declarator (e.g. `const char *s`); the
            // `type` field alone only captures the base type node.
            let already_const = {
                let mut c = param.walk();
                param.children(&mut c).any(|n| {
                    n.kind() == "type_qualifier"
                        && n.start_byte() < param_declarator.start_byte()
                        && n.utf8_text(bytes) == Ok("const")
                })
            };
            if already_const {
                continue;
            }
            let type_field = param
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(bytes).ok())
                .unwrap_or("");
            let base_type = type_field.split_whitespace().collect::<Vec<_>>().join(" ");
            if base_type == "void" {
                continue;
            }
            let (name, is_function) = match resolve_declarator_name(param_declarator, content) {
                Some(r) => r,
                None => continue,
            };
            if is_function {
                continue;
            }

            let (written, passed_to_call) = scan_pointer_usage(body, content, name);
            if !written && !passed_to_call {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: param.start_position().row + 1,
                    severity: Severity::Info,
                    code: "C-A1".to_string(),
                    message: format!("Pointer parameter '{}' should be const", name),
                });
            }
        }
    }
    diagnostics
}

/// Scans a function body for writes through `name` (as a pointer) and for
/// any bare use of `name` as a call argument. Returns (written, passed_to_call).
fn scan_pointer_usage(body: Node, content: &str, name: &str) -> (bool, bool) {
    let mut written = false;
    let mut passed_to_call = false;
    walk_pointer_usage(body, content, name, &mut written, &mut passed_to_call);
    (written, passed_to_call)
}

fn walk_pointer_usage(
    node: Node,
    content: &str,
    name: &str,
    written: &mut bool,
    passed_to_call: &mut bool,
) {
    if *written && *passed_to_call {
        return;
    }
    let bytes = content.as_bytes();

    if node.kind() == "assignment_expression" {
        if let Some(left) = node.child_by_field_name("left") {
            let is_write = match left.kind() {
                "pointer_expression" => left
                    .child_by_field_name("argument")
                    .and_then(|n| n.utf8_text(bytes).ok())
                    == Some(name),
                "subscript_expression" => left
                    .child_by_field_name("argument")
                    .and_then(|n| n.utf8_text(bytes).ok())
                    == Some(name),
                "field_expression" => {
                    left.child_by_field_name("operator").map(|o| o.kind()) == Some("->")
                        && left
                            .child_by_field_name("argument")
                            .and_then(|n| n.utf8_text(bytes).ok())
                            == Some(name)
                }
                _ => false,
            };
            if is_write {
                *written = true;
            }
        }
    }

    if node.kind() == "call_expression" {
        if let Some(args) = node.child_by_field_name("arguments") {
            let mut acursor = args.walk();
            for arg in args.children(&mut acursor) {
                if arg.kind() == "identifier" && arg.utf8_text(bytes).ok() == Some(name) {
                    *passed_to_call = true;
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_pointer_usage(child, content, name, written, passed_to_call);
    }
}

fn check_typing(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };
    let root = tree.root_node();
    walk_typing(root, content, filename, &mut diagnostics);
    diagnostics
}

fn is_counter_name(name: &str) -> bool {
    name == "counter" || name.contains("count") || name.ends_with("_count") || name.starts_with("nb_")
}

fn walk_typing(node: Node, content: &str, filename: &str, diagnostics: &mut Vec<Diagnostic>) {
    let bytes = content.as_bytes();

    if node.kind() == "declaration" {
        let type_field = node
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(bytes).ok())
            .unwrap_or("");
        if type_field.trim() == "int" {
            let mut cursor = node.walk();
            for declarator in node.children_by_field_name("declarator", &mut cursor) {
                if let Some((name, is_function)) = resolve_declarator_name(declarator, content) {
                    if !is_function && is_counter_name(name) {
                        diagnostics.push(Diagnostic {
                            file: filename.to_string(),
                            line: node.start_position().row + 1,
                            severity: Severity::Info,
                            code: "C-A2".to_string(),
                            message: format!(
                                "Variable '{}' looks like a counter and should use an unsigned type",
                                name
                            ),
                        });
                    }
                }
            }
        }
    }

    if node.kind() == "function_definition" {
        let type_field = node
            .child_by_field_name("type")
            .and_then(|n| n.utf8_text(bytes).ok())
            .unwrap_or("");
        let normalized = type_field.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized == "int" || normalized == "unsigned int" {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if declarator.kind() == "function_declarator" {
                    if let Some((name, _)) = resolve_declarator_name(declarator, content) {
                        let is_size_like = name
                            .split('_')
                            .any(|part| part == "size" || part == "len" || part == "length");
                        if is_size_like {
                            diagnostics.push(Diagnostic {
                                file: filename.to_string(),
                                line: node.start_position().row + 1,
                                severity: Severity::Info,
                                code: "C-A2".to_string(),
                                message: format!(
                                    "Function '{}' returns a size-like value and should return size_t",
                                    name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_typing(child, content, filename, diagnostics);
    }
}

/// Project-wide check for C-A4: top-level non-static functions/globals in
/// .c files that are never referenced from any other file in the project.
pub fn check_static_usage(files: &[(String, String)]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (idx, (filename, content)) in files.iter().enumerate() {
        if filename.ends_with(".h") {
            continue;
        }
        let tree = match parse(content) {
            Some(t) => t,
            None => continue,
        };
        let root = tree.root_node();
        let mut cursor = root.walk();

        for node in root.children(&mut cursor) {
            match node.kind() {
                "function_definition" => {
                    if has_storage_class(node, content, "static") {
                        continue;
                    }
                    let declarator = match node.child_by_field_name("declarator") {
                        Some(d) => d,
                        None => continue,
                    };
                    let name = match resolve_declarator_name(declarator, content) {
                        Some((n, _)) => n,
                        None => continue,
                    };
                    if name == "main" {
                        continue;
                    }
                    if !used_in_other_files(name, files, idx) {
                        diagnostics.push(Diagnostic {
                            file: filename.to_string(),
                            line: node.start_position().row + 1,
                            severity: Severity::Info,
                            code: "C-A4".to_string(),
                            message: format!(
                                "'{}' is not used outside this file and should be marked static",
                                name
                            ),
                        });
                    }
                }
                "declaration" => {
                    if has_storage_class(node, content, "static")
                        || has_storage_class(node, content, "extern")
                    {
                        continue;
                    }
                    let mut dcursor = node.walk();
                    for declarator in node.children_by_field_name("declarator", &mut dcursor) {
                        let (name, is_function) =
                            match resolve_declarator_name(declarator, content) {
                                Some(r) => r,
                                None => continue,
                            };
                        if is_function {
                            // function prototype, not a variable definition
                            continue;
                        }
                        if !used_in_other_files(name, files, idx) {
                            diagnostics.push(Diagnostic {
                                file: filename.to_string(),
                                line: node.start_position().row + 1,
                                severity: Severity::Info,
                                code: "C-A4".to_string(),
                                message: format!(
                                    "'{}' is not used outside this file and should be marked static",
                                    name
                                ),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    diagnostics
}

fn used_in_other_files(name: &str, files: &[(String, String)], self_idx: usize) -> bool {
    if name.is_empty() {
        return true;
    }
    let re = match Regex::new(&format!(r"\b{}\b", regex::escape(name))) {
        Ok(r) => r,
        Err(_) => return true,
    };
    files
        .iter()
        .enumerate()
        .any(|(i, (_, content))| i != self_idx && re.is_match(content))
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

    #[test]
    fn c_a1_triggers_when_pointer_param_never_written_or_passed() {
        let content = "int f(int *ptr)\n{\n    return *ptr;\n}\n";
        let diags = check_const_pointers("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-A1"));
    }

    #[test]
    fn c_a1_does_not_trigger_when_pointer_is_written() {
        let content = "int f(int *ptr)\n{\n    *ptr = 1;\n    return 0;\n}\n";
        let diags = check_const_pointers("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A1"));
    }

    #[test]
    fn c_a1_does_not_trigger_when_already_const() {
        let content = "int f(const int *ptr)\n{\n    return *ptr;\n}\n";
        let diags = check_const_pointers("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A1"));
    }

    #[test]
    fn c_a1_does_not_trigger_when_passed_to_another_call() {
        let content = "int f(int *ptr)\n{\n    bar(ptr);\n    return 0;\n}\n";
        let diags = check_const_pointers("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A1"));
    }

    #[test]
    fn c_a1_does_not_trigger_on_void_pointer() {
        let content = "int f(void *data)\n{\n    return 0;\n}\n";
        let diags = check_const_pointers("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A1"));
    }

    #[test]
    fn c_a2_triggers_on_counter_variable() {
        let content = "int f(void)\n{\n    int counter;\n    counter = 0;\n    return counter;\n}\n";
        let diags = check_typing("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-A2"));
    }

    #[test]
    fn c_a2_does_not_trigger_on_regular_int() {
        let content = "int f(void)\n{\n    int result;\n    result = 0;\n    return result;\n}\n";
        let diags = check_typing("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A2"));
    }

    #[test]
    fn c_a2_triggers_on_size_like_function_returning_int() {
        let content = "int get_size(void)\n{\n    return 0;\n}\n";
        let diags = check_typing("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-A2"));
    }

    #[test]
    fn c_a2_does_not_trigger_on_regular_function() {
        let content = "int compute(void)\n{\n    return 0;\n}\n";
        let diags = check_typing("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-A2"));
    }

    #[test]
    fn c_a4_triggers_on_unused_non_static_function() {
        let files = vec![
            (
                "a.c".to_string(),
                "int helper(void)\n{\n    return 0;\n}\n\nint main(void)\n{\n    return helper();\n}\n"
                    .to_string(),
            ),
            ("b.c".to_string(), "int other(void)\n{\n    return 1;\n}\n".to_string()),
        ];
        let diags = check_static_usage(&files);
        assert!(diags.iter().any(|d| d.code == "C-A4" && d.file == "a.c"));
    }

    #[test]
    fn c_a4_does_not_trigger_when_used_in_another_file() {
        let files = vec![
            (
                "a.c".to_string(),
                "int helper(void)\n{\n    return 0;\n}\n".to_string(),
            ),
            (
                "b.c".to_string(),
                "int main(void)\n{\n    return helper();\n}\n".to_string(),
            ),
        ];
        let diags = check_static_usage(&files);
        assert!(!diags.iter().any(|d| d.code == "C-A4" && d.file == "a.c"));
    }

    #[test]
    fn c_a4_does_not_trigger_when_already_static() {
        let files = vec![
            (
                "a.c".to_string(),
                "static int helper(void)\n{\n    return 0;\n}\n".to_string(),
            ),
            ("b.c".to_string(), "int main(void)\n{\n    return 0;\n}\n".to_string()),
        ];
        let diags = check_static_usage(&files);
        assert!(!diags.iter().any(|d| d.code == "C-A4"));
    }
}
