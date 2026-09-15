mod c_a;
mod c_c;
mod c_f;
mod c_g;
mod c_o;

use crate::diagnostic::Diagnostic;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(c_g::check(filename, content));
    diagnostics.extend(c_a::check(filename, content));
    diagnostics.extend(c_c::check(filename, content));
    diagnostics.extend(c_f::check(filename, content));
    diagnostics.extend(c_o::check(filename, content));

    diagnostics
}
