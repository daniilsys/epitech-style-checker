use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;
use regex::Regex;
use std::sync::OnceLock;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_header(filename, content));
    diagnostics.extend(check_function_separation(filename, content));
    diagnostics.extend(check_trailing_spaces(filename, content));
    diagnostics.extend(check_line_endings(filename, content));
    diagnostics.extend(check_leading_trailing_lines(filename, content));
    diagnostics.extend(check_preprocessor_indentation(filename, content));
    diagnostics.extend(check_global_variable_constness(filename, content));
    diagnostics.extend(check_non_header_inclusion(filename, content));
    diagnostics.extend(check_inline_assembly(filename, content));

    diagnostics
}
static HEADER_RE: OnceLock<Regex> = OnceLock::new();

fn check_header(filename: &str, content: &str) -> Vec<Diagnostic> {
    let re = HEADER_RE.get_or_init(|| {
        Regex::new(
            r"(?s)^/\*\n\*\* EPITECH PROJECT, [1-9][0-9]{3}\n\*\* \S.+\n\*\* File description:\n(\*\* .*\n)+\*/(\n|$)"
        ).unwrap()
    });
    if !re.is_match(content) {
        return vec![Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Minor,
            code: "C-G1".to_string(),
            message: "Missing or invalid Epitech header".to_string(),
        }];
    }
    vec![]
}

fn check_function_separation(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let lines: Vec<&str> = content.split('\n').collect();
    let mut empty_count = 0;
    let mut last_closing_brace = false;

    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            empty_count += 1;
        } else {
            if last_closing_brace && empty_count != 1 && empty_count > 0 {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: i + 1,
                    severity: Severity::Major,
                    code: "C-G2".to_string(),
                    message: "Functions must be separated by one and only one empty line"
                        .to_string(),
                });
            }
            last_closing_brace = line.trim() == "}";
            empty_count = 0;
        }
    }
    diagnostics
}

fn check_line_endings(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (i, line) in content.split('\n').enumerate() {
        if line.ends_with('\r') || line.ends_with("\\") {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Minor,
                code: "C-G6".to_string(),
                message: "Line ends with CR character".to_string(),
            });
        }
    }
    diagnostics
}

fn check_trailing_spaces(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if line.ends_with(' ') || line.ends_with('\t') {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Minor,
                code: "C-G7".to_string(),
                message: "Trailing spaces".to_string(),
            });
        }
    }
    diagnostics
}

fn check_leading_trailing_lines(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let lines: Vec<&str> = content.split('\n').collect();

    if lines.first().map_or(false, |l| l.is_empty()) {
        diagnostics.push(Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Minor,
            code: "C-G8".to_string(),
            message: "No leading empty lines".to_string(),
        });
    }

    let last_idx = lines.len().saturating_sub(2);
    if lines.len() >= 2 && lines[last_idx].is_empty() {
        diagnostics.push(Diagnostic {
            file: filename.to_string(),
            line: lines.len(),
            severity: Severity::Minor,
            code: "C-G8".to_string(),
            message: "No more than 1 trailing empty line".to_string(),
        });
    }

    diagnostics
}

static PP_OPEN_RE: OnceLock<Regex> = OnceLock::new();
static PP_BRANCH_RE: OnceLock<Regex> = OnceLock::new();
static PP_CLOSE_RE: OnceLock<Regex> = OnceLock::new();
static PP_ANY_RE: OnceLock<Regex> = OnceLock::new();

fn check_preprocessor_indentation(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let open_re =
        PP_OPEN_RE.get_or_init(|| Regex::new(r"^[ \t]*#[ \t]*(if|ifdef|ifndef)\b").unwrap());
    let branch_re =
        PP_BRANCH_RE.get_or_init(|| Regex::new(r"^[ \t]*#[ \t]*(elif|else)\b").unwrap());
    let close_re = PP_CLOSE_RE.get_or_init(|| Regex::new(r"^[ \t]*#[ \t]*endif\b").unwrap());
    let any_re = PP_ANY_RE.get_or_init(|| Regex::new(r"^[ \t]*#[ \t]*\w+").unwrap());

    let mut indentation_stack: Vec<i64> = vec![-1];

    for (i, line) in content.split('\n').enumerate() {
        let line_number = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let indent = (line.len() - line.trim_start().len()) as i64;

        let is_open = open_re.is_match(line);
        let is_branch = branch_re.is_match(line);
        let is_close = close_re.is_match(line);
        let is_any = any_re.is_match(line);

        if is_any && indent < *indentation_stack.last().unwrap() {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: line_number,
                severity: Severity::Minor,
                code: "C-G3".to_string(),
                message: "Preprocessor directive is incorrectly indented".to_string(),
            });
        }

        if is_open {
            indentation_stack.push(indent);
        } else if is_branch {
            if indent > *indentation_stack.last().unwrap() {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: line_number,
                    severity: Severity::Minor,
                    code: "C-G3".to_string(),
                    message: "Preprocessor directive is incorrectly indented".to_string(),
                });
            }
        } else if is_close {
            if indent > *indentation_stack.last().unwrap() {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: line_number,
                    severity: Severity::Minor,
                    code: "C-G3".to_string(),
                    message: "Preprocessor directive is incorrectly indented".to_string(),
                });
            }
            if indentation_stack.len() >= 2 {
                indentation_stack.pop();
            }
        } else if is_any && indent == *indentation_stack.last().unwrap() {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: line_number,
                severity: Severity::Minor,
                code: "C-G3".to_string(),
                message: "Preprocessor directive is incorrectly indented".to_string(),
            });
        }
    }

    diagnostics
}

static INCLUDE_RE: OnceLock<Regex> = OnceLock::new();

