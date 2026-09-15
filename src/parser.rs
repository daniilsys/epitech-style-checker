use tree_sitter::{Parser, Tree};

pub fn parse(content: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .expect("Failed to load C grammar");
    parser.parse(content, None)
}
