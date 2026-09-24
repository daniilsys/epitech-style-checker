use crate::diagnostic::{Diagnostic, Severity};
use crate::parser::parse;
use regex::Regex;
use std::sync::OnceLock;
use tree_sitter::Node;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_function_return_type(filename, content));
    diagnostics.extend(check_macro_names(filename, content));
    diagnostics.extend(check_typedef_names(filename, content));
    diagnostics.extend(check_enum_members(filename, content));
    diagnostics.extend(check_pointer_attachments(filename, content));

    diagnostics
}

const ALLOWED_TYPES: &[&str] = &[
    "sfBlack", "sfBlendAdd", "sfBlendAlpha", "sfBlendMultiply", "sfBlendNone", "sfBlue",
    "sfCircleShape", "sfClock", "sfColor", "sfContext", "sfConvexShape", "sfCursor", "sfCyan",
    "sfFloatRect", "sfFont", "sfGreen", "sfImage", "sfIntRect", "sfJoystick", "sfKeyboard",
    "sfKeyCode", "sfListener", "sfMagenta", "sfMicroseconds", "sfMilliseconds", "sfMouse",
    "sfMouseButton", "sfMouseButtonEvent", "sfMusic", "sfMutex", "sfRectangleShape", "sfRed",
    "sfRenderStates", "sfRenderTexture", "sfRenderWindow", "sfSeconds", "sfSensor", "sfShader",
    "sfShape", "sfSleep", "sfSound", "sfSoundBuffer", "sfSoundBufferRecorder",
    "sfSoundRecorder", "sfSoundStream", "sfSprite", "sfText", "sfTexture", "sfThread", "sfTime",
    "sfTouch", "sfTransform", "sfTransformable", "sfTransparent", "sfVertex", "sfVertexArray",
    "sfVertexBuffer", "sfVideoMode", "sfView", "sfWhite", "sfWindow", "sfYellow", "sfBool",
    "sfFtp", "sfFtpDirectoryResponse", "sfFtpListingResponse", "sfFtpResponse", "sfGlslIvec2",
    "sfGlslVec2", "sfGlslVec3", "sfHttp", "sfHttpRequest", "sfHttpResponse", "sfInputStream",
    "sfInputStreamGetSizeFunc", "sfInputStreamReadFunc", "sfInputStreamSeekFunc",
    "sfInputStreamTellFunc", "sfInt16", "sfInt32", "sfInt64", "sfInt8", "sfPacket",
    "sfShapeGetPointCallback", "sfSocketSelector", "sfSoundRecorderProcessCallback",
    "sfSoundRecorderStartCallback", "sfSoundRecorderStopCallback", "sfSoundStreamChunk",
    "sfSoundStreamGetDataCallback", "sfSoundStreamSeekCallback", "sfTcpListener", "sfTcpSocket",
    "sfUdpSocket", "sfUint16", "sfUint32", "sfUint64", "sfUint8", "sfVector2f", "sfVector2u",
    "sfVector2i", "sfVector3f", "sfVector3u", "sfVector3i", "sfWindowHandle", "userData",
    "FILE", "DIR", "Elf_Byte", "Elf32_Sym", "Elf32_Off", "Elf32_Addr", "Elf32_Section",
    "Elf32_Versym", "Elf32_Half", "Elf32_Sword", "Elf32_Word", "Elf32_Sxword", "Elf32_Xword",
    "Elf32_Ehdr", "Elf32_Phdr", "Elf32_Shdr", "Elf32_Rel", "Elf32_Rela", "Elf32_Dyn",
    "Elf32_Nhdr", "Elf64_Sym", "Elf64_Off", "Elf64_Addr", "Elf64_Section", "Elf64_Versym",
    "Elf64_Half", "Elf64_Sword", "Elf64_Word", "Elf64_Sxword", "Elf64_Xword", "Elf64_Ehdr",
    "Elf64_Phdr", "Elf64_Shdr", "Elf64_Rel", "Elf64_Rela", "Elf64_Dyn", "Elf64_Nhdr", "_Bool",
    "WINDOW",
];

