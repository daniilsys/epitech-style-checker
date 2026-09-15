#[derive(Debug)]
pub enum Severity {
    Fatal,
    Major,
    Minor,
    Info,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub severity: Severity,
    pub code: String,
    pub message: String,
}
