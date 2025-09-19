pub mod attributes;
pub mod builder;
pub mod errors;
pub mod reader;
pub mod session;
#[cfg(test)]
mod test;
pub const CRLF: &str = "\r\n";
pub const LF: &str = "\n";