fn check_non_header_inclusion(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let re = INCLUDE_RE
        .get_or_init(|| Regex::new(r#"^[ \t]*#[ \t]*include[ \t]*"([^"]+)""#).unwrap());

    for (i, line) in content.split('\n').enumerate() {
        if let Some(caps) = re.captures(line) {
            let included = &caps[1];
            if !included.ends_with(".h") {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: i + 1,
                    severity: Severity::Major,
                    code: "C-G5".to_string(),
                    message: format!("Included file '{}' should be a header", included),
                });
            }
        }
    }
    diagnostics
}

static ASM_RE: OnceLock<Regex> = OnceLock::new();

fn check_inline_assembly(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let re = ASM_RE.get_or_init(|| Regex::new(r"\b(asm|__asm__)\b").unwrap());

    for (i, line) in content.split('\n').enumerate() {
        if re.is_match(line) {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Fatal,
                code: "C-G10".to_string(),
                message: "Inline assembly usage is forbidden".to_string(),
            });
        }
    }
    diagnostics
}

fn check_global_variable_constness(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    for node in root.children(&mut cursor) {
        if node.kind() != "declaration" {
            continue;
        }
        let has_const = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "const" || c.utf8_text(content.as_bytes()) == Ok("const"));
        let mut dc = node.walk();
        for declarator in node.children(&mut dc) {
            if declarator.kind() == "init_declarator" {
                let assign = declarator
                    .children(&mut declarator.walk())
                    .find(|c| c.kind() == "=");
                if let Some(assign) = assign {
                    if !has_const {
                        diagnostics.push(Diagnostic {
                            file: filename.to_string(),
                            line: assign.start_position().row + 1,
                            severity: Severity::Major,
                            code: "C-G4".to_string(),
                            message: "Global variable should be const if never reassigned"
                                .to_string(),
                        });
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

    const VALID_HEADER: &str = "/*\n** EPITECH PROJECT, 2024\n** my_project\n** File description:\n** some description\n*/\n";

    #[test]
    fn c_g1_triggers_on_missing_header() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check_header("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G1"));
    }

    #[test]
    fn c_g1_does_not_trigger_on_valid_header() {
        let content = format!("{}int main(void)\n{{\n    return 0;\n}}\n", VALID_HEADER);
        let diags = check_header("test.c", &content);
        assert!(!diags.iter().any(|d| d.code == "C-G1"));
    }

    #[test]
    fn c_g2_triggers_when_extra_blank_lines_between_functions() {
        let content = "int f(void)\n{\n    return 0;\n}\n\n\nint g(void)\n{\n    return 0;\n}\n";
        let diags = check_function_separation("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G2"));
    }

    #[test]
    fn c_g2_does_not_trigger_with_one_blank_line() {
        let content = "int f(void)\n{\n    return 0;\n}\n\nint g(void)\n{\n    return 0;\n}\n";
        let diags = check_function_separation("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G2"));
    }

    #[test]
    fn c_g6_triggers_on_cr_line_ending() {
        let content = "int main(void)\r\n{\n    return 0;\n}\n";
        let diags = check_line_endings("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G6"));
    }

    #[test]
    fn c_g6_does_not_trigger_on_plain_lf() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check_line_endings("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G6"));
    }

    #[test]
    fn c_g7_triggers_on_trailing_space() {
        let content = "int main(void) \n{\n    return 0;\n}\n";
        let diags = check_trailing_spaces("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G7"));
    }

    #[test]
    fn c_g7_does_not_trigger_without_trailing_space() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check_trailing_spaces("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G7"));
    }

    #[test]
    fn c_g8_triggers_on_leading_empty_line() {
        let content = "\nint main(void)\n{\n    return 0;\n}\n";
        let diags = check_leading_trailing_lines("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G8"));
    }

    #[test]
    fn c_g8_triggers_on_multiple_trailing_empty_lines() {
        let content = "int main(void)\n{\n    return 0;\n}\n\n\n";
        let diags = check_leading_trailing_lines("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G8"));
    }

    #[test]
    fn c_g8_does_not_trigger_on_clean_file() {
        let content = "int main(void)\n{\n    return 0;\n}\n";
        let diags = check_leading_trailing_lines("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G8"));
    }

    #[test]
    fn c_g3_triggers_on_misindented_directive() {
        let content = "#if 1\n#define FOO 1\n#endif\n";
        let diags = check_preprocessor_indentation("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G3"));
    }

    #[test]
    fn c_g3_does_not_trigger_on_properly_indented_directives() {
        let content = "#if 1\n    #define FOO 1\n#endif\n";
        let diags = check_preprocessor_indentation("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G3"));
    }

    #[test]
    fn c_g5_triggers_on_non_header_include() {
        let content = "#include \"foo.c\"\n";
        let diags = check_non_header_inclusion("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G5"));
    }

    #[test]
    fn c_g5_does_not_trigger_on_header_include() {
        let content = "#include \"foo.h\"\n";
        let diags = check_non_header_inclusion("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G5"));
    }

    #[test]
    fn c_g10_triggers_on_inline_assembly() {
        let content = "void f(void)\n{\n    asm(\"nop\");\n}\n";
        let diags = check_inline_assembly("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G10"));
    }

    #[test]
    fn c_g10_does_not_trigger_without_assembly() {
        let content = "void f(void)\n{\n    return;\n}\n";
        let diags = check_inline_assembly("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G10"));
    }

    #[test]
    fn c_g4_triggers_on_non_const_global_with_initializer() {
        let content = "int x = 5;\n";
        let diags = check_global_variable_constness("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-G4"));
    }

    #[test]
    fn c_g4_does_not_trigger_on_const_global() {
        let content = "const int x = 5;\n";
        let diags = check_global_variable_constness("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-G4"));
    }
}
