use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;
use regex::Regex;
use std::sync::OnceLock;
use tree_sitter::Node;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_multiple_statements(filename, content));
    diagnostics.extend(check_line_indentation(filename, content));
    diagnostics.extend(check_spaces(filename, content));
    diagnostics.extend(check_brace_placement(filename, content));
    diagnostics.extend(check_variable_declarations(filename, content));
    diagnostics.extend(check_line_breaks(filename, content));

    diagnostics
}

fn node_contains_kind(node: &Node, kind: &str) -> bool {
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

fn report_l1(filename: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(Diagnostic {
        file: filename.to_string(),
        line,
        severity: Severity::Major,
        code: "C-L1".to_string(),
        message: "Multiple statements or a chained assignment on a single line".to_string(),
    });
}

fn check_multiple_statements(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    walk_l1(&root, filename, &mut diagnostics);
    diagnostics
}

fn walk_l1(node: &Node, filename: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == "compound_statement" {
        let mut cursor = node.walk();
        let statements: Vec<Node> = node
            .children(&mut cursor)
            .filter(|c| c.kind() != "{" && c.kind() != "}")
            .collect();
        for i in 1..statements.len() {
            if statements[i].start_position().row == statements[i - 1].end_position().row {
                report_l1(filename, statements[i].start_position().row + 1, diagnostics);
            }
        }
    }

    match node.kind() {
        "if_statement" | "while_statement" | "switch_statement" => {
            if let Some(cond) = node.child_by_field_name("condition") {
                if node_contains_kind(&cond, "assignment_expression") {
                    report_l1(filename, node.start_position().row + 1, diagnostics);
                }
            }
        }
        "return_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if node_contains_kind(&child, "assignment_expression") {
                    report_l1(filename, node.start_position().row + 1, diagnostics);
                }
            }
        }
        "expression_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "comma_expression" {
                    report_l1(filename, node.start_position().row + 1, diagnostics);
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_l1(&child, filename, diagnostics);
    }
}

fn check_line_indentation(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut function_ranges: Vec<(usize, usize)> = Vec::new();
    collect_function_body_ranges(&root, &mut function_ranges);

    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let line_number = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        if line.trim_end().ends_with("*/") {
            continue;
        }
        if line.trim_start().starts_with('#') {
            continue;
        }

        let in_function = function_ranges
            .iter()
            .any(|(start, end)| line_number > *start && line_number < *end);

        let indent = line.len() - line.trim_start_matches(' ').len();

        let ok = if in_function {
            indent >= 4 && indent % 4 == 0
        } else {
            indent % 4 == 0
        };

        if !ok {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: line_number,
                severity: Severity::Minor,
                code: "C-L2".to_string(),
                message: "Line is not indented with 4-space groups".to_string(),
            });
        }
    }
    diagnostics
}

static KEYWORD_SPACE_RE: OnceLock<Regex> = OnceLock::new();
static COMMA_SPACE_RE: OnceLock<Regex> = OnceLock::new();
static SPACE_BEFORE_COMMA_SEMI_RE: OnceLock<Regex> = OnceLock::new();
static PAREN_SPACE_RE: OnceLock<Regex> = OnceLock::new();
static ARROW_SPACE_RE: OnceLock<Regex> = OnceLock::new();
static SEMI_NO_SPACE_RE: OnceLock<Regex> = OnceLock::new();
static DOUBLE_SPACE_RE: OnceLock<Regex> = OnceLock::new();

fn report_l3(filename: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(Diagnostic {
        file: filename.to_string(),
        line,
        severity: Severity::Minor,
        code: "C-L3".to_string(),
        message: "Incorrect spacing around an operator or keyword".to_string(),
    });
}