const TYPE_MODIFIERS: &[&str] = &[
    "inline", "static", "unsigned", "signed", "short", "long", "volatile", "struct",
];

static TYPE_NAME_RE: OnceLock<Regex> = OnceLock::new();
static MACRO_NAME_RE: OnceLock<Regex> = OnceLock::new();

fn check_function_return_type(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let type_re = TYPE_NAME_RE.get_or_init(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    for node in root.children(&mut cursor) {
        if node.kind() != "function_definition" {
            continue;
        }
        let Some(type_node) = node.child_by_field_name("type") else {
            continue;
        };
        let type_text = type_node.utf8_text(content.as_bytes()).unwrap_or("");
        let core: Vec<&str> = type_text
            .split_whitespace()
            .filter(|w| !TYPE_MODIFIERS.contains(w))
            .collect();
        let Some(core_type) = core.last() else {
            continue;
        };
        if !type_re.is_match(core_type) && !ALLOWED_TYPES.contains(core_type) {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: node.start_position().row + 1,
                severity: Severity::Minor,
                code: "C-V1".to_string(),
                message: format!("Return type '{}' is not in snake_case", core_type),
            });
        }
    }
    diagnostics
}

fn check_macro_names(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let name_re = MACRO_NAME_RE.get_or_init(|| Regex::new(r"^[A-Z_$][A-Z0-9_$]+").unwrap());

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("#define") else {
            continue;
        };
        let rest = rest.trim_start();
        let end = rest
            .find(|c: char| c == ' ' || c == '\t' || c == '(')
            .unwrap_or(rest.len());
        let macro_name = &rest[..end];
        if macro_name.is_empty() {
            continue;
        }
        if !name_re.is_match(macro_name) {
            diagnostics.push(Diagnostic {
                file: filename.to_string(),
                line: i + 1,
                severity: Severity::Minor,
                code: "C-V1".to_string(),
                message: format!("Macro name '{}' should be in UPPER_CASE", macro_name),
            });
        }
    }
    diagnostics
}

fn first_identifier<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "type_identifier" || node.kind() == "identifier" {
        return Some(*node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = first_identifier(&child) {
            return Some(found);
        }
    }
    None
}

fn check_typedef_names(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    let mut cursor = root.walk();
    for node in root.children(&mut cursor) {
        if node.kind() != "type_definition" {
            continue;
        }
        let type_node = node.child_by_field_name("type");
        let mut dc = node.walk();
        for child in node.children(&mut dc) {
            if child.kind() == "typedef" || child.kind() == ";" || child.kind() == "," {
                continue;
            }
            if Some(child) == type_node {
                continue;
            }
            if let Some(ident) = first_identifier(&child) {
                let name = ident.utf8_text(content.as_bytes()).unwrap_or("");
                if !name.ends_with("_t") {
                    diagnostics.push(Diagnostic {
                        file: filename.to_string(),
                        line: node.start_position().row + 1,
                        severity: Severity::Minor,
                        code: "C-V1".to_string(),
                        message: format!("Typedef name '{}' must end with '_t'", name),
                    });
                }
            }
        }
    }
    diagnostics
}

fn check_enum_members(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };

    let root = tree.root_node();
    walk_enums(&root, content, filename, &mut diagnostics);
    diagnostics
}

