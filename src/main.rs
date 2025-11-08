mod builtins;
mod io_helpers;
mod parser;
mod shell;
mod utils;

use crate::shell::Shell;

fn main() {
    let mut shell = Shell::new();
    if let Err(err) = shell.run() {
        eprintln!("shell error: {}", err);
    }
}
