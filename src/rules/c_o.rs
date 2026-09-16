use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_snake_case(filename, content));
    diagnostics.extend(check_delivery_files(filename, content));
    diagnostics.extend(check_functions_count(filename, content));

    diagnostics
}

fn check_snake_case(filename: &str, _content: &str) -> Vec<Diagnostic> {
    let name = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if !name
        .chars()
        .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return vec![Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Minor,
            code: "C-O4".to_string(),
            message: "Filename must be in snake_case".to_string(),
        }];
    }
    vec![]
}

static UNWANTED_FILE_RES: OnceLock<Vec<Regex>> = OnceLock::new();
const UNWANTED_MAGIC: [&[u8]; 4] = [b"\x7fELF", b"MZ", b"\xfe\xed\xfa\xce", b"\x4d\x5a"];

fn check_delivery_files(filename: &str, content: &str) -> Vec<Diagnostic> {
    let regexes = UNWANTED_FILE_RES.get_or_init(|| {
        [
            r"^.*\.d$",
            r"^.*\.o$",
            r"^.*\.ko$",
            r"^.*\.obj$",
            r"^.*\.elf$",
            r"^.*\.ilk$",
            r"^.*\.map$",
            r"^.*\.exp$",
            r"^.*\.gch$",
            r"^.*\.pch$",
            r"^.*\.lib$",
            r"^.*\.a$",
            r"^.*\.la$",
            r"^.*\.lo$",
            r"^.*\.dll$",
            r"^.*\.so$",
            r"^.*\.so\..*$",
            r"^.*\.dylib$",
            r"^.*\.exe$",
            r"^.*\.out$",
            r"^.*\.app$",
            r"^.*\.i.*86$",
            r"^.*\.x86_64$",
            r"^.*\.hex$",
            r"^.*\.su$",
            r"^.*\.idb$",
            r"^.*\.pdb$",
            r"^.*\.mod.*$",
            r"^.*\.cmd$",
            r"^modules\.order$",
            r"^Module\.symvers$",
            r"^Mkfile\.old$",
            r"^dkms\.conf$",
            r"^.*\.gcno$",
            r"^.*\.gcda$",
            r"^.*\.gcov$",
            r"^.*~.*$",
            r"^.*#.*$",
            r"^vgcore\.\d+$",
        ]
        .iter()
        .map(|p| Regex::new(p).unwrap())
        .collect()
    });

    let bytes = content.as_bytes();
    if UNWANTED_MAGIC.iter().any(|m| bytes.starts_with(m)) {
        return vec![Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Major,
            code: "C-O1".to_string(),
            message: "Binary files must not be delivered".to_string(),
        }];
    }

    let name = Path::new(filename)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if regexes.iter().any(|re| re.is_match(name)) {
        return vec![Diagnostic {
            file: filename.to_string(),
            line: 1,
            severity: Severity::Major,
            code: "C-O1".to_string(),
            message: "Unwanted file must not be delivered".to_string(),
        }];
    }
    vec![]
}

const MAX_FUNCTION_COUNT: usize = 10;
const MAX_NON_STATIC_FUNCTION_COUNT: usize = 5;

fn check_functions_count(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !filename.ends_with(".c") && !filename.ends_with(".h") {
        return diagnostics;
    }
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };
    let root = tree.root_node();
    let bytes = content.as_bytes();
    let mut cursor = root.walk();

    let mut function_count = 0;
    let mut non_static_function_count = 0;

    for node in root.children(&mut cursor) {
        if node.kind() != "function_definition" {
            continue;
        }
        let declarator_start = node
            .child_by_field_name("body")
            .map(|b| b.start_byte())
            .unwrap_or(node.start_byte());
        let prefix =
            std::str::from_utf8(&bytes[node.start_byte()..declarator_start]).unwrap_or("");
        let is_static = prefix.contains("static");

        function_count += 1;
        let mut reported = false;
        if !is_static {
            non_static_function_count += 1;
            if non_static_function_count > MAX_NON_STATIC_FUNCTION_COUNT {
                diagnostics.push(Diagnostic {
                    file: filename.to_string(),
                    line: node.start_position().row + 1,
                    severity: Severity::Major,
                    code: "C-O3".to_string(),
                    message: "Too many non-static functions in the file".to_string(),
                });
                reported = true;
            }
        }
        if !reported && function_count > MAX_FUNCTION_COUNT {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: node.start_position().row + 1,
                severity: Severity::Major,
                code: "C-O3".to_string(),
                message: "Too many functions in the file".to_string(),
            });
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_o4_triggers_on_uppercase_filename() {
        let diags = check_snake_case("MyFile.c", "");
        assert!(diags.iter().any(|d| d.code == "C-O4"));
    }

    #[test]
    fn c_o4_does_not_trigger_on_snake_case_filename() {
        let diags = check_snake_case("my_file.c", "");
        assert!(!diags.iter().any(|d| d.code == "C-O4"));
    }

    #[test]
    fn c_o1_triggers_on_binary_magic() {
        let content = "\u{7f}ELFrest of a fake binary";
        let diags = check_delivery_files("a.out", content);
        assert!(diags.iter().any(|d| d.code == "C-O1"));
    }

    #[test]
    fn c_o1_triggers_on_unwanted_extension() {
        let diags = check_delivery_files("main.o", "");
        assert!(diags.iter().any(|d| d.code == "C-O1"));
    }

    #[test]
    fn c_o1_does_not_trigger_on_normal_source_file() {
        let diags = check_delivery_files("main.c", "int main(void) { return 0; }\n");
        assert!(!diags.iter().any(|d| d.code == "C-O1"));
    }

    #[test]
    fn c_o3_triggers_on_too_many_functions() {
        let mut content = String::new();
        for i in 0..11 {
            content.push_str(&format!("static int f{}(void)\n{{\n    return 0;\n}}\n\n", i));
        }
        let diags = check_functions_count("test.c", &content);
        assert!(diags.iter().any(|d| d.code == "C-O3"));
    }

    #[test]
    fn c_o3_triggers_on_too_many_non_static_functions() {
        let mut content = String::new();
        for i in 0..6 {
            content.push_str(&format!("int f{}(void)\n{{\n    return 0;\n}}\n\n", i));
        }
        let diags = check_functions_count("test.c", &content);
        assert!(diags.iter().any(|d| d.code == "C-O3"));
    }

    #[test]
    fn c_o3_does_not_trigger_with_few_functions() {
        let content = "int f(void)\n{\n    return 0;\n}\n\nint g(void)\n{\n    return 0;\n}\n";
        let diags = check_functions_count("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-O3"));
    }
}