fn walk_enums(node: &Node, content: &str, filename: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == "enum_specifier" {
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for enumerator in body.children(&mut cursor) {
                if enumerator.kind() != "enumerator" {
                    continue;
                }
                if let Some(name_node) = enumerator.child_by_field_name("name") {
                    let name = name_node.utf8_text(content.as_bytes()).unwrap_or("");
                    let has_alpha = name.chars().any(|c| c.is_alphabetic());
                    let all_upper = name
                        .chars()
                        .filter(|c| c.is_alphabetic())
                        .all(|c| c.is_uppercase());
                    if has_alpha && !all_upper {
                        diagnostics.push(Diagnostic {
                            file: filename.to_string(),
                            line: name_node.start_position().row + 1,
                            severity: Severity::Minor,
                            code: "C-V1".to_string(),
                            message: format!("Enum member '{}' must be uppercase", name),
                        });
                    }
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_enums(&child, content, filename, diagnostics);
    }
}

fn check_pointer_attachments(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tree = match parse(content) {
        Some(t) => t,
        None => return diagnostics,
    };
    let root = tree.root_node();
    let bytes = content.as_bytes();
    let mut stars = Vec::new();
    collect_stars(&root, &mut stars);

    for star in stars {
        let start = star.start_byte();
        let line = star.start_position().row + 1;
        let before = if start > 0 { bytes.get(start - 1).copied() } else { None };
        let after = bytes.get(start + 1).copied();

        if before == Some(b'*') {
            if after == Some(b' ') {
                diagnostics.push(make_v3(filename, line));
            }
            continue;
        }

        let before_is_unary_operator = matches!(before, Some(b'-') | Some(b'!') | Some(b'~') | Some(b'&') | Some(b'+'));

        if before != Some(b' ')
            && before != Some(b'(')
            && before != Some(b'[')
            && !before_is_unary_operator
        {
            diagnostics.push(make_v3(filename, line));
        } else if after == Some(b' ') {
            diagnostics.push(make_v3(filename, line));
        }
    }
    diagnostics
}

fn make_v3(filename: &str, line: usize) -> Diagnostic {
    Diagnostic {
        file: filename.to_string(),
        line,
        severity: Severity::Minor,
        code: "C-V3".to_string(),
        message: "Pointer '*' must be attached to the variable name".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_v1_triggers_on_non_snake_case_return_type_from_typedef() {
        let content = "typedef struct MyType { int x; } MyType_t;\n\nMyType_t f(void)\n{\n    MyType_t t;\n    return t;\n}\n";
        let diags = check_function_return_type("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_does_not_trigger_on_snake_case_return_type() {
        let content = "int f(void)\n{\n    return 0;\n}\n";
        let diags = check_function_return_type("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_triggers_on_lowercase_macro_name() {
        let content = "#define max_val 42\n";
        let diags = check_macro_names("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_does_not_trigger_on_upper_case_macro_name() {
        let content = "#define MAX_VAL 42\n";
        let diags = check_macro_names("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_triggers_on_typedef_name_without_t_suffix() {
        let content = "typedef struct point { int x; int y; } point;\n";
        let diags = check_typedef_names("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_does_not_trigger_on_typedef_name_ending_in_t() {
        let content = "typedef struct point { int x; int y; } point_t;\n";
        let diags = check_typedef_names("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_triggers_on_lowercase_enum_member() {
        let content = "enum color { red, green, blue };\n";
        let diags = check_enum_members("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v1_does_not_trigger_on_uppercase_enum_member() {
        let content = "enum color { RED, GREEN, BLUE };\n";
        let diags = check_enum_members("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-V1"));
    }

    #[test]
    fn c_v3_triggers_on_star_attached_to_type() {
        let content = "int f(void)\n{\n    int* p;\n    return 0;\n}\n";
        let diags = check_pointer_attachments("test.c", content);
        assert!(diags.iter().any(|d| d.code == "C-V3"));
    }

    #[test]
    fn c_v3_does_not_trigger_on_star_attached_to_name() {
        let content = "int f(void)\n{\n    int *p;\n    return 0;\n}\n";
        let diags = check_pointer_attachments("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-V3"));
    }

    #[test]
    fn c_v3_does_not_trigger_on_dereference_after_unary_operator() {
        let content = "void f(int *sign)\n{\n    *sign = -*sign;\n}\n";
        let diags = check_pointer_attachments("test.c", content);
        assert!(!diags.iter().any(|d| d.code == "C-V3"));
    }
}

fn collect_stars<'a>(node: &Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "pointer_declarator" || node.kind() == "pointer_expression" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "*" {
                out.push(child);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_stars(&child, out);
    }
}
