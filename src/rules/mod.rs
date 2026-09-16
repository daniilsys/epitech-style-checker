mod c_a;
mod c_c;
mod c_f;
mod c_g;
mod c_h;
mod c_l;
mod c_o;
mod c_v;
mod c_z;

use crate::diagnostic::Diagnostic;

pub fn check(filename: &str, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(c_g::check(filename, content));
    diagnostics.extend(c_a::check(filename, content));
    diagnostics.extend(c_c::check(filename, content));
    diagnostics.extend(c_f::check(filename, content));
    diagnostics.extend(c_o::check(filename, content));
    diagnostics.extend(c_h::check(filename, content));
    diagnostics.extend(c_l::check(filename, content));
    diagnostics.extend(c_v::check(filename, content));
    diagnostics.extend(c_z::check(filename, content));

    diagnostics
}
