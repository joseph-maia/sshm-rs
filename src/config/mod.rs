// SSH config parser module
// Parses ~/.ssh/config (with Include support) into SSHHost structs
// Handles: add, edit, delete, backup, multi-alias hosts

mod parser;
mod paths;

pub use parser::*;
pub use paths::*;