fn check_spaces(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let keyword_re = KEYWORD_SPACE_RE.get_or_init(|| {
        Regex::new(r"\b(if|switch|case|for|do|while|return|struct)(\(|;|\{)").unwrap()
    });
    let comma_re = COMMA_SPACE_RE.get_or_init(|| Regex::new(r",[^\s\n\)]").unwrap());
    let space_before_re =
        SPACE_BEFORE_COMMA_SEMI_RE.get_or_init(|| Regex::new(r"[ \t](,|;)").unwrap());
    let paren_re =
        PAREN_SPACE_RE.get_or_init(|| Regex::new(r"\([ \t]|[ \t]\)").unwrap());
    let arrow_re = ARROW_SPACE_RE.get_or_init(|| Regex::new(r"[ \t]->|->[ \t]").unwrap());
    let semi_re = SEMI_NO_SPACE_RE.get_or_init(|| Regex::new(r";[A-Za-z0-9_]").unwrap());
    let double_space_re = DOUBLE_SPACE_RE.get_or_init(|| Regex::new(r"\S(  +)\S").unwrap());

    for (i, line) in content.lines().enumerate() {
        let line_number = i + 1;
        let code = strip_line_comment(line);

        if keyword_re.is_match(code) {
            report_l3(filename, line_number, &mut diagnostics);
        }
        if comma_re.is_match(code) {
            report_l3(filename, line_number, &mut diagnostics);
        }
        if space_before_re.is_match(code) {
            report_l3(filename, line_number, &mut diagnostics);
        }
        if paren_re.is_match(code) {
            report_l3(filename, line_number, &mut diagnostics);
        }
        if arrow_re.is_match(code) {
            report_l3(filename, line_number, &mut diagnostics);
        }
        if semi_re.is_match(code) {
            report_l3(filename, line_number, &mut diagnostics);
        }
        if double_space_re.is_match(code.trim_start()) {
            report_l3(filename, line_number, &mut diagnostics);
        }
    }
    diagnostics
}

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn report_l4(filename: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(Diagnostic {
        file: filename.to_string(),
        line,
        severity: Severity::Minor,
        code: "C-L4".to_string(),
        message: "Curly bracket is misplaced".to_string(),
    });
}

fn check_brace_placement(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let lines: Vec<&str> = content.lines().collect();
    let root = tree.root_node();
    walk_l4(&root, &lines, filename, &mut diagnostics);
    diagnostics
}

