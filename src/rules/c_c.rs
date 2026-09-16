use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;
use tree_sitter::Node;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_goto(filename, content));
    diagnostics.extend(check_conditional_branching(filename, content));
    diagnostics.extend(check_ternary_operator(filename, content));

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

const MAX_BRANCHING_DEPTH: usize = 2;
const CONTROL_KINDS: [&str; 5] = [
    "if_statement",
    "for_statement",
    "while_statement",
    "do_statement",
    "switch_statement",
];

fn check_conditional_branching(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            if let Some(body) = node.child_by_field_name("body") {
                walk_branching_depth(&body, 0, filename, &mut diagnostics);
            }
        }
    }
    diagnostics
}

fn walk_branching_depth(
    node: &Node,
    depth: usize,
    filename: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if CONTROL_KINDS.contains(&child.kind()) {
            let new_depth = depth + 1;
            if new_depth > MAX_BRANCHING_DEPTH {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: child.start_position().row + 1,
                    severity: Severity::Major,
                    code: "C-C1".to_string(),
                    message: "Conditional branching is nested too deeply".to_string(),
                });
            }
            walk_branching_depth(&child, new_depth, filename, diagnostics);
        } else {
            walk_branching_depth(&child, depth, filename, diagnostics);
        }
    }
}

fn check_ternary_operator(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    for node in root.children(&mut cursor) {
        if node.kind() == "function_definition" {
            if let Some(body) = node.child_by_field_name("body") {
                find_ternary(&body, content, filename, &mut diagnostics);
            }
        }
    }
    diagnostics
}

fn find_ternary(
    node: &Node,
    content: &str,
    filename: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if node.kind() == "conditional_expression" {
        check_ternary_node(node, content, filename, diagnostics);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_ternary(&child, content, filename, diagnostics);
    }
}

fn report_ternary(node: &Node, filename: &str, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(Diagnostic {
        file: filename.to_string(),
        line: node.start_position().row + 1,
        severity: Severity::Major,
        code: "C-C2".to_string(),
        message: "Misuse of the ternary operator".to_string(),
    });
}

fn check_ternary_node(
    node: &Node,
    content: &str,
    filename: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let consequence = node.child_by_field_name("consequence");
    let alternative = node.child_by_field_name("alternative");

    let parent_kind = node.parent().map(|p| p.kind()).unwrap_or("");
    if parent_kind == "expression_statement" {
        report_ternary(node, filename, diagnostics);
        return;
    }

    if let Some(cons) = consequence {
        if node_contains_kind(&cons, "conditional_expression")
            || node_contains_kind(&cons, "assignment_expression")
        {
            report_ternary(&cons, filename, diagnostics);
        }
    }
    if let Some(alt) = alternative {
        if node_contains_kind(&alt, "conditional_expression")
            || node_contains_kind(&alt, "assignment_expression")
        {
            report_ternary(&alt, filename, diagnostics);
        }
    }

    if let Some(cond) = node.child_by_field_name("condition") {
        if node_contains_kind(&cond, "assignment_expression") {
            report_ternary(&cond, filename, diagnostics);
        }
    }

    if let (Some(cons), Some(alt)) = (consequence, alternative) {
        let cons_text = cons.utf8_text(content.as_bytes()).unwrap_or("").trim();
        let alt_text = alt.utf8_text(content.as_bytes()).unwrap_or("").trim();
        if cons_text == alt_text && !cons_text.is_empty() {
            report_ternary(node, filename, diagnostics);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_c3_triggers_on_goto() {
        let content = "int main(void)\n{\n    goto end;\nend:\n    return 0;\n}\n";
        let diags = check_goto("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-C3"));
    }

    #[test]
    fn c_c3_does_not_trigger_without_goto() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check_goto("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-C3"));
    }

    #[test]
    fn c_c1_triggers_when_nesting_exceeds_two() {
        let content = "int f(int a)\n{\n    if (a) {\n        if (a) {\n            if (a) {\n                a = 1;\n            }\n        }\n    }\n    return a;\n}\n";
        let diags = check_conditional_branching("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-C1"));
    }

    #[test]
    fn c_c1_does_not_trigger_at_exactly_two_levels() {
        let content = "int f(int a)\n{\n    if (a) {\n        if (a) {\n            a = 1;\n        }\n    }\n    return a;\n}\n";
        let diags = check_conditional_branching("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-C1"));
    }

    #[test]
    fn c_c2_triggers_on_ternary_as_statement() {
        let content = "int f(int a)\n{\n    a ? f(1) : f(0);\n    return a;\n}\n";
        let diags = check_ternary_operator("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-C2"));
    }

    #[test]
    fn c_c2_triggers_on_nested_ternary() {
        let content = "int f(int a)\n{\n    return a ? (a ? 1 : 2) : 0;\n}\n";
        let diags = check_ternary_operator("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-C2"));
    }

    #[test]
    fn c_c2_does_not_trigger_on_simple_ternary_in_return() {
        let content = "int f(int a)\n{\n    return a ? 1 : 0;\n}\n";
        let diags = check_ternary_operator("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-C2"));
    }
}
