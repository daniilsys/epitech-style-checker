use crate::diagnostic::{Diagnostic, Severity};
use regex::Regex;
use std::sync::OnceLock;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_header(filename, content));
    diagnostics.extend(check_function_separation(filename, content));
    diagnostics.extend(check_trailing_spaces(filename, content));
    diagnostics.extend(check_line_endings(filename, content));
    diagnostics.extend(check_leading_trailing_lines(filename, content));

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