fn walk_l4(node: &Node, lines: &[&str], filename: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == "compound_statement" {
        let is_function_body = node
            .parent()
            .map(|p| p.kind() == "function_definition")
            .unwrap_or(false);
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        if let (Some(open_brace), Some(close_brace)) =
            (children.first(), children.last())
        {
            let open_row = open_brace.start_position().row;
            if let Some(line_text) = lines.get(open_row) {
                let before_col = open_brace.start_position().column.min(line_text.len());
                let after_col = open_brace.end_position().column.min(line_text.len());
                let prefix = line_text[..before_col].trim();
                let suffix = strip_line_comment(&line_text[after_col..]).trim();
                let misplaced = if is_function_body {
                    !prefix.is_empty() || !suffix.is_empty()
                } else {
                    prefix.is_empty() || !suffix.is_empty()
                };
                if misplaced {
                    report_l4(filename, open_row + 1, diagnostics);
                }
            }

            let close_row = close_brace.start_position().row;
            if let Some(line_text) = lines.get(close_row) {
                let before_col = close_brace.start_position().column.min(line_text.len());
                let after_col = close_brace.end_position().column.min(line_text.len());
                let prefix = line_text[..before_col].trim();
                let suffix = strip_line_comment(&line_text[after_col..]).trim();
                if !prefix.is_empty()
                    || (!suffix.is_empty()
                        && !suffix.starts_with("else")
                        && !suffix.starts_with("while"))
                {
                    report_l4(filename, close_row + 1, diagnostics);
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_l4(&child, lines, filename, diagnostics);
    }
}

fn check_variable_declarations(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    walk_l5(&root, filename, &mut diagnostics);
    diagnostics
}

fn walk_l5(node: &Node, filename: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == "compound_statement" {
        let mut declaration_zone = true;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" | "}" | "comment" => continue,
                "declaration" => {
                    let comma_count = child
                        .children(&mut child.walk())
                        .filter(|c| c.kind() == ",")
                        .count();
                    if comma_count > 0 || !declaration_zone {
                        diagnostics.push(Diagnostic {
                            file: filename.to_string(),
                            line: child.start_position().row + 1,
                            severity: Severity::Major,
                            code: "C-L5".to_string(),
                            message: "Only one variable must be declared per line, at the top of the scope"
                                .to_string(),
                        });
                    }
                }
                _ => {
                    declaration_zone = false;
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_l5(&child, filename, diagnostics);
    }
}

fn check_line_breaks(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let lines: Vec<&str> = content.lines().collect();
    let root = tree.root_node();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() != "function_definition" {
            continue;
        }
        let Some(body) = node.child_by_field_name("body") else {
            continue;
        };
        let body_start_row = body.start_position().row;
        let body_end_row = body.end_position().row;

        let mut empty_lines: Vec<usize> = Vec::new();
        for row in body_start_row..=body_end_row {
            if let Some(line) = lines.get(row) {
                if line.trim().is_empty() {
                    empty_lines.push(row + 1);
                }
            }
        }

        let mut bc = body.walk();
        let statements: Vec<Node> = body
            .children(&mut bc)
            .filter(|c| c.kind() != "{" && c.kind() != "}" && c.kind() != "comment")
            .collect();

        let mut decl_count = 0;
        for stmt in &statements {
            if stmt.kind() == "declaration" {
                decl_count += 1;
            } else {
                break;
            }
        }

        if decl_count > 0 && decl_count < statements.len() {
            let first_after = &statements[decl_count];
            let first_stmt_line = first_after.start_position().row + 1;
            let mandatory_blank_line = first_stmt_line - 1;

            if !empty_lines.contains(&mandatory_blank_line) {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: first_stmt_line,
                    severity: Severity::Minor,
                    code: "C-L6".to_string(),
                    message: "Missing blank line after variable declarations".to_string(),
                });
            } else {
                empty_lines.retain(|l| *l != mandatory_blank_line);
            }
        }

        for line in empty_lines {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line,
                severity: Severity::Minor,
                code: "C-L6".to_string(),
                message: "Unnecessary blank line inside function".to_string(),
            });
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_l1_triggers_on_multiple_statements_per_line() {
        let content = "int f(void)\n{\n    int a; a = 1;\n    return a;\n}\n";
        let diags = check_multiple_statements("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L1"));
    }

    #[test]
    fn c_l1_does_not_trigger_with_one_statement_per_line() {
        let content = "int f(void)\n{\n    int a;\n    a = 1;\n    return a;\n}\n";
        let diags = check_multiple_statements("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-L1"));
    }

    #[test]
    fn c_l1_triggers_on_assignment_in_condition() {
        let content = "int f(int a)\n{\n    if ((a = 1)) {\n        return a;\n    }\n    return 0;\n}\n";
        let diags = check_multiple_statements("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L1"));
    }

    #[test]
    fn c_l2_triggers_on_bad_indentation() {
        let content = "int f(void)\n{\n  return 0;\n}\n";
        let diags = check_line_indentation("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L2"));
    }

    #[test]
    fn c_l2_does_not_trigger_on_four_space_indentation() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_line_indentation("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-L2"));
    }

    #[test]
    fn c_l3_triggers_on_missing_space_before_brace() {
        let content = "int f(void)\n{\n    if(1) {\n        return 1;\n    }\n    return 0;\n}\n";
        let diags = check_spaces("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L3"));
    }

    #[test]
    fn c_l3_triggers_on_space_before_semicolon() {
        let content = "int f(void)\n{\n    return 0 ;\n}\n";
        let diags = check_spaces("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L3"));
    }

    #[test]
    fn c_l3_does_not_trigger_on_correct_spacing() {
        let content = "int f(void)\n{\n    if (1) {\n        return 1;\n    }\n    return 0;\n}\n";
        let diags = check_spaces("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-L3"));
    }

    #[test]
    fn c_l4_triggers_on_brace_not_on_own_line_for_function() {
        let content = "int f(void) {\n    return 0;\n}\n";
        let diags = check_brace_placement("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L4"));
    }

    #[test]
    fn c_l4_does_not_trigger_on_correct_function_brace() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_brace_placement("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-L4"));
    }

    #[test]
    fn c_l5_triggers_on_multiple_declarations_per_line() {
        let content = "int f(void)\n{\n    int a, b;\n    return a + b;\n}\n";
        let diags = check_variable_declarations("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L5"));
    }

    #[test]
    fn c_l5_triggers_on_declaration_after_statement() {
        let content = "int f(void)\n{\n    int a;\n\n    a = 1;\n    int b;\n    return a + b;\n}\n";
        let diags = check_variable_declarations("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L5"));
    }

    #[test]
    fn c_l5_does_not_trigger_on_single_declarations_at_top() {
        let content = "int f(void)\n{\n    int a;\n    int b;\n\n    a = 1;\n    return a + b;\n}\n";
        let diags = check_variable_declarations("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-L5"));
    }

    #[test]
    fn c_l6_triggers_on_missing_blank_line_after_declarations() {
        let content = "int f(void)\n{\n    int a;\n    a = 1;\n    return a;\n}\n";
        let diags = check_line_breaks("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L6"));
    }

    #[test]
    fn c_l6_triggers_on_unnecessary_blank_line() {
        let content = "int f(void)\n{\n    int a;\n\n    a = 1;\n\n    return a;\n}\n";
        let diags = check_line_breaks("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-L6"));
    }

    #[test]
    fn c_l6_does_not_trigger_on_correct_blank_line_usage() {
        let content = "int f(void)\n{\n    int a;\n\n    a = 1;\n    return a;\n}\n";
        let diags = check_line_breaks("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-L6"));
    }
}

fn collect_function_body_ranges(node: &Node, out: &mut Vec<(usize, usize)>) {
    if node.kind() == "function_definition" {
        if let Some(body) = node.child_by_field_name("body") {
            out.push((body.start_position().row + 1, body.end_position().row + 1));
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_function_body_ranges(&child, out);
    }
}
